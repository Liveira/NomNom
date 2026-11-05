//! File conversion for NOMNOM File Tools.
//!
//! Sources are only ever read. Results are written to a temporary file, checked, and then moved
//! into place under a name that isn't taken. No Tauri code lives here.

pub mod batch;
mod error;
pub mod ffmpeg;
mod image_conv;
pub mod model;
pub mod naming;
pub mod office;
pub mod pdf;
pub mod presets;

pub use batch::{run, Event, Outcome, Request, Status};
pub use error::ConvertError;
pub use model::{Backend, Kind, Options, PdfPage, Target, VideoQuality};
pub use presets::{presets, Preset};

use nomnom_tools::{Tool, Tools};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// The external programs conversions can use.
#[derive(Debug, Clone, Default)]
pub struct Backends {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    pub soffice: Option<PathBuf>,
}

impl Backends {
    pub fn detect(tools: &Tools) -> Backends {
        Backends {
            ffmpeg: tools.locate(Tool::Ffmpeg),
            ffprobe: tools.locate(Tool::Ffprobe),
            soffice: office::find_soffice(),
        }
    }

    pub fn has(&self, b: Backend) -> bool {
        match b {
            Backend::Image | Backend::Pdf => true,
            Backend::Ffmpeg => self.ffmpeg.is_some() && self.ffprobe.is_some(),
            Backend::Office => self.soffice.is_some(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lossiness {
    /// The decoded picture or sound is kept exactly.
    Lossless,
    /// Some detail is discarded.
    Lossy,
    /// Copied without re-encoding when the file allows it.
    CopyWhenPossible,
}

#[derive(Debug, Clone, Serialize)]
pub struct Conversion {
    pub kind: Kind,
    pub target: Target,
    pub label: &'static str,
    pub backend: Backend,
    pub lossiness: Lossiness,
    pub available: bool,
    /// What to install when it isn't available, in words the user knows.
    pub needs: Option<&'static str>,
}

fn lossiness(kind: Kind, target: Target) -> Lossiness {
    match target {
        Target::Png | Target::Bmp | Target::Tiff | Target::Ico | Target::Wav | Target::Flac => {
            Lossiness::Lossless
        }
        Target::Pdf | Target::Html => Lossiness::Lossless,
        Target::Mkv | Target::NoAudio => Lossiness::CopyWhenPossible,
        Target::M4a if kind == Kind::Video || kind == Kind::Audio => Lossiness::CopyWhenPossible,
        Target::Mp4 | Target::Webm => Lossiness::CopyWhenPossible,
        _ => Lossiness::Lossy,
    }
}

/// Every conversion NOMNOM knows, marked with whether it can run right now.
pub fn capabilities(backends: &Backends) -> Vec<Conversion> {
    let kinds = [
        Kind::Image,
        Kind::Video,
        Kind::Audio,
        Kind::Text,
        Kind::Office,
    ];
    let mut out = Vec::new();
    for kind in kinds {
        for &target in model::targets_for(kind) {
            let backend = model::backend_for(kind, target);
            let available = backends.has(backend);
            out.push(Conversion {
                kind,
                target,
                label: target.label(),
                backend,
                lossiness: lossiness(kind, target),
                available,
                needs: match (available, backend) {
                    (true, _) => None,
                    (false, Backend::Ffmpeg) => Some("FFmpeg"),
                    (false, Backend::Office) => Some("LibreOffice"),
                    _ => None,
                },
            });
        }
    }
    out
}

/// Conversions that work for every one of `files`.
pub fn applicable(files: &[PathBuf], caps: &[Conversion]) -> Vec<Conversion> {
    if files.is_empty() {
        return vec![];
    }
    let kinds: Vec<Kind> = files.iter().filter_map(|f| Kind::of(f)).collect();
    if kinds.len() != files.len() {
        return vec![];
    }
    caps.iter()
        .filter(|c| {
            files
                .iter()
                .all(|f| model::valid_for(f, c.target) && Kind::of(f) == Some(c.kind))
        })
        .cloned()
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub kind: Option<Kind>,
    pub size: u64,
    /// Why this file can't be converted, if it can't.
    pub problem: Option<String>,
}

/// Looks at dropped paths. Folders and unknown types are reported, not expanded.
pub fn describe(paths: &[PathBuf]) -> Vec<FileInfo> {
    paths
        .iter()
        .map(|p| {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            let meta = std::fs::symlink_metadata(p);
            let (size, problem) = match &meta {
                Err(_) => (0, Some("This file couldn't be found.".to_string())),
                Ok(m) if m.is_dir() => (
                    0,
                    Some("Folders aren't converted. Drop the files inside.".to_string()),
                ),
                Ok(m) if m.file_type().is_symlink() => {
                    (0, Some("Shortcuts and links aren't converted.".to_string()))
                }
                Ok(m) if m.len() == 0 => (0, Some("This file is empty.".to_string())),
                Ok(m) => (m.len(), None),
            };
            let kind = Kind::of(p);
            let problem = problem.or_else(|| {
                kind.is_none()
                    .then(|| "NOMNOM can't convert this type of file.".to_string())
            });
            FileInfo {
                path: p.clone(),
                name,
                kind,
                size,
                problem,
            }
        })
        .collect()
}

/// Whether `dir` is a place results may be written (it exists or can be created, and isn't a drive root).
pub fn output_dir_ok(dir: &Path) -> bool {
    dir.is_absolute() && dir.parent().is_some() && !dir.to_string_lossy().contains("..")
}
