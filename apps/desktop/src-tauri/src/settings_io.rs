//! Saving the settings to a file and loading them back, for a new computer or after a fresh
//! install. What belongs to one computer (how NomNom starts, its update state) is not carried over.

use crate::state::AppState;
use nomnom_core::settings::Settings;
use std::path::Path;
use tauri::{AppHandle, State};

type CmdResult<T> = Result<T, String>;

const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// Writes the settings as one JSON file. A folder, or a file that is not `.json`, is refused.
#[tauri::command]
pub fn settings_export(state: State<'_, AppState>, path: String) -> CmdResult<()> {
    let target = Path::new(&path);
    if target.is_dir()
        || target
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("json"))
    {
        return Err("Choose a file name that ends in .json.".into());
    }
    let text = serde_json::to_string_pretty(&state.settings.get()).map_err(|e| e.to_string())?;
    std::fs::write(target, text).map_err(|e| format!("Couldn't save the file: {e}"))
}

/// Reads a settings file made by NomNom and applies it. Nothing changes unless the whole file reads.
#[tauri::command]
pub fn settings_import(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CmdResult<Settings> {
    let source = Path::new(&path);
    let meta = std::fs::metadata(source).map_err(|_| "That file isn't there.".to_string())?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err("That doesn't look like a NomNom settings file.".into());
    }
    let text =
        std::fs::read_to_string(source).map_err(|e| format!("Couldn't read the file: {e}"))?;
    let imported: Settings = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|_| "That doesn't look like a NomNom settings file.".to_string())?;
    // Every saved action has to make sense before anything is replaced.
    for a in &imported.actions {
        nomnom_core::actions::validate(a)
            .map_err(|e| format!("An action in the file is not valid: {e}"))?;
    }
    nomnom_core::actions::check_list(&imported.actions)?;
    let merged = state.settings.get().with_imported(imported);
    let saved = state
        .settings
        .update("all", |s| *s = merged)
        .map_err(|e| e.to_string())?;
    crate::shortcuts::sync(&app);
    crate::shortcuts::sync_custom(&app);
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use nomnom_core::settings::Settings;

    #[test]
    fn what_belongs_to_this_computer_is_kept_and_the_rest_comes_from_the_file() {
        let mut here = Settings::default();
        here.system.autostart = true;
        here.system.first_run_done = true;
        let mut file = Settings::default();
        file.system.autostart = false;
        file.system.first_run_done = false;
        file.appearance.sidebar_tools = Some(vec!["notes".into()]);
        file.clipboard.keep_days = 7;
        let merged = here.with_imported(file);
        assert!(merged.system.autostart && merged.system.first_run_done);
        assert_eq!(
            merged.appearance.sidebar_tools,
            Some(vec!["notes".to_string()])
        );
        assert_eq!(merged.clipboard.keep_days, 7);
    }
}
