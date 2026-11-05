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
}

#[tauri::command]
pub fn default_download_dir(state: State<'_, AppState>) -> String {
    state.paths.downloads.to_string_lossy().into_owned()
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, patch: SettingsPatch) -> CmdResult<Settings> {
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
            if let Some(v) = patch.notify_downloads {
                s.downloads.notify = v;
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
