//! The PATH editor. It shows the PATH of the current user and of the whole machine with the problems
//! marked, and changes only the user's own, after saving a copy of what was there. The machine's PATH is
//! shown but never written, since that needs administrator rights.

use crate::state::AppState;
use nomnom_core::path_tools::{self as rules, Entry};
use serde::Serialize;
use tauri::State;

type CmdResult<T> = Result<T, String>;

#[derive(Debug, Serialize)]
pub struct View {
    pub user: Vec<Entry>,
    pub machine: Vec<Entry>,
    /// Saved copies, newest first (file names).
    pub backups: Vec<String>,
    pub supported: bool,
}

#[cfg(windows)]
fn backups_dir(state: &AppState) -> std::path::PathBuf {
    state.paths.data_dir.join("path-backups")
}

#[cfg(windows)]
fn list_backups(state: &AppState) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(backups_dir(state))
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| is_backup_name(n))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names.reverse();
    names
}

/// "user-path-20261120-103000.txt": the only names this ever reads or writes.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_backup_name(n: &str) -> bool {
    let Some(mid) = n
        .strip_prefix("user-path-")
        .and_then(|r| r.strip_suffix(".txt"))
    else {
        return false;
    };
    mid.len() == 15
        && mid
            .char_indices()
            .all(|(i, c)| if i == 8 { c == '-' } else { c.is_ascii_digit() })
}

#[cfg(windows)]
fn view(state: &AppState) -> View {
    let exists = |p: &str| std::path::Path::new(p).is_dir();
    let read = |raw: Option<String>| {
        rules::analyze(&raw.unwrap_or_default(), &nomnom_win::env::expand, &exists)
    };
    View {
        user: read(nomnom_win::env::user_path()),
        machine: read(nomnom_win::env::machine_path()),
        backups: list_backups(state),
        supported: true,
    }
}

#[tauri::command]
pub fn env_get(state: State<'_, AppState>) -> View {
    #[cfg(windows)]
    {
        view(&state)
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        View {
            user: Vec::new(),
            machine: Vec::new(),
            backups: Vec::new(),
            supported: false,
        }
    }
}

#[cfg(windows)]
fn save_backup(state: &AppState) -> CmdResult<()> {
    let dir = backups_dir(state);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Couldn't keep a copy of the old value: {e}"))?;
    let name = format!(
        "user-path-{}.txt",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    std::fs::write(
        dir.join(&name),
        nomnom_win::env::user_path().unwrap_or_default(),
    )
    .map_err(|e| format!("Couldn't keep a copy of the old value: {e}"))?;
    // Only the newest twenty copies are kept.
    for old in list_backups(state).into_iter().skip(20) {
        let _ = std::fs::remove_file(dir.join(old));
    }
    Ok(())
}

/// Saves a new list as the user's PATH, after keeping a copy of the old one.
#[tauri::command]
pub fn env_apply(state: State<'_, AppState>, entries: Vec<String>) -> CmdResult<View> {
    #[cfg(windows)]
    {
        rules::validate(&entries)?;
        save_backup(&state)?;
        nomnom_win::env::set_user_path(&rules::join(&entries))?;
        Ok(view(&state))
    }
    #[cfg(not(windows))]
    {
        let _ = (state, entries);
        Err("The PATH editor works on Windows for now.".into())
    }
}

/// Puts a saved copy back as the user's PATH (the one it replaces is saved first).
#[tauri::command]
pub fn env_restore(state: State<'_, AppState>, name: String) -> CmdResult<View> {
    #[cfg(windows)]
    {
        if !is_backup_name(&name) {
            return Err("That is not a saved copy.".into());
        }
        let text = std::fs::read_to_string(backups_dir(&state).join(&name))
            .map_err(|_| "That copy is gone.".to_string())?;
        let entries = rules::split(&text);
        if !entries.is_empty() {
            rules::validate(&entries)?;
        }
        save_backup(&state)?;
        nomnom_win::env::set_user_path(&text)?;
        Ok(view(&state))
    }
    #[cfg(not(windows))]
    {
        let _ = (state, name);
        Err("The PATH editor works on Windows for now.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_files_this_makes_count_as_saved_copies() {
        assert!(is_backup_name("user-path-20261120-103000.txt"));
        assert!(!is_backup_name("user-path-20261120-10300.txt"));
        assert!(!is_backup_name("user-path-2026112a-103000.txt"));
        assert!(!is_backup_name("../user-path-20261120-103000.txt"));
        assert!(!is_backup_name("user-path-20261120-103000.txt.exe"));
        assert!(!is_backup_name("other.txt"));
    }
}
