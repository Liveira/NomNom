use crate::state::AppState;
use nomnom_core::module::ModuleManifest;
use nomnom_core::settings::{Settings, Theme};
use nomnom_core::tasks::Task;
use serde::Deserialize;
use tauri::State;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn list_modules(state: State<'_, AppState>) -> Vec<ModuleManifest> {
    state.modules.manifests()
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    close_to_tray: Option<bool>,
    theme: Option<Theme>,
    motion: Option<nomnom_core::settings::Motion>,
    /// One of: auto, en, pt, es.
    language: Option<String>,
    sidebar_collapsed: Option<bool>,
    max_concurrent: Option<u8>,
    /// Empty string resets to the Downloads folder.
    output_dir: Option<String>,
    allow_duplicates: Option<bool>,
    firefox_cookies: Option<bool>,
    notify_downloads: Option<bool>,
    quick_keep_ready: Option<bool>,
    launch_from_browser: Option<bool>,
    organizer_dest: Option<String>,
    look_inside_folders: Option<u8>,
    organizer_conflict: Option<String>,
    auto_organize: Option<bool>,
    link_downloads: Option<bool>,
    default_kind: Option<nomnom_core::settings::DefaultKind>,
    default_video: Option<String>,
    default_audio: Option<String>,
    sort_into_folders: Option<bool>,
    shell_menu: Option<bool>,
    autostart: Option<bool>,
    first_run_done: Option<bool>,
    storage_schedule: Option<String>,
    storage_roots: Option<Vec<String>>,
    storage_keep: Option<u32>,
    storage_depth: Option<u8>,
    storage_big_mb: Option<u32>,
    auto_update_check: Option<bool>,
    subtitles: Option<bool>,
    monitor_tray: Option<bool>,
    subtitle_langs: Option<String>,
    activity_enabled: Option<bool>,
    activity_idle_secs: Option<u32>,
    activity_track_nomnom: Option<bool>,
    sidebar_tools: Option<Vec<String>>,
    renderer: Option<String>,
    pretty_notifications: Option<bool>,
    goal_reminder_hour: Option<u8>,
    weekly_report: Option<bool>,
    clipboard_enabled: Option<bool>,
    clipboard_keep_days: Option<u32>,
    text_enabled: Option<bool>,
    text_target: Option<String>,
    text_ocr: Option<bool>,
    text_ocr_translate: Option<bool>,
    phone_dir: Option<String>,
    phone_idle_minutes: Option<u32>,
    launcher_on: Option<bool>,
}

#[tauri::command]
pub fn default_download_dir(state: State<'_, AppState>) -> String {
    state.paths.downloads.to_string_lossy().into_owned()
}

