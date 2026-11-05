//! The data the organizer passes around. Everything the interface shows comes from these types,
//! and nothing the interface sends back is ever used as a path (it sends only an id).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub exts: Vec<String>,
}

/// What to do when the destination already has a file with that name. There is no "replace".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    /// Leave the file where it is.
    #[default]
    Skip,
    /// Move it as `name (2).ext`.
    KeepBoth,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub source: PathBuf,
    pub dest_root: PathBuf,
    /// 0 looks only at files directly in `source`; 1 or more also opens folders that pass the
    /// software check, that many levels down.
    pub max_depth: u32,
    pub categories: Vec<Category>,
    /// Extensions to leave alone even when they belong to a category.
    pub excluded_exts: Vec<String>,
    /// Only these categories are moved (empty means all).
    pub only_categories: Vec<String>,
    pub conflict: ConflictPolicy,
    /// Files changed more recently than this many seconds are left alone (still downloading).
    pub settle_secs: u64,
    pub max_entries: usize,
    /// Places the source may never be inside of (Windows, Program Files, app data...). Filled from
    /// the system's known folders by the app; tests pass their own.
    pub protected_zones: Vec<PathBuf>,
    /// Places the source may never be exactly (the user profile folder, drive roots).
    pub protected_exact: Vec<PathBuf>,
}

impl Options {
    pub fn new(source: PathBuf, dest_root: PathBuf) -> Self {
        Options {
            source,
            dest_root,
            max_depth: 0,
            categories: crate::classify::default_categories(),
            excluded_exts: vec![],
            only_categories: vec![],
            conflict: ConflictPolicy::Skip,
            settle_secs: 30,
            max_entries: 50_000,
            protected_zones: vec![],
            protected_exact: vec![],
        }
    }
}

/// What a file looked like when it was planned. It is checked again right before it moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub size: u64,
    pub mtime: i64,
    pub volume: u32,
    pub index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Move {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub category: String,
    pub fp: Fingerprint,
    /// The destination is on another drive, so the file is copied, checked, then removed.
    pub cross_volume: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipCode {
    Link,
    Hidden,
    Cloud,
    InProgress,
    RecentlyChanged,
    Locked,
    Unknown,
    Mismatch,
    Program,
    Archive,
    Empty,
    SoftwareFolder,
    FolderNotOpened,
    Protected,
    Conflict,
    CrossVolume,
    AlreadyOrganized,
    Unreadable,
    Excluded,
    TooMany,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Skip {
    pub path: PathBuf,
    pub is_dir: bool,
    pub code: SkipCode,
    /// A sentence for the user.
    pub message: String,
}

impl Skip {
    pub fn new(
        path: impl Into<PathBuf>,
        is_dir: bool,
        code: SkipCode,
        message: impl Into<String>,
    ) -> Self {
        Skip {
            path: path.into(),
            is_dir,
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub source: PathBuf,
    pub dest_root: PathBuf,
    pub moves: Vec<Move>,
    pub skips: Vec<Skip>,
}

impl Plan {
    pub fn cross_volume_count(&self) -> usize {
        self.moves.iter().filter(|m| m.cross_volume).count()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OrganizerError {
    #[error("{0}")]
    Refused(String),
    #[error("{0}")]
    Io(String),
    #[error("journal error: {0}")]
    Journal(String),
}
