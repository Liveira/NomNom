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
            return Err("That isn't a video format NOMNOM can save.".into());
        }
    }
    if let Some(a) = patch.default_audio.as_deref() {
        if !crate::modules::downloads::AUDIO_FORMATS.contains(&a) {
            return Err("That isn't an audio format NOMNOM can save.".into());
        }
    }
    if let Some(dir) = patch.output_dir.as_deref().filter(|d| !d.is_empty()) {
        if !std::path::Path::new(dir).is_dir() {
            return Err("That folder doesn't exist.".into());
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
            if let Some(v) = patch.first_run_done {
                s.system.first_run_done = v;
            }
            if let Some(v) = patch.close_to_tray {
                s.general.close_to_tray = v;
            }
            if let Some(v) = patch.theme {
                s.appearance.theme = v;
            }
            if let Some(v) = patch.sidebar_collapsed {
                s.appearance.sidebar_collapsed = v;
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
