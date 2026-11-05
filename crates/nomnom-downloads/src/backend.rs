//! The seam between the queue and the outside world. `YtDlpBackend` talks to the real tool;
//! tests plug in a fake so queue behaviour can be checked without a network.

use crate::analyze::{parse_media_info, MediaInfo};
use crate::error::{classify_stderr, DlError};
use crate::format::{build_download_args, common_args, ArgsContext, Choice};
use crate::progress::{parse_line, OutputLine, Progress};
use async_trait::async_trait;
use nomnom_core::process;
use nomnom_tools::{Tool, Tools};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

pub struct DownloadJob {
    pub url: String,
    pub choice: Choice,
    pub staging: PathBuf,
    pub firefox_cookies: bool,
}

pub type ProgressFn<'a> = &'a (dyn Fn(Progress) + Send + Sync);

#[async_trait]
pub trait MediaBackend: Send + Sync {
    async fn analyze(&self, url: &str, firefox_cookies: bool) -> Result<MediaInfo, DlError>;
    /// Downloads into `job.staging` and returns the finished file inside it.
    async fn download(
        &self,
        job: &DownloadJob,
        on_progress: ProgressFn<'_>,
    ) -> Result<PathBuf, DlError>;
}

pub struct YtDlpBackend {
    tools: Tools,
}

impl YtDlpBackend {
    pub fn new(tools: Tools) -> Self {
        YtDlpBackend { tools }
    }

    fn ytdlp(&self) -> Result<PathBuf, DlError> {
        self.tools
            .locate(Tool::YtDlp)
            .ok_or(DlError::ToolMissing("yt-dlp"))
    }
}

const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;
const ANALYZE_TIMEOUT: Duration = Duration::from_secs(90);

#[async_trait]
impl MediaBackend for YtDlpBackend {
    async fn analyze(&self, url: &str, firefox_cookies: bool) -> Result<MediaInfo, DlError> {
        let exe = self.ytdlp()?;
        let mut args = common_args(firefox_cookies);
        args.extend(["-J", "--flat-playlist", "--", url].map(OsString::from));
        let mut child = process::spawn(&exe, args, true).map_err(|e| DlError::Io(e.to_string()))?;
        let mut stdout = child
            .child
            .stdout
            .take()
            .ok_or_else(|| DlError::Io("no output".into()))?;
        let mut stderr = child
            .child
            .stderr
            .take()
            .ok_or_else(|| DlError::Io("no output".into()))?;

        let run = async {
            let (mut out, mut err) = (Vec::new(), String::new());
            let mut limited_out = (&mut stdout).take(MAX_JSON_BYTES);
            let mut limited_err = (&mut stderr).take(1 << 20);
            let (_, _, status) = tokio::join!(
                limited_out.read_to_end(&mut out),
                limited_err.read_to_string(&mut err),
                child.child.wait()
            );
            (out, err, status)
        };
        let (out, err, status) = tokio::time::timeout(ANALYZE_TIMEOUT, run)
            .await
            .map_err(|_| DlError::Failed("Looking up this link took too long.".into()))?;

        if !status.map(|s| s.success()).unwrap_or(false) {
            return Err(classify_stderr(&err));
        }
        let json: serde_json::Value = serde_json::from_slice(&out)
            .map_err(|_| DlError::Failed("The site returned something unexpected.".into()))?;
        parse_media_info(&json, url)
    }

    async fn download(
        &self,
        job: &DownloadJob,
        on_progress: ProgressFn<'_>,
    ) -> Result<PathBuf, DlError> {
        let exe = self.ytdlp()?;
        let ffmpeg = self.tools.locate(Tool::Ffmpeg);
        let ctx = ArgsContext {
            ffmpeg: ffmpeg.as_deref(),
            staging: &job.staging,
            firefox_cookies: job.firefox_cookies,
        };
        let args = build_download_args(&job.choice, &ctx, &job.url);

        let mut child = process::spawn(&exe, args, true).map_err(|e| DlError::Io(e.to_string()))?;
        let stdout = child
            .child
            .stdout
            .take()
            .ok_or_else(|| DlError::Io("no output".into()))?;
        let mut stderr = child
            .child
            .stderr
            .take()
            .ok_or_else(|| DlError::Io("no output".into()))?;

        // Keep stderr drained so yt-dlp never blocks on a full pipe; we only need its tail.
        let err_task = tokio::spawn(async move {
            let mut text = String::new();
            let _ = (&mut stderr)
                .take(256 * 1024)
                .read_to_string(&mut text)
                .await;
            text
        });

        let mut final_path = None;
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            match parse_line(&line) {
                Some(OutputLine::Progress(p)) => on_progress(p),
                Some(OutputLine::FinalPath(p)) => final_path = Some(p),
                None => {}
            }
        }
        let status = child
            .child
            .wait()
            .await
            .map_err(|e| DlError::Io(e.to_string()))?;
        let stderr_text = err_task.await.unwrap_or_default();
        if !status.success() {
            return Err(classify_stderr(&stderr_text));
        }
        resolve_output(final_path, &job.staging)
    }
}

/// The path yt-dlp printed if it exists inside staging, otherwise the largest finished file there.
fn resolve_output(printed: Option<PathBuf>, staging: &Path) -> Result<PathBuf, DlError> {
    if let Some(p) = printed {
        let p = if p.is_absolute() { p } else { staging.join(p) };
        if p.is_file() && p.starts_with(staging) {
            return Ok(p);
        }
    }
    let mut best: Option<(u64, PathBuf)> = None;
    let entries = std::fs::read_dir(staging).map_err(|e| DlError::Io(e.to_string()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let unfinished = [".part", ".ytdl", ".temp"]
            .iter()
            .any(|s| name.ends_with(s))
            || name.contains(".part-");
        if unfinished || !path.is_file() {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if best.as_ref().is_none_or(|(s, _)| size > *s) {
            best = Some((size, path));
        }
    }
    best.map(|(_, p)| p)
        .ok_or_else(|| DlError::Failed("The download finished but no file was produced.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_printed_path_inside_staging() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.mp4");
        std::fs::write(&f, "x").unwrap();
        std::fs::write(dir.path().join("bigger.bin"), "xxxxxxxx").unwrap();
        assert_eq!(resolve_output(Some(f.clone()), dir.path()).unwrap(), f);
    }

    #[test]
    fn ignores_printed_path_outside_staging() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let evil = outside.path().join("evil.exe");
        std::fs::write(&evil, "x").unwrap();
        std::fs::write(dir.path().join("real.mp4"), "xx").unwrap();
        let got = resolve_output(Some(evil), dir.path()).unwrap();
        assert_eq!(got.file_name().unwrap(), "real.mp4");
    }

    #[test]
    fn scan_skips_unfinished_parts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("v.mp4.part"), "xxxxxxxxxx").unwrap();
        std::fs::write(dir.path().join("v.mp4"), "xx").unwrap();
        assert_eq!(
            resolve_output(None, dir.path())
                .unwrap()
                .file_name()
                .unwrap(),
            "v.mp4"
        );
    }

    #[test]
    fn empty_staging_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(resolve_output(None, dir.path()).is_err());
    }
}
