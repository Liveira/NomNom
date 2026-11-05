//! Locates, installs and updates the external tools NOMNOM drives: yt-dlp and ffmpeg/ffprobe.
//!
//! Nothing here runs unless the user asks. Installs download from the tools' official GitHub
//! releases and verify the SHA-256 listed in the same release before anything is put in place.

use nomnom_core::process;
use std::path::{Path, PathBuf};

mod install;

pub use install::{parse_checksum, verify_sha256, InstallProgress};

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("network error: {0}")]
    Network(String),
    #[error("The download didn't match its published checksum, so it was discarded.")]
    ChecksumMismatch,
    #[error("couldn't find {0} in the downloaded archive")]
    MissingInArchive(&'static str),
    #[error("{0}")]
    Io(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    YtDlp,
    Ffmpeg,
    Ffprobe,
}

impl Tool {
    pub fn exe_name(self) -> &'static str {
        match self {
            Tool::YtDlp => "yt-dlp.exe",
            Tool::Ffmpeg => "ffmpeg.exe",
            Tool::Ffprobe => "ffprobe.exe",
        }
    }
    fn version_args(self) -> &'static [&'static str] {
        match self {
            Tool::YtDlp => &["--version"],
            _ => &["-version"],
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolStatus {
    pub name: &'static str,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    /// True when NOMNOM installed it (as opposed to finding it on PATH).
    pub managed: bool,
}

#[derive(Debug, Clone)]
pub struct Tools {
    dir: PathBuf,
}

impl Tools {
    pub fn new(dir: PathBuf) -> Self {
        Tools { dir }
    }

    pub fn managed_path(&self, tool: Tool) -> PathBuf {
        self.dir.join(tool.exe_name())
    }

    /// Managed copy first, then anything on PATH.
    pub fn locate(&self, tool: Tool) -> Option<PathBuf> {
        let managed = self.managed_path(tool);
        if managed.is_file() {
            return Some(managed);
        }
        let path_var = std::env::var_os("PATH")?;
        std::env::split_paths(&path_var)
            .map(|d| d.join(tool.exe_name()))
            .find(|p| p.is_file())
    }

    pub async fn status(&self, tool: Tool) -> ToolStatus {
        let path = self.locate(tool);
        let version = match &path {
            Some(p) => version_of(p, tool).await,
            None => None,
        };
        let managed = path.as_deref().is_some_and(|p| p.starts_with(&self.dir));
        ToolStatus {
            name: tool.exe_name(),
            path,
            version,
            managed,
        }
    }

    /// Downloads and installs yt-dlp. `progress` receives (bytes_done, bytes_total).
    pub async fn install_ytdlp(&self, progress: &InstallProgress) -> Result<(), ToolError> {
        install::install_ytdlp(&self.dir, progress).await
    }

    /// Downloads ffmpeg and ffprobe. The archive is large (about 150 MB).
    pub async fn install_ffmpeg(&self, progress: &InstallProgress) -> Result<(), ToolError> {
        install::install_ffmpeg(&self.dir, progress).await
    }

    /// Updates the managed yt-dlp in place. Never touches a copy found on PATH.
    pub async fn update_ytdlp(&self) -> Result<String, ToolError> {
        let path = self.managed_path(Tool::YtDlp);
        if !path.is_file() {
            return Err(ToolError::Io(
                "NOMNOM's own copy of yt-dlp isn't installed.".into(),
            ));
        }
        let mut child =
            process::spawn(&path, ["-U"], true).map_err(|e| ToolError::Io(e.to_string()))?;
        let _ = child.child.wait().await;
        Ok(version_of(&path, Tool::YtDlp).await.unwrap_or_default())
    }
}

async fn version_of(path: &Path, tool: Tool) -> Option<String> {
    let mut child = process::spawn(path, tool.version_args(), true).ok()?;
    let mut out = child.child.stdout.take()?;
    let mut text = String::new();
    let read = async {
        use tokio::io::AsyncReadExt;
        let _ = out.read_to_string(&mut text).await;
        let _ = child.child.wait().await;
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), read)
        .await
        .ok()?;
    text.lines()
        .next()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_copy_wins_over_path() {
        let dir = tempfile::tempdir().unwrap();
        let tools = Tools::new(dir.path().to_path_buf());
        assert_eq!(
            tools
                .locate(Tool::YtDlp)
                .filter(|p| p.starts_with(dir.path())),
            None
        );
        std::fs::write(dir.path().join("yt-dlp.exe"), b"x").unwrap();
        assert_eq!(
            tools.locate(Tool::YtDlp),
            Some(dir.path().join("yt-dlp.exe"))
        );
    }
}
