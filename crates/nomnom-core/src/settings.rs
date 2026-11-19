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
    System,
    #[default]
    Dark,
    Light,
    Aurora,
    Synthwave,
    Sakura,
    Ocean,
    Forest,
    Nebula,
    Sunset,
    Matcha,
    Lavender,
    Ember,
    Frost,
    Matrix,
    Cyberpunk,
    Candy,
    Graphite,
}

/// How much the interface moves. "Full" animates the background and the interface, "calm" keeps
/// small interface motion and a still background, "off" stops all of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    #[default]
    Full,
    Calm,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    /// Closing the main window keeps NOMNOM running in the tray.
    pub close_to_tray: bool,
    pub tray_notice_shown: bool,
    /// "auto" (the computer's language), "en", "pt" or "es".
    pub language: String,
    /// Linux only. How the web view draws: "auto", "gpu" or "cpu". Applies after a restart.
    pub renderer: String,
    /// Notifications as themed cards in the corner (otherwise the system's own).
    pub pretty_notifications: bool,
}
impl Default for General {
    fn default() -> Self {
        General {
            close_to_tray: true,
            tray_notice_shown: false,
            language: "auto".into(),
            renderer: "auto".into(),
            pretty_notifications: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub theme: Theme,
    pub motion: Motion,
    pub sidebar_collapsed: bool,
    /// The tools the person put in the sidebar. `None` means they never chose, so all show.
    pub sidebar_tools: Option<Vec<String>>,
    /// Linux: the moving background starts calm once, for settings made before that was the
    /// default. After that the person's own choice stands.
    pub linux_tuned: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            theme: Theme::default(),
            // On Linux the web view often paints without the graphics card, so the moving
            // background starts calm. It is one click away in Settings.
            motion: if cfg!(target_os = "linux") {
                Motion::Calm
            } else {
                Motion::Full
            },
            sidebar_collapsed: false,
            sidebar_tools: None,
            linux_tuned: false,
        }
    }
}