#[tauri::command]
pub fn update_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> CmdResult<Settings> {
    if let Some(l) = patch.language.as_deref() {
        if !matches!(l, "auto" | "en" | "pt" | "es") {
            return Err("Pick Automatic, English, Portugues or Espanol.".into());
        }
    }
    if let Some(t) = patch.text_target.as_deref() {
        if t != "auto"
            && !nomnom_core::translate::LANGUAGES
                .iter()
                .any(|l| l.code == t)
        {
            return Err("Pick a language from the list.".into());
        }
    }
    if let Some(r) = patch.renderer.as_deref() {
        if !matches!(r, "auto" | "gpu" | "cpu") {
            return Err("Pick Automatic, Graphics card or Processor only.".into());
        }
    }
    if let Some(c) = patch.organizer_conflict.as_deref() {
        if c != "skip" && c != "keep_both" {
            return Err("Pick Skip or Keep both.".into());
        }
    }
    if let Some(dir) = patch.organizer_dest.as_deref().filter(|d| !d.is_empty()) {
        let (zones, _) = state.paths.protected_folders();
        if !crate::organizer::dest_is_acceptable(std::path::Path::new(dir), &zones) {
            return Err(
                "Pick a folder that isn't a whole drive or a system or program folder.".into(),
            );
        }
    }
    if let Some(v) = patch.default_video.as_deref() {
        if !crate::modules::downloads::VIDEO_FORMATS.contains(&v) {
            return Err("That isn't a video format NomNom can save.".into());
        }
    }
    if let Some(a) = patch.default_audio.as_deref() {
        if !crate::modules::downloads::AUDIO_FORMATS.contains(&a) {
            return Err("That isn't an audio format NomNom can save.".into());
        }
    }
    if let Some(dir) = patch.output_dir.as_deref().filter(|d| !d.is_empty()) {
        if !std::path::Path::new(dir).is_dir() {
            return Err("That folder doesn't exist.".into());
        }
    }
    if let Some(v) = patch.storage_schedule.as_deref() {
        if !["off", "on_start", "daily", "weekly"].contains(&v) {
            return Err("Pick Off, When NomNom starts, Daily or Weekly.".into());
        }
    }
    if let Some(roots) = &patch.storage_roots {
        if roots.len() > 10 {
            return Err("Pick at most 10 folders to scan on a schedule.".into());
        }
        for r in roots {
            let p = std::path::Path::new(r);
            if !p.is_absolute() || nomnom_organizer::pathutil::has_traversal(p) || !p.is_dir() {
                return Err(format!("{r} isn't a folder NomNom can scan."));
            }
        }
    }
    let result = state
        .settings
        .update("app", |s| {
            if let Some(v) = patch.max_concurrent {
                s.downloads.max_concurrent = v.clamp(1, 8);
            }
            if let Some(v) = &patch.output_dir {
                s.downloads.output_dir = (!v.is_empty()).then(|| v.into());
            }
            if let Some(v) = patch.allow_duplicates {
                s.downloads.allow_duplicates = v;
            }
            if let Some(v) = patch.firefox_cookies {
                s.downloads.use_firefox_cookies = v;
            }
            if let Some(v) = patch.default_kind {
                s.downloads.default_kind = v;
            }
            if let Some(v) = &patch.default_video {
                s.downloads.default_video = v.clone();
            }
            if let Some(v) = &patch.default_audio {
                s.downloads.default_audio = v.clone();
            }
            if let Some(v) = patch.sort_into_folders {
                s.downloads.sort_into_folders = v;
            }
            if let Some(v) = &patch.organizer_dest {
                s.organizer.dest_root = (!v.is_empty()).then(|| v.into());
            }
            if let Some(v) = patch.look_inside_folders {
                s.organizer.look_inside_folders = v.min(3);
            }
            if let Some(v) = &patch.organizer_conflict {
                s.organizer.conflict = v.clone();
            }
            if let Some(v) = patch.auto_organize {
                s.organizer.auto_enabled = v;
            }
            if let Some(v) = patch.link_downloads {
                s.organizer.link_downloads = v;
            }
            if let Some(v) = patch.launch_from_browser {
                s.browser.launch_from_browser = v;
            }
            if let Some(v) = patch.quick_keep_ready {
                s.shortcuts.keep_ready = v;
            }
            if let Some(v) = patch.notify_downloads {
                s.downloads.notify = v;
            }
            if let Some(v) = patch.shell_menu {
                s.system.shell_menu = v;
            }
            if let Some(v) = patch.autostart {
                s.system.autostart = v;
            }
            if let Some(v) = &patch.sidebar_tools {
                // Only short names made of letters, each once.
                let mut kept: Vec<String> = Vec::new();
                for id in v.iter().take(32) {
                    let ok = !id.is_empty()
                        && id.len() <= 24
                        && id.chars().all(|c| c.is_ascii_lowercase() || c == '_');
                    if ok && !kept.contains(id) {
                        kept.push(id.clone());
                    }
                }
                s.appearance.sidebar_tools = Some(kept);
            }
            if let Some(v) = patch.first_run_done {
                s.system.first_run_done = v;
            }
            if let Some(v) = &patch.storage_schedule {
                s.storage.schedule = v.clone();
            }
            if let Some(v) = &patch.storage_roots {
                let mut seen: Vec<std::path::PathBuf> = Vec::new();
                for r in v {
                    let p = std::path::PathBuf::from(r);
                    if !seen
                        .iter()
                        .any(|x| nomnom_organizer::pathutil::is_same(x, &p))
                    {
                        seen.push(p);
                    }
                }
                s.storage.schedule_roots = seen;
            }
            if let Some(v) = patch.storage_keep {
                s.storage.keep_scheduled = v.clamp(1, 100);
            }
            if let Some(v) = patch.storage_depth {
                s.storage.snapshot_depth = v.clamp(1, 12);
            }
            if let Some(v) = patch.monitor_tray {
                s.monitor.tray_tooltip = v;
            }
            if let Some(v) = patch.subtitles {
                s.downloads.subtitles = v;
            }
            if let Some(v) = &patch.subtitle_langs {
                s.downloads.subtitle_langs = v.chars().take(60).collect();
            }
            if let Some(v) = patch.auto_update_check {
                s.updates.auto_check = v;
            }
            if let Some(v) = patch.storage_big_mb {
                s.storage.big_file_mb = v.clamp(1, 100_000);
            }
            if let Some(v) = patch.close_to_tray {
                s.general.close_to_tray = v;
            }
            if let Some(v) = patch.theme {
                s.appearance.theme = v;
            }
            if let Some(v) = patch.motion {
                s.appearance.motion = v;
            }
            if let Some(v) = &patch.language {
                s.general.language = v.clone();
            }
            if let Some(v) = &patch.renderer {
                s.general.renderer = v.clone();
            }
            if let Some(v) = patch.goal_reminder_hour {
                s.activity.goal_reminder_hour = v.min(24);
            }
            if let Some(v) = &patch.phone_dir {
                s.phone.save_dir = (!v.trim().is_empty()).then(|| v.clone());
            }
            if let Some(v) = patch.phone_idle_minutes {
                s.phone.idle_minutes = v.clamp(1, 240);
            }
            if let Some(v) = patch.launcher_on {
                s.shortcuts.launcher_on = v;
            }
            if let Some(v) = patch.text_ocr {
                s.text.ocr = v;
            }
            if let Some(v) = patch.text_ocr_translate {
                s.text.ocr_translate = v;
            }
            if let Some(v) = patch.text_enabled {
                s.text.enabled = v;
            }
            if let Some(v) = &patch.text_target {
                s.text.target = v.clone();
            }
            if let Some(v) = patch.clipboard_enabled {
                s.clipboard.enabled = v;
            }
            if let Some(v) = patch.clipboard_keep_days {
                s.clipboard.keep_days = v.min(3650);
            }
            if let Some(v) = patch.weekly_report {
                s.activity.weekly_report = v;
            }
            if let Some(v) = patch.pretty_notifications {
                s.general.pretty_notifications = v;
            }
            if let Some(v) = patch.sidebar_collapsed {
                s.appearance.sidebar_collapsed = v;
            }
            if let Some(v) = patch.activity_enabled {
                s.activity.enabled = v;
            }
            if let Some(v) = patch.activity_idle_secs {
                s.activity.idle_timeout_secs = v.clamp(60, 3600);
            }
            if let Some(v) = patch.activity_track_nomnom {
                s.activity.track_nomnom = v;
            }
        })
        .map_err(|e| e.to_string());
    if patch.auto_organize.is_some() {
        crate::organizer::apply_auto(&app);
    }
    if patch.launch_from_browser.is_some() {
        crate::browser::write_integration(&state);
    }
    if patch.shell_menu.is_some() {
        crate::integrations::apply_shell(&state);
    }
    if patch.autostart.is_some() {
        crate::integrations::apply_autostart(&state);
    }
    if patch.language.is_some() {
        crate::tray::relabel(&app);
    }
    if patch.clipboard_enabled.is_some()
        || patch.text_enabled.is_some()
        || patch.text_ocr.is_some()
        || patch.launcher_on.is_some()
    {
        crate::shortcuts::sync(&app);
    }
    if patch.subtitles.is_some() || patch.subtitle_langs.is_some() {
        crate::modules::downloads::apply_subtitles(&state);
    }
    if patch.quick_keep_ready.is_some() {
        crate::quick::apply_keep_ready(&app);
    }
    if patch.max_concurrent.is_some() {
        let max = state.settings.get().downloads.max_concurrent as usize;
        state.tasks.set_limit("downloads", max);
    }
    result
}

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>) -> Vec<Task> {
    state.tasks.list()
}

