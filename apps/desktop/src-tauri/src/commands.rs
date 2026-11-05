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
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, patch: SettingsPatch) -> CmdResult<Settings> {
    state
        .settings
        .update("app", |s| {
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
        .map_err(|e| e.to_string())
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
