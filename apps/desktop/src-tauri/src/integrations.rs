//! Windows integration: the right-click menu (modern package, with the classic menu as a
//! fallback) and starting with Windows. Everything is per-user and undone by `uninstall_all`.
//!
//! Debug builds can point these at throwaway registry keys with `NOMNOM_TEST_CLASSES_ROOT` and
//! `NOMNOM_TEST_RUN_KEY` (only keys starting with `Software\NOMNOM-Test`), so testing never touches
//! the real right-click menu or the real Run key. That code is compiled out of release builds.

use crate::state::AppState;
use nomnom_win::{autostart, shell_menu, sparse};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[cfg(debug_assertions)]
fn test_key(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .filter(|v| v.starts_with(r"Software\NOMNOM-Test") && !v.contains(".."))
}

fn classes_root() -> String {
    #[cfg(debug_assertions)]
    if let Some(r) = test_key("NOMNOM_TEST_CLASSES_ROOT") {
        return r;
    }
    shell_menu::CLASSES_ROOT.to_string()
}

fn run_key() -> String {
    #[cfg(debug_assertions)]
    if let Some(r) = test_key("NOMNOM_TEST_RUN_KEY") {
        return r;
    }
    autostart::RUN_KEY.to_string()
}

/// True when the real Windows package manager may be asked to add or remove the menu package.
fn real_package_allowed() -> bool {
    #[cfg(debug_assertions)]
    if test_key("NOMNOM_TEST_CLASSES_ROOT").is_some() {
        return false;
    }
    true
}

fn exe() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

fn owned_file(state: &AppState) -> PathBuf {
    state.paths.data_dir.join("registry-owned.json")
}

/// The signed package and the DLL it points at, when this install has them.
fn modern_files() -> Option<(PathBuf, PathBuf)> {
    let dir = exe()?.parent()?.to_path_buf();
    let msix = dir.join("nomnom-shell.msix");
    (msix.is_file() && dir.join("nomnom_shell.dll").is_file()).then_some((msix, dir))
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct ShellStatus {
    /// What the user chose.
    pub enabled: bool,
    /// "modern" (Windows 11 menu), "classic" (Show more options), or "none".
    pub mode: &'static str,
    /// Plain sentence about why the Windows 11 menu isn't in use, when it isn't.
    pub note: Option<String>,
}

pub fn shell_status(state: &AppState) -> ShellStatus {
    let enabled = state.settings.get().system.shell_menu;
    let modern = real_package_allowed() && sparse::is_registered();
    let classic = shell_menu::classic_installed(&classes_root());
    let mode = if !enabled {
        "none"
    } else if modern {
        "modern"
    } else if classic {
        "classic"
    } else {
        "none"
    };
    let note = if enabled && !modern {
        state.shell_note.lock().unwrap().clone()
    } else {
        None
    };
    ShellStatus {
        enabled,
        mode,
        note,
    }
}

/// Makes the registry and package match the setting. Never fails loudly: whatever couldn't be done
/// is reported in the status, and the classic menu is the fallback.
pub fn apply_shell(state: &AppState) {
    crate::browser::write_integration(state);
    let enabled = state.settings.get().system.shell_menu;
    let owned = owned_file(state);
    let root = classes_root();
    if !enabled {
        // The modern entry hides itself from integration.json; only the classic keys need removing.
        if let Err(e) = shell_menu::uninstall_classic(&root, &owned) {
            tracing::warn!("couldn't remove the classic menu: {e}");
        }
        *state.shell_note.lock().unwrap() = None;
        return;
    }
    let mut note = None;
    let mut modern = real_package_allowed() && sparse::is_registered();
    if !modern {
        match (real_package_allowed(), modern_files()) {
            (false, _) => note = Some("Test mode: the Windows 11 menu isn't used.".to_string()),
            (true, None) => {
                note = Some(
                    "This copy of NomNom doesn't include the Windows 11 menu package.".to_string(),
                )
            }
            (true, Some((msix, dir))) => match sparse::register(&msix, &dir) {
                Ok(()) => modern = true,
                Err(e) => {
                    tracing::info!("windows 11 menu not available: {e}");
                    note = Some(e.to_string());
                }
            },
        }
    }
    if modern {
        // The Windows 11 menu is in use; the classic entries would only repeat it.
        let _ = shell_menu::uninstall_classic(&root, &owned);
    } else if let Some(exe) = exe() {
        if let Err(e) = shell_menu::install_classic(&exe, &root, &owned) {
            tracing::warn!("couldn't add the classic menu: {e}");
            note = Some(e.to_string());
        }
        if note.is_some() {
            note = note.map(|n| format!("{n} Using the classic menu (Show more options)."));
        }
    }
    *state.shell_note.lock().unwrap() = note;
}

pub fn apply_autostart(state: &AppState) {
    let on = state.settings.get().system.autostart;
    if let Some(exe) = exe() {
        if let Err(e) = autostart::set(on, &exe, &run_key()) {
            tracing::warn!("couldn't change start with Windows: {e}");
        }
    }
}

pub fn autostart_on(_state: &AppState) -> bool {
    exe().is_some_and(|e| autostart::status(&e, &run_key()) == Some(true))
}

/// What the installer runs. Before the user has answered the first-run screen nothing is set up
/// (that screen asks first). Afterwards this repairs everything to match their choices.
/// With `defaults` (silent installs) the defaults are used and the first-run screen is skipped.
pub fn install_all(state: &AppState, defaults: bool) -> Result<(), String> {
    if defaults {
        let _ = state.settings.update("system", |s| {
            s.system = Default::default();
            s.system.shell_menu = true;
            s.system.first_run_done = true;
        });
    }
    if !state.settings.get().system.first_run_done {
        return Ok(());
    }
    let firefox = crate::browser::connect(state);
    apply_shell(state);
    apply_autostart(state);
    firefox
}

/// What the uninstaller runs. Removes only what NOMNOM added.
pub fn uninstall_all(state: &AppState) -> Result<(), String> {
    let mut problems = Vec::new();
    if let Err(e) = crate::browser::disconnect(state) {
        problems.push(e);
    }
    if let Err(e) = shell_menu::uninstall_classic(&classes_root(), &owned_file(state)) {
        problems.push(e.to_string());
    }
    if real_package_allowed() {
        if let Err(e) = sparse::unregister() {
            problems.push(e.to_string());
        }
    }
    if let Some(exe) = exe() {
        if let Err(e) = autostart::set(false, &exe, &run_key()) {
            problems.push(e);
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join(" "))
    }
}

#[derive(Serialize)]
pub struct IntegrationStatus {
    pub shell: ShellStatus,
    pub autostart: bool,
    /// The Windows 11 package and its helper are part of this install.
    pub has_modern_package: bool,
}

#[tauri::command]
pub fn integration_status(state: State<'_, AppState>) -> IntegrationStatus {
    IntegrationStatus {
        shell: shell_status(&state),
        autostart: autostart_on(&state),
        has_modern_package: modern_files().is_some(),
    }
}

/// Applies everything again: repairs a menu that points at an old folder or a missing entry.
#[tauri::command]
pub fn integration_repair(state: State<'_, AppState>) -> IntegrationStatus {
    apply_shell(&state);
    apply_autostart(&state);
    integration_status(state)
}