/// Whether links from the shortcut, the browser and the popup download video or audio by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DefaultKind {
    #[default]
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Downloads {
    pub max_concurrent: u8,
    /// `None` means the user's Downloads folder.
    pub output_dir: Option<std::path::PathBuf>,
    pub allow_duplicates: bool,
    /// Lets yt-dlp read Firefox cookies. Off unless the user turns it on.
    pub use_firefox_cookies: bool,
    pub notify: bool,
    pub default_kind: DefaultKind,
    /// One of: original, mp4, webm, mkv. Checked by the app before it is saved.
    pub default_video: String,
    /// One of: original, m4a, mp3, opus, ogg, flac, wav. Checked by the app before it is saved.
    pub default_audio: String,
    /// Put finished downloads in a Videos or Audio folder inside the save folder.
    pub sort_into_folders: bool,
    /// Add subtitles inside video files (needs FFmpeg).
    pub subtitles: bool,
    /// Language codes for the subtitles, like "en, pt".
    pub subtitle_langs: String,
}
impl Default for Downloads {
    fn default() -> Self {
        Downloads {
            max_concurrent: 3,
            output_dir: None,
            allow_duplicates: false,
            use_firefox_cookies: false,
            notify: true,
            default_kind: DefaultKind::Video,
            default_video: "original".into(),
            default_audio: "original".into(),
            sort_into_folders: true,
            subtitles: false,
            subtitle_langs: "en".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub quick_download: String,
    pub open_app: String,
    /// Opens the clipboard history (when it is switched on).
    pub clipboard: String,
    /// Replaces the selected text with its translation (when the translator is on).
    pub translate_replace: String,
    /// Shows the translation of the selected text next to the pointer.
    pub translate_popup: String,
    /// Reads the text in a rectangle of the screen (when that is switched on).
    pub capture_text: String,
    /// Opens the launcher: a search box for programs, files, folders and your own actions.
    pub launcher: String,
    /// Whether the launcher shortcut is taken at all.
    pub launcher_on: bool,
    /// Keep a hidden, pre-loaded popup so the shortcut responds instantly (uses memory while idle).
    pub keep_ready: bool,
}
impl Default for Shortcuts {
    fn default() -> Self {
        Shortcuts {
            quick_download: "Ctrl+Alt+D".into(),
            open_app: "Ctrl+Alt+N".into(),
            clipboard: "Ctrl+Alt+V".into(),
            translate_replace: "Ctrl+Alt+R".into(),
            translate_popup: "Ctrl+Alt+Y".into(),
            capture_text: "Ctrl+Alt+O".into(),
            launcher: "Alt+Space".into(),
            launcher_on: true,
            keep_ready: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Browser {
    /// When Firefox sends a download and NOMNOM isn't running, start it in the background
    /// (without a window).
    pub launch_from_browser: bool,
}
impl Default for Browser {
    fn default() -> Self {
        Browser {
            launch_from_browser: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Organizer {
    /// `None` means DocumentsFiles.
    pub dest_root: Option<std::path::PathBuf>,
    /// 0 looks only at files directly in the folder; 1 to 3 also opens plain-media folders.
    pub look_inside_folders: u8,
    /// "skip" or "keep_both".
    pub conflict: String,
    /// Move new downloads automatically once they have finished.
    pub auto_enabled: bool,
    /// Save NomNom Downloader's files straight into the organizer's folders.
    pub link_downloads: bool,
}
impl Default for Organizer {
    fn default() -> Self {
        Organizer {
            dest_root: None,
            look_inside_folders: 0,
            conflict: "skip".into(),
            auto_enabled: false,
            link_downloads: true,
        }
    }
}

/// Disk Analyzer: what snapshots keep, and the optional scheduled scans. Scheduled scans only read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Storage {
    /// How many folder levels a snapshot keeps.
    pub snapshot_depth: u8,
    /// Files at least this big (in MB) are kept in a snapshot by name.
    pub big_file_mb: u32,
    /// "off", "on_start", "daily" or "weekly".
    pub schedule: String,
    /// Folders and drives the schedule scans.
    pub schedule_roots: Vec<std::path::PathBuf>,
    /// How many scheduled snapshots to keep for each folder. Snapshots made by hand are never removed.
    pub keep_scheduled: u32,
}
impl Default for Storage {
    fn default() -> Self {
        Storage {
            snapshot_depth: 6,
            big_file_mb: 50,
            schedule: "off".into(),
            schedule_roots: Vec::new(),
            keep_scheduled: 12,
        }
    }
}

/// Windows integration: the right-click menu and starting with Windows. Everything is off until the
/// user answers the first-run screen (which offers the menu switched on).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct System {
    /// Show NOMNOM in the Explorer right-click menu.
    pub shell_menu: bool,
    /// Start with Windows, in the tray without a window.
    pub autostart: bool,
    /// The first-run screen has been answered.
    pub first_run_done: bool,
}

/// NomNom's own updates. The check only reads the latest release page; nothing is installed unasked.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Updates {
    /// Look for a new version when NomNom starts and then about once a day.
    pub auto_check: bool,
    /// Unix seconds of the last successful check, 0 if never.
    pub last_checked: i64,
}
impl Default for Updates {
    fn default() -> Self {
        Updates {
            auto_check: true,
            last_checked: 0,
        }
    }
}

/// Active foreground app & game time tracking (Playtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Activity {
    /// Record active foreground usage of apps and games.
    pub enabled: bool,
    /// Seconds without keyboard, mouse or gamepad input before usage pauses as AFK (60..=3600).
    pub idle_timeout_secs: u32,
    /// Whether to include NomNom's own window in tracked statistics.
    pub track_nomnom: bool,
    /// The hour (0..=23) of the "you are still short of your goal" reminder; 24 turns it off.
    pub goal_reminder_hour: u8,
    /// Make a report of each week that ends, and say so.
    pub weekly_report: bool,
}
impl Default for Activity {
    fn default() -> Self {
        Activity {
            enabled: true,
            idle_timeout_secs: 300,
            track_nomnom: false,
            goal_reminder_hour: 20,
            weekly_report: true,
        }
    }
}

/// The clipboard history. Off until the person turns it on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Clipboard {
    pub enabled: bool,
    /// Days an entry is kept after it was last copied; 0 keeps them for ever. Pinned ones stay.
    pub keep_days: u32,
}
impl Default for Clipboard {
    fn default() -> Self {
        Clipboard {
            enabled: false,
            keep_days: 30,
        }
    }
}

/// Translating selected text with DeepL, and reading text from the screen. Off until switched on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Text {
    pub enabled: bool,
    /// A DeepL language code such as "PT-BR", or "auto" for the language NomNom is shown in.
    pub target: String,
    /// Read text from the screen with a shortcut.
    pub ocr: bool,
    /// Translate the text that was read, instead of copying it as it is.
    pub ocr_translate: bool,
}
impl Default for Text {
    fn default() -> Self {
        Text {
            enabled: false,
            target: "auto".into(),
            ocr: false,
            ocr_translate: false,
        }
    }
}

/// Sending files between this computer and a phone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Phone {
    /// Where files from the phone are saved. `None` means "From phone" inside the Downloads folder.
    pub save_dir: Option<String>,
    /// Sharing turns itself off after this many quiet minutes.
    pub idle_minutes: u32,
}
impl Default for Phone {
    fn default() -> Self {
        Phone {
            save_dir: None,
            idle_minutes: 15,
        }
    }
}

/// The system monitor.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Monitor {
    /// Show processor and memory in the tray tooltip (read every few seconds while on).
    pub tray_tooltip: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub general: General,
    pub appearance: Appearance,
    pub downloads: Downloads,
    pub shortcuts: Shortcuts,
    pub browser: Browser,
    pub organizer: Organizer,
    pub system: System,
    pub storage: Storage,
    pub updates: Updates,
    pub monitor: Monitor,
    pub activity: Activity,
    pub clipboard: Clipboard,
    pub text: Text,
    pub phone: Phone,
    /// Things the person added: programs, folders, links and commands with shortcuts.
    pub actions: Vec<crate::actions::Action>,
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
            downloads: Downloads::default(),
            shortcuts: Shortcuts::default(),
            browser: Browser::default(),
            organizer: Organizer::default(),
            system: System::default(),
            storage: Storage::default(),
            updates: Updates::default(),
            monitor: Monitor::default(),
            activity: Activity::default(),
            clipboard: Clipboard::default(),
            text: Text::default(),
            phone: Phone::default(),
            actions: Vec::new(),
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
            // Editors such as Notepad save a UTF-8 byte order mark; don't treat that as corruption.
            Ok(text) => match serde_json::from_str::<Settings>(text.trim_start_matches('\u{feff}'))
            {
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
        #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
        let mut settings = settings;
        #[cfg(target_os = "linux")]
        if !settings.appearance.linux_tuned {
            settings.appearance.motion = Motion::Calm;
            settings.appearance.linux_tuned = true;
        }
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
            r#"{"version":1,"futuretool":{"level":5},"appearance":{"theme":"dark"}}"#,
        )
        .unwrap();
        let store = SettingsStore::load(&path).unwrap();
        assert_eq!(store.get().appearance.theme, Theme::Dark);
        store
            .update("general", |s| s.general.close_to_tray = false)
            .unwrap();

