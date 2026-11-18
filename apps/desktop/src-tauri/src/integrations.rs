//! Desktop integration: right-click file manager menu and starting on login. Everything is
//! per-user and undone cleanly by `uninstall_all`.

use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

fn exe() -> Option<PathBuf> {
    #[cfg(not(windows))]
    if let Some(appimg) = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return Some(appimg);
    }
    std::env::current_exe().ok()
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct ShellStatus {
    /// What the user chose.
    pub enabled: bool,
    /// "modern" (Windows 11 menu), "classic" (Show more options), "linux" (Dolphin/Files/Nemo), or "none".
    pub mode: &'static str,
    /// Plain sentence about why the preferred menu isn't in use, when it isn't.
    pub note: Option<String>,
}

#[derive(Serialize)]
pub struct IntegrationStatus {
    pub platform: &'static str,
    pub shell: ShellStatus,
    pub autostart: bool,
    /// On Windows: whether the Windows 11 MSIX menu package is bundled; always true on Linux.
    pub has_modern_package: bool,
}

#[cfg(windows)]
mod imp {
    use super::*;
    use nomnom_win::{autostart, shell_menu, sparse};

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

    fn real_package_allowed() -> bool {
        #[cfg(debug_assertions)]
        {
            test_key("NOMNOM_TEST_CLASSES_ROOT").is_none()
        }
        #[cfg(not(debug_assertions))]
        {
            true
        }
    }

    fn owned_file(state: &AppState) -> PathBuf {
        state.paths.data_dir.join("registry-owned.json")
    }

    pub fn modern_files() -> Option<(PathBuf, PathBuf)> {
        let dir = exe()?.parent()?.to_path_buf();
        let msix = dir.join("nomnom-shell.msix");
        (msix.is_file() && dir.join("nomnom_shell.dll").is_file()).then_some((msix, dir))
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

    pub fn apply_shell(state: &AppState) {
        crate::browser::write_integration(state);
        let enabled = state.settings.get().system.shell_menu;
        let owned = owned_file(state);
        let root = classes_root();
        if !enabled {
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
                        "This copy of NomNom doesn't include the Windows 11 menu package."
                            .to_string(),
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

    pub fn has_modern() -> bool {
        modern_files().is_some()
    }

    pub fn uninstall_os(state: &AppState, problems: &mut Vec<String>) {
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
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    use nomnom_linux::{autostart, shell_menu};

    fn data_home() -> Option<PathBuf> {
        #[cfg(debug_assertions)]
        if let Ok(p) = std::env::var("NOMNOM_TEST_XDG_DATA") {
            let pb = PathBuf::from(p);
            if pb.is_absolute() && !pb.to_string_lossy().contains("..") {
                return Some(pb);
            }
        }
        shell_menu::default_data_home()
    }

    fn autostart_dir() -> Option<PathBuf> {
        #[cfg(debug_assertions)]
        if let Ok(p) = std::env::var("NOMNOM_TEST_AUTOSTART_DIR") {
            let pb = PathBuf::from(p);
            if pb.is_absolute() && !pb.to_string_lossy().contains("..") {
                return Some(pb);
            }
        }
        autostart::default_autostart_dir()
    }

    pub fn shell_status(state: &AppState) -> ShellStatus {
        let enabled = state.settings.get().system.shell_menu;
        let active = enabled
            && exe()
                .zip(data_home())
                .is_some_and(|(e, d)| shell_menu::installed(&e, &d));
        let mode = if active { "linux" } else { "none" };
        let note = if enabled && !active {
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

    pub fn apply_shell(state: &AppState) {
        crate::browser::write_integration(state);
        let enabled = state.settings.get().system.shell_menu;
        let Some(data) = data_home() else {
            return;
        };
        if !enabled {
            if let Err(e) = shell_menu::uninstall(&data) {
                tracing::warn!("couldn't remove file manager actions: {e}");
            }
            *state.shell_note.lock().unwrap() = None;
            return;
        }
        let mut note = None;
        if let Some(exe) = exe() {
            if let Err(e) = shell_menu::install(&exe, &data) {
                tracing::warn!("couldn't install file manager actions: {e}");
                note = Some(e);
            }
        }
        *state.shell_note.lock().unwrap() = note;
    }

    pub fn apply_autostart(state: &AppState) {
        let on = state.settings.get().system.autostart;
        if let (Some(exe), Some(dir)) = (exe(), autostart_dir()) {
            if let Err(e) = autostart::set(on, &exe, &dir) {
                tracing::warn!("couldn't change XDG autostart: {e}");
            }
        }
    }

    pub fn autostart_on(_state: &AppState) -> bool {
        exe()
            .zip(autostart_dir())
            .is_some_and(|(e, d)| autostart::status(&e, &d) == Some(true))
    }

    pub fn has_modern() -> bool {
        true
    }

    pub fn uninstall_os(_state: &AppState, problems: &mut Vec<String>) {
        if let Some(data) = data_home() {
            if let Err(e) = shell_menu::uninstall(&data) {
                problems.push(e);
            }
        }
        if let (Some(exe), Some(dir)) = (exe(), autostart_dir()) {
            if let Err(e) = autostart::set(false, &exe, &dir) {
                problems.push(e);
            }
        }
    }
}

pub fn shell_status(state: &AppState) -> ShellStatus {
    imp::shell_status(state)
}

/// A debug build started with its own test data folder (`NOMNOM_TEST_DATA`) must never change the
/// real Windows registry, the startup entry or the browser setup of the person running it, unless
/// the test names its own throwaway registry key. Release builds are never sandboxed.
#[cfg(debug_assertions)]
pub fn sandboxed() -> bool {
    std::env::var_os("NOMNOM_TEST_DATA").is_some()
        && [
            "NOMNOM_TEST_CLASSES_ROOT",
            "NOMNOM_TEST_RUN_KEY",
            "NOMNOM_TEST_FIREFOX_ROOT",
            "NOMNOM_TEST_FIREFOX_DIR",
        ]
        .iter()
        .all(|k| std::env::var_os(k).is_none())
}

#[cfg(not(debug_assertions))]
pub fn sandboxed() -> bool {
    false
}

pub fn apply_shell(state: &AppState) {
    if sandboxed() {
        return;
    }
    imp::apply_shell(state);
}

pub fn apply_autostart(state: &AppState) {
    if sandboxed() {
        return;
    }
    imp::apply_autostart(state);
}

pub fn autostart_on(state: &AppState) -> bool {
    imp::autostart_on(state)
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

/// What the uninstaller runs. Removes only what NomNom added.
pub fn uninstall_all(state: &AppState) -> Result<(), String> {
    if sandboxed() {
        return Ok(());
    }
    let mut problems = Vec::new();
    if let Err(e) = crate::browser::disconnect(state) {
        problems.push(e);
    }
    imp::uninstall_os(state, &mut problems);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join(" "))
    }
}

#[tauri::command]
pub fn integration_status(state: State<'_, AppState>) -> IntegrationStatus {
    IntegrationStatus {
        platform: if cfg!(windows) { "windows" } else { "linux" },
        shell: shell_status(&state),
        autostart: autostart_on(&state),
        has_modern_package: imp::has_modern(),
    }
}

/// Applies everything again: repairs a menu that points at an old folder or a missing entry.
#[tauri::command]
pub fn integration_repair(state: State<'_, AppState>) -> IntegrationStatus {
    apply_shell(&state);
    apply_autostart(&state);
    integration_status(state)
}