/// The frontend reports when it first rendered so startup time can be measured on release builds.
#[tauri::command]
pub fn app_ready(state: State<'_, AppState>, ms: f64) {
    tracing::info!(
        webview_ms = ms,
        since_process_start_ms = state.started.elapsed().as_millis() as u64,
        "main window ready"
    );
}

#[derive(serde::Serialize)]
pub struct ShortcutInfo {
    id: &'static str,
    accelerator: String,
    /// Why it isn't active, when it isn't.
    error: Option<String>,
}

#[tauri::command]
pub fn shortcuts_status(state: State<'_, AppState>) -> Vec<ShortcutInfo> {
    let s = state.settings.get().shortcuts;
    let errors = state.shortcut_errors.lock().unwrap();
    [
        (crate::shortcuts::QUICK, s.quick_download),
        (crate::shortcuts::OPEN, s.open_app),
        (crate::shortcuts::CLIPBOARD, s.clipboard),
        (crate::shortcuts::TRANSLATE_REPLACE, s.translate_replace),
        (crate::shortcuts::TRANSLATE_POPUP, s.translate_popup),
        (crate::shortcuts::CAPTURE, s.capture_text),
        (crate::shortcuts::LAUNCHER, s.launcher),
    ]
    .into_iter()
    .map(|(id, accelerator)| ShortcutInfo {
        id,
        accelerator,
        error: errors.get(id).cloned(),
    })
    .collect()
}

#[tauri::command]
pub fn set_shortcut(app: tauri::AppHandle, id: String, accelerator: String) -> CmdResult<()> {
    crate::shortcuts::change(&app, &id, &accelerator)
}
