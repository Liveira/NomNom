//! Typed settings with atomic persistence and change notifications.
//! Unknown keys in the file are preserved so newer/older versions don't clobber each other.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    /// Closing the main window keeps NOMNOM running in the tray.
    pub close_to_tray: bool,
    pub tray_notice_shown: bool,
}
impl Default for General {
    fn default() -> Self {
        General {
            close_to_tray: true,
            tray_notice_shown: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Appearance {
    pub theme: Theme,
    pub sidebar_collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub general: General,
    pub appearance: Appearance,
    /// Sections owned by modules or by newer versions, kept verbatim.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: 1,
            general: General::default(),
            appearance: Appearance::default(),
            extra: Map::new(),
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<Settings>,
    changed: broadcast::Sender<String>,
}

impl SettingsStore {
    /// Loads settings from `path`. A missing file yields defaults; a corrupt file is
    /// moved aside as `settings.json.bad` and defaults are used.
    pub fn load(path: &Path) -> Result<Self> {
        let settings = match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("settings file is corrupt ({e}); starting from defaults");
                    let _ = std::fs::rename(path, path.with_extension("json.bad"));
                    Settings::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => return Err(Error::io(path, e)),
        };
        let (tx, _) = broadcast::channel(16);
        Ok(SettingsStore {
            path: path.to_path_buf(),
            current: RwLock::new(settings),
            changed: tx,
        })
    }

    pub fn get(&self) -> Settings {
        self.current.read().unwrap().clone()
    }

    /// Applies `f`, persists atomically, and notifies subscribers with `section`.
    pub fn update(&self, section: &str, f: impl FnOnce(&mut Settings)) -> Result<Settings> {
        let snapshot = {
            let mut guard = self.current.write().unwrap();
            f(&mut guard);
            guard.clone()
        };
        self.save(&snapshot)?;
        let _ = self.changed.send(section.to_string());
        Ok(snapshot)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.changed.subscribe()
    }

    fn save(&self, s: &Settings) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(s)?;
        std::fs::write(&tmp, text).map_err(|e| Error::io(&tmp, e))?;
        // On Windows, std::fs::rename replaces an existing destination
        // (MoveFileExW with MOVEFILE_REPLACE_EXISTING).
        std::fs::rename(&tmp, &self.path).map_err(|e| Error::io(&self.path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_unknown_keys_survive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"version":1,"downloads":{"max_concurrent":5},"appearance":{"theme":"dark"}}"#,
        )
        .unwrap();
        let store = SettingsStore::load(&path).unwrap();
        assert_eq!(store.get().appearance.theme, Theme::Dark);
        store
            .update("general", |s| s.general.close_to_tray = false)
            .unwrap();

        let again = SettingsStore::load(&path).unwrap().get();
        assert!(!again.general.close_to_tray);
        assert_eq!(again.extra["downloads"]["max_concurrent"], 5);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{ not json").unwrap();
        let store = SettingsStore::load(&path).unwrap();
        assert!(store.get().general.close_to_tray);
        assert!(dir.path().join("settings.json.bad").exists());
    }

    #[test]
    fn update_notifies_subscribers() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::load(&dir.path().join("s.json")).unwrap();
        let mut rx = store.subscribe();
        store
            .update("appearance", |s| s.appearance.theme = Theme::Light)
            .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "appearance");
    }
}