        let again = SettingsStore::load(&path).unwrap().get();
        assert!(!again.general.close_to_tray);
        assert_eq!(again.extra["futuretool"]["level"], 5);
    }

    #[test]
    fn file_saved_with_a_byte_order_mark_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "\u{feff}{\"general\":{\"close_to_tray\":false}}").unwrap();
        let store = SettingsStore::load(&path).unwrap();
        assert!(
            !store.get().general.close_to_tray,
            "BOM must not discard the user's settings"
        );
        assert!(!dir.path().join("settings.json.bad").exists());
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

    #[test]
    fn every_theme_and_motion_level_survives_a_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(&path).unwrap();
        assert_eq!(store.get().appearance.motion, Appearance::default().motion);
        for theme in [
            Theme::System,
            Theme::Dark,
            Theme::Light,
            Theme::Aurora,
            Theme::Synthwave,
            Theme::Sakura,
            Theme::Ocean,
            Theme::Forest,
            Theme::Nebula,
            Theme::Sunset,
            Theme::Matcha,
            Theme::Lavender,
            Theme::Ember,
            Theme::Frost,
            Theme::Matrix,
            Theme::Cyberpunk,
            Theme::Candy,
            Theme::Graphite,
        ] {
            store
                .update("appearance", |s| {
                    s.appearance.theme = theme;
                    s.appearance.motion = Motion::Calm;
                })
                .unwrap();
            let again = SettingsStore::load(&path).unwrap().get();
            assert_eq!(again.appearance.theme, theme);
            assert_eq!(again.appearance.motion, Motion::Calm);
        }
        // The names in the file are the ones the interface uses.
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"theme\": \"graphite\""), "{text}");
    }
}
