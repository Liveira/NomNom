//! Global shortcuts: Quick Download, Open NOMNOM, the clipboard history, the translator and the
//! screen-text capture. Registered at startup, re-registered when the user changes one. A tool's
//! shortcut is only taken while the tool is switched on, so nobody's keys are held for nothing.
//! A shortcut another app already owns is reported, never forced.

use crate::state::AppState;
use crate::{quick, windows};
use std::str::FromStr;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const QUICK: &str = "quick_download";
pub const OPEN: &str = "open_app";
pub const CLIPBOARD: &str = "clipboard";
pub const TRANSLATE_REPLACE: &str = "translate_replace";
pub const TRANSLATE_POPUP: &str = "translate_popup";
pub const CAPTURE: &str = "capture_text";
pub const LAUNCHER: &str = "launcher";
pub const NOTES: &str = "notes";
pub const IDS: [&str; 8] = [
    QUICK,
    OPEN,
    CLIPBOARD,
    TRANSLATE_REPLACE,
    TRANSLATE_POPUP,
    CAPTURE,
    LAUNCHER,
    NOTES,
];

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
        CLIPBOARD => crate::clipboard::open_popup(app),
        TRANSLATE_REPLACE => crate::translate::replace_selection(app),
        TRANSLATE_POPUP => crate::translate::popup_selection(app),
        CAPTURE => crate::ocr::capture(app),
        LAUNCHER => crate::launcher::toggle(app),
        NOTES => crate::launcher::go(app, "notes"),
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
    match id {
        QUICK => s.quick_download,
        OPEN => s.open_app,
        TRANSLATE_REPLACE => s.translate_replace,
        TRANSLATE_POPUP => s.translate_popup,
        CAPTURE => s.capture_text,
        LAUNCHER => s.launcher,
        NOTES => s.notes,
        _ => s.clipboard,
    }
}

/// Is a tool in the sidebar? Nobody has chosen yet means all of them are.
fn tool_on(s: &nomnom_core::settings::Settings, id: &str) -> bool {
    s.appearance
        .sidebar_tools
        .as_ref()
        .is_none_or(|t| t.iter().any(|x| x == id))
}

/// Is this shortcut in use? The two basic ones always are; the others only with their tool on.
fn wanted(app: &AppHandle, id: &str) -> bool {
    let s = app.state::<AppState>().settings.get();
    match id {
        CLIPBOARD => s.clipboard.enabled,
        TRANSLATE_REPLACE | TRANSLATE_POPUP => s.text.enabled,
        CAPTURE => s.text.ocr,
        LAUNCHER => s.shortcuts.launcher_on,
        NOTES => tool_on(&s, "notes"),
        _ => true,
    }
}

/// Registers the shortcuts that are in use. Failures are kept for the Settings screen.
pub fn register_all(app: &AppHandle) {
    let state = app.state::<AppState>();
    for id in IDS {
        if !wanted(app, id) || state.shortcuts_registered.lock().unwrap().contains_key(id) {
            continue;
        }
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

/// Matches the registered shortcuts to what is switched on now: a tool that was turned off lets go
/// of its keys, one that was turned on takes them.
pub fn sync(app: &AppHandle) {
    let state = app.state::<AppState>();
    for id in IDS {
        if !wanted(app, id) {
            if let Some(old) = state.shortcuts_registered.lock().unwrap().remove(id) {
                let _ = app.global_shortcut().unregister(old);
            }
            state.shortcut_errors.lock().unwrap().remove(id);
        }
    }
    register_all(app);
}

/// Changes one shortcut. The old one stays active if the new one can't be registered.
pub fn change(app: &AppHandle, id: &str, accel: &str) -> Result<(), String> {
    let id: &'static str = IDS
        .into_iter()
        .find(|i| *i == id)
        .ok_or("Unknown shortcut.")?;
    let new = parse_accelerator(accel)?;
    if IDS
        .into_iter()
        .filter(|other| *other != id)
        .any(|other| parse_accelerator(&configured(app, other)).is_ok_and(|o| o == new))
    {
        return Err("That's already used by another NomNom shortcut.".into());
    }
    if custom_uses(app, &new, None) {
        return Err("One of your own actions already uses that.".into());
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
    if !wanted(app, id) {
        // Its tool is off: only remember the keys.
        return persist(app, id, accel);
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
        .update("shortcuts", |s| match id {
            QUICK => s.shortcuts.quick_download = accel.to_string(),
            OPEN => s.shortcuts.open_app = accel.to_string(),
            TRANSLATE_REPLACE => s.shortcuts.translate_replace = accel.to_string(),
            TRANSLATE_POPUP => s.shortcuts.translate_popup = accel.to_string(),
            CAPTURE => s.shortcuts.capture_text = accel.to_string(),
            LAUNCHER => s.shortcuts.launcher = accel.to_string(),
            NOTES => s.shortcuts.notes = accel.to_string(),
            _ => s.shortcuts.clipboard = accel.to_string(),
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Does one of the person's own actions (other than `except`) already use these keys?
pub fn custom_uses(app: &AppHandle, keys: &Shortcut, except: Option<&str>) -> bool {
    app.state::<AppState>()
        .settings
        .get()
        .actions
        .iter()
        .filter(|a| Some(a.id.as_str()) != except)
        .filter_map(|a| a.shortcut.as_deref())
        .any(|k| parse_accelerator(k).is_ok_and(|p| p == *keys))
}

/// Does this accelerator belong to one of NomNom's own shortcuts?
pub fn builtin_uses(app: &AppHandle, keys: &Shortcut) -> bool {
    IDS.into_iter()
        .any(|id| parse_accelerator(&configured(app, id)).is_ok_and(|o| o == *keys))
}

/// Takes the keys of every action that has a shortcut and lets go of the rest. Run after the list
/// changes and at start. A key another app owns is kept as a note beside the action.
pub fn sync_custom(app: &AppHandle) {
    let state = app.state::<AppState>();
    let actions = state.settings.get().actions;
    let old: Vec<_> = state.custom_shortcuts.lock().unwrap().drain().collect();
    for (_, s) in old {
        let _ = app.global_shortcut().unregister(s);
    }
    state.custom_errors.lock().unwrap().clear();
    for a in actions {
        let Some(accel) = a.shortcut.clone().filter(|k| !k.trim().is_empty()) else {
            continue;
        };
        let id = a.id.clone();
        let result = parse_accelerator(&accel).and_then(|sc| {
            if builtin_uses(app, &sc) {
                return Err("That's already used by a NomNom shortcut.".to_string());
            }
            let run_id = id.clone();
            app.global_shortcut()
                .on_shortcut(sc, move |app, _s, event| {
                    if event.state() == ShortcutState::Pressed {
                        crate::actions::run_in_background(app, &run_id);
                    }
                })
                .map_err(|_| {
                    format!("{accel} is used by another app. Pick a different shortcut.")
                })?;
            Ok(sc)
        });
        match result {
            Ok(sc) => {
                state.custom_shortcuts.lock().unwrap().insert(id, sc);
            }
            Err(e) => {
                tracing::warn!(%id, "action shortcut not registered: {e}");
                state.custom_errors.lock().unwrap().insert(id, e);
            }
        }
    }
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
