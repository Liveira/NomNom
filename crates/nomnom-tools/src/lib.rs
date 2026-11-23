//! Locates, installs and updates the external tools NOMNOM drives: yt-dlp and ffmpeg/ffprobe.
//!
//! Nothing here runs unless the user asks. Installs download from the tools' official GitHub
//! releases and verify the SHA-256 listed in the same release before anything is put in place.

use nomnom_core::process;
use std::path::{Path, PathBuf};

mod install;
pub mod update;

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
    Adb,
    Scrcpy,
}

impl Tool {
    pub fn exe_name(self) -> &'static str {
        #[cfg(windows)]
        match self {
            Tool::YtDlp => "yt-dlp.exe",
            Tool::Ffmpeg => "ffmpeg.exe",
            Tool::Ffprobe => "ffprobe.exe",
            Tool::Adb => "adb.exe",
            Tool::Scrcpy => "scrcpy.exe",
        }
        #[cfg(not(windows))]
        match self {
            Tool::YtDlp => "yt-dlp",
            Tool::Ffmpeg => "ffmpeg",
            Tool::Ffprobe => "ffprobe",
            Tool::Adb => "adb",
            Tool::Scrcpy => "scrcpy",
        }
    }

    fn version_args(self) -> &'static [&'static str] {
        match self {
            Tool::YtDlp | Tool::Scrcpy => &["--version"],
            Tool::Adb => &["version"],
            _ => &["-version"],
        }
    }
}

fn is_usable_binary(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        p.metadata()
            .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        p.is_file()
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
        match tool {
            Tool::Adb | Tool::Scrcpy => self.dir.join("scrcpy").join(tool.exe_name()),
            _ => self.dir.join(tool.exe_name()),
        }
    }

    /// Managed copy first, then anything on PATH, then well-known SDK folders for ADB.
    pub fn locate(&self, tool: Tool) -> Option<PathBuf> {
        let managed = self.managed_path(tool);
        if is_usable_binary(&managed) {
            return Some(managed);
        }
        if let Some(path_var) = std::env::var_os("PATH") {
            if let Some(found) = std::env::split_paths(&path_var)
                .map(|d| d.join(tool.exe_name()))
                .find(|p| is_usable_binary(p))
            {
                return Some(found);
            }
        }
        #[cfg(windows)]
        if tool == Tool::Adb {
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                let sdk_adb = PathBuf::from(local)
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join("adb.exe");
                if is_usable_binary(&sdk_adb) {
                    return Some(sdk_adb);
                }
            }
        }
        None
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

    /// Downloads ffmpeg and ffprobe. The archive is large (about 120–150 MB).
    pub async fn install_ffmpeg(&self, progress: &InstallProgress) -> Result<(), ToolError> {
        install::install_ffmpeg(&self.dir, progress).await
    }

    /// Downloads scrcpy (which bundles adb on Windows and Linux) into `tools/scrcpy/`.
    pub async fn install_scrcpy(&self, progress: &InstallProgress) -> Result<(), ToolError> {
        install::install_scrcpy(&self.dir, progress).await
    }

    /// Updates the managed yt-dlp in place. Never touches a copy found on PATH (such as one
    /// managed by `pacman` or `apt`).
    pub async fn update_ytdlp(&self) -> Result<String, ToolError> {
        let path = self.managed_path(Tool::YtDlp);
        if !is_usable_binary(&path) {
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
        let bin = dir.path().join(Tool::YtDlp.exe_name());
        std::fs::write(&bin, b"x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(tools.locate(Tool::YtDlp), Some(bin));
    }
}
