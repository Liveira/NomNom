//! Global shortcuts: Quick Download and Open NOMNOM. Registered at startup, re-registered when
//! the user changes one. A shortcut another app already owns is reported, never forced.

use crate::state::AppState;
use crate::{quick, windows};
use std::str::FromStr;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const QUICK: &str = "quick_download";
pub const OPEN: &str = "open_app";
const IDS: [&str; 2] = [QUICK, OPEN];

/// Parses an accelerator such as `Ctrl+Alt+D`. Requires at least one modifier so a plain letter
/// can never be hijacked system-wide.
pub fn parse_accelerator(text: &str) -> Result<Shortcut, String> {
    let shortcut = Shortcut::from_str(text.trim())
        .map_err(|_| "That isn't a valid shortcut. Try something like Ctrl+Alt+D.".to_string())?;
    if shortcut.mods.is_empty() {
        return Err("Use at least one of Ctrl, Alt, Shift or Win with a key.".into());
    }
    Ok(shortcut)
}

fn action(app: &AppHandle, id: &str) {
    match id {
        QUICK => quick::trigger(app),
        OPEN => windows::open_main(app),
        _ => {}
    }
}

fn register(app: &AppHandle, id: &'static str, accel: &str) -> Result<Shortcut, String> {
    let shortcut = parse_accelerator(accel)?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _s, event| {
            if event.state() == ShortcutState::Pressed {
                action(app, id);
            }
        })
        .map_err(|_| format!("{accel} is used by another app. Pick a different shortcut."))?;
    Ok(shortcut)
}

fn configured(app: &AppHandle, id: &str) -> String {
    let s = app.state::<AppState>().settings.get().shortcuts;
    if id == QUICK {
        s.quick_download
    } else {
        s.open_app
    }
}

/// Registers both shortcuts from settings. Failures are kept for the Settings screen.
pub fn register_all(app: &AppHandle) {
    let state = app.state::<AppState>();
    for id in IDS {
        let accel = configured(app, id);
        match register(app, id, &accel) {
            Ok(s) => {
                state.shortcuts_registered.lock().unwrap().insert(id, s);
                state.shortcut_errors.lock().unwrap().remove(id);
            }
            Err(e) => {
                tracing::warn!(%id, %accel, "shortcut not registered: {e}");
                state.shortcut_errors.lock().unwrap().insert(id, e);
            }
        }
    }
}

/// Changes one shortcut. The old one stays active if the new one can't be registered.
pub fn change(app: &AppHandle, id: &str, accel: &str) -> Result<(), String> {
    let id: &'static str = IDS
        .into_iter()
        .find(|i| *i == id)
        .ok_or("Unknown shortcut.")?;
    let new = parse_accelerator(accel)?;
    let other = IDS.into_iter().find(|i| *i != id).unwrap_or(QUICK);
    if parse_accelerator(&configured(app, other)).is_ok_and(|o| o == new) {
        return Err("That's already used by another NOMNOM shortcut.".into());
    }

    let state = app.state::<AppState>();
    let old = state.shortcuts_registered.lock().unwrap().remove(id);
    if let Some(old) = old {
        if old == new {
            state.shortcuts_registered.lock().unwrap().insert(id, old);
            return persist(app, id, accel);
        }
        let _ = app.global_shortcut().unregister(old);
    }
    match register(app, id, accel) {
        Ok(s) => {
            state.shortcuts_registered.lock().unwrap().insert(id, s);
            state.shortcut_errors.lock().unwrap().remove(id);
            persist(app, id, accel)
        }
        Err(e) => {
            // Put the previous shortcut back so the user isn't left without one.
            let previous = configured(app, id);
            if let Ok(s) = register(app, id, &previous) {
                state.shortcuts_registered.lock().unwrap().insert(id, s);
            }
            Err(e)
        }
    }
}

fn persist(app: &AppHandle, id: &str, accel: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    state
        .settings
        .update("shortcuts", |s| {
            if id == QUICK {
                s.shortcuts.quick_download = accel.to_string();
            } else {
                s.shortcuts.open_app = accel.to_string();
            }
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_modifier_plus_key() {
        assert!(parse_accelerator("Ctrl+Alt+D").is_ok());
        assert!(parse_accelerator(" ctrl+shift+F5 ").is_ok());
        assert!(parse_accelerator("Alt+Space").is_ok());
    }

    /// Every key name the Settings recorder can produce must parse here.
    #[test]
    fn accepts_everything_the_recorder_emits() {
        for accel in [
            "Ctrl+Shift+5",
            "Alt+F5",
            "Alt+F24",
            "Ctrl+ArrowUp",
            "Ctrl+ArrowLeft",
            "Super+W",
            "Ctrl+Alt+Space",
            "Ctrl+Alt+Enter",
            "Ctrl+Alt+Tab",
            "Ctrl+Alt+Backspace",
            "Ctrl+Alt+Delete",
            "Ctrl+Insert",
            "Ctrl+Home",
            "Ctrl+End",
            "Ctrl+PageUp",
            "Ctrl+PageDown",
        ] {
            assert!(parse_accelerator(accel).is_ok(), "{accel} should parse");
        }
    }

    #[test]
    fn rejects_bare_keys_and_nonsense() {
        assert!(parse_accelerator("D").is_err());
        assert!(parse_accelerator("F5").is_err());
        assert!(parse_accelerator("").is_err());
        assert!(parse_accelerator("Ctrl+").is_err());
        assert!(parse_accelerator("Ctrl+Alt+Banana").is_err());
    }

    #[test]
    fn same_keys_in_any_spelling_are_equal() {
        assert_eq!(
            parse_accelerator("Ctrl+Alt+D").unwrap(),
            parse_accelerator("alt+control+d").unwrap()
        );
    }
}
