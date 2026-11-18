//! Folder backups that keep versions. Each run makes a new dated folder. A file that has not changed
//! since the last version is linked to it instead of copied, so a version costs only what changed.
//! Sources are only ever read. Nothing outside the backup folder is changed or deleted. No Tauri code.

mod glob;
mod restore;
mod run;

pub use glob::matches_name;
pub use restore::{list_versions, read_manifest, restore, verify, RestoreReport, VersionInfo};
pub use run::{run, Progress, RunReport};

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Written in every version folder. Only folders that have it are ever read as versions or removed.
pub const MANIFEST: &str = "manifest.json";
/// Marks the plan's folder, so a stray folder is never mistaken for a backup.
pub const MARKER: &str = ".nomnom-backup";
/// Most files one backup may hold. A bigger job is refused instead of running out of memory.
pub const MAX_FILES: usize = 300_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub sources: Vec<PathBuf>,
    /// The folder that holds the backups. The plan keeps its own folder inside it.
    pub destination: PathBuf,
    /// "manual", "hourly", "daily" or "weekly".
    pub schedule: String,
    /// How many versions to keep (1 to 100). Older ones are removed after a good backup.
    pub keep: u32,
    /// File and folder names to leave out, like "node_modules, *.tmp".
    pub exclude: String,
    pub enabled: bool,
}

impl Default for Plan {
    fn default() -> Self {
        Plan {
            id: String::new(),
            name: String::new(),
            sources: Vec::new(),
            destination: PathBuf::new(),
            schedule: "manual".into(),
            keep: 10,
            exclude: "node_modules, .git, *.tmp".into(),
            enabled: true,
        }
    }
}

impl Plan {
    /// A folder name made from the plan's name that is safe on every system.
    pub fn folder_name(&self) -> String {
        let cleaned: String = self
            .name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || " -_.()".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let t = cleaned.trim().trim_matches('.').to_string();
        let t: String = t.chars().take(60).collect();
        if t.is_empty() {
            "Backup".into()
        } else {
            t
        }
    }

    /// Where this plan's versions live.
    pub fn home(&self) -> PathBuf {
        self.destination.join(self.folder_name())
    }

    /// Seconds between automatic runs, or `None` for manual plans.
    pub fn interval_secs(&self) -> Option<i64> {
        match self.schedule.as_str() {
            "hourly" => Some(3600),
            "daily" => Some(86_400),
            "weekly" => Some(7 * 86_400),
            _ => None,
        }
    }

    pub fn exclude_list(&self) -> Vec<String> {
        self.exclude
            .split([',', '\n', ';'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .take(40)
            .collect()
    }

    /// Why this plan can't run, or `None` if it is fine.
    pub fn problem(&self) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some("Give the backup a name.".into());
        }
        if self.sources.is_empty() {
            return Some("Add at least one folder to back up.".into());
        }
        if self.sources.len() > 12 {
            return Some("Pick at most 12 folders for one backup.".into());
        }
        if self.destination.as_os_str().is_empty() {
            return Some("Choose where the backups are kept.".into());
        }
        if !(1..=100).contains(&self.keep) {
            return Some("Keep between 1 and 100 versions.".into());
        }
        for s in &self.sources {
            if !s.is_absolute() || !s.is_dir() {
                return Some(format!("{} isn't a folder that can be read.", s.display()));
            }
            if is_root(s) {
                return Some(
                    "A whole drive can't be backed up this way. Pick folders inside it.".into(),
                );
            }
            if inside(&self.destination, s) {
                return Some(
                    "The backups can't be kept inside a folder that is being backed up.".into(),
                );
            }
            if inside(s, &self.destination) {
                return Some(
                    "A folder being backed up can't be inside the folder that keeps the backups."
                        .into(),
                );
            }
        }
        if !self.destination.is_absolute() {
            return Some("The backup folder has to be a complete path.".into());
        }
        None
    }
}

fn is_root(p: &Path) -> bool {
    p.parent().is_none()
}

/// True when `inner` is the same as, or inside, `outer`. Compares real paths where it can.
pub fn inside(inner: &Path, outer: &Path) -> bool {
    let a = std::fs::canonicalize(inner).unwrap_or_else(|_| inner.to_path_buf());
    let b = std::fs::canonicalize(outer).unwrap_or_else(|_| outer.to_path_buf());
    let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
    let (a, b) = (norm(&a), norm(&b));
    a == b || a.starts_with(&format!("{}\\", b.trim_end_matches('\\')))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(src: &Path, dst: &Path) -> Plan {
        Plan {
            id: "p".into(),
            name: "Photos".into(),
            sources: vec![src.to_path_buf()],
            destination: dst.to_path_buf(),
            ..Plan::default()
        }
    }

    #[test]
    fn a_good_plan_has_no_problem() {
        let s = tempfile::tempdir().unwrap();
        let d = tempfile::tempdir().unwrap();
        assert_eq!(plan(s.path(), d.path()).problem(), None);
    }

    #[test]
    fn refuses_a_destination_inside_a_source_and_the_other_way_round() {
        let s = tempfile::tempdir().unwrap();
        let inner = s.path().join("backups");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(plan(s.path(), &inner).problem().is_some());
        assert!(plan(&inner, s.path()).problem().is_some());
        assert!(plan(s.path(), s.path()).problem().is_some());
    }

    #[test]
    fn refuses_empty_or_odd_plans() {
        let s = tempfile::tempdir().unwrap();
        let d = tempfile::tempdir().unwrap();
        let mut p = plan(s.path(), d.path());
        p.name = "  ".into();
        assert!(p.problem().is_some());
        let mut p = plan(s.path(), d.path());
        p.sources.clear();
        assert!(p.problem().is_some());
        let mut p = plan(s.path(), d.path());
        p.keep = 0;
        assert!(p.problem().is_some());
        let mut p = plan(&s.path().join("missing"), d.path());
        assert!(p.problem().is_some());
        p.sources = vec![PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" })];
        assert!(p.problem().is_some());
    }

    #[test]
    fn folder_names_are_safe() {
        let named = |n: &str| {
            Plan {
                name: n.into(),
                ..Plan::default()
            }
            .folder_name()
        };
        assert_eq!(named("My: photos/2024?"), "My_ photos_2024_");
        assert_eq!(named("..."), "Backup");
    }

    #[test]
    fn schedule_names_map_to_intervals() {
        assert_eq!(Plan::default().interval_secs(), None);
        let daily = Plan {
            schedule: "daily".into(),
            ..Plan::default()
        };
        assert_eq!(daily.interval_secs(), Some(86_400));
        assert_eq!(
            Plan::default().exclude_list(),
            ["node_modules", ".git", "*.tmp"]
        );
    }
}
