//! The small everyday tools: keep a window on top, paste without formatting, and clean the
//! tracking off a link. Each one is a shortcut that does one thing and says what it did.

use crate::i18n::{lang_of, tr};
use crate::state::AppState;
use crate::toast::{self, Toast};
use nomnom_core::links;
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

fn say(app: &AppHandle, kind: &str, title: &str, body: &str) {
    let lang = lang_of(&app.state::<AppState>().settings.get());
    toast::show(
        app,
        Toast::new(kind, tr(lang, title), tr(lang, body)).opens("mini"),
    );
}

/// Keeps the window that has the keyboard above the others, or lets it go back.
pub fn pin_top(app: &AppHandle) {
    #[cfg(windows)]
    {
        let app = app.clone();
        // The shortcut's keys are still down; the window call itself is instant.
        std::thread::spawn(move || match nomnom_win::window::toggle_topmost() {
            Some(p) => {
                let title: String = p.title.chars().take(80).collect();
                let lang = lang_of(&app.state::<AppState>().settings.get());
                let head = if p.on_top {
                    tr(lang, "Pinned on top")
                } else {
                    tr(lang, "Not on top any more")
                };
                toast::show(&app, Toast::new("info", head, title).opens("mini"));
            }
            None => say(&app, "info", "NomNom", "There is no window to pin."),
        });
    }
    #[cfg(not(windows))]
    say(
        app,
        "info",
        "NomNom",
        "Pinning windows works on Windows only.",
    );
}

/// Takes the tracking off the link on the clipboard.
pub fn clean_clipboard(app: &AppHandle) {
    let text = match app.clipboard().read_text() {
        Ok(t) if !t.trim().is_empty() => t,
        _ => return say(app, "info", "NomNom", "The clipboard has no text."),
    };
    match links::clean(&text) {
        Some(clean) => {
            crate::clipboard::suppress(1500);
            if app.clipboard().write_text(clean.clone()).is_ok() {
                say(app, "success", "Link cleaned", &clean);
            }
        }
        None => say(
            app,
            "info",
            "Link cleaned",
            "There was no tracking in the link on the clipboard.",
        ),
    }
}

/// Pastes what is on the clipboard with no formatting: only its text.
#[cfg(windows)]
pub fn paste_plain(app: &AppHandle) {
    let text = match app.clipboard().read_text() {
        Ok(t) if !t.is_empty() => t,
        _ => return say(app, "info", "NomNom", "The clipboard has no text."),
    };
    crate::clipboard::suppress(2500);
    if app.clipboard().write_text(text).is_err() {
        return;
    }
    std::thread::spawn(|| {
        // Ctrl+Alt+Shift+V is still held: wait for it to be let go, or the paste reads as that.
        nomnom_win::clipboard::wait_modifiers_released(1500);
        nomnom_win::clipboard::press_paste();
    });
}

/// Plain paste is a Windows tool; its shortcut is never taken elsewhere.
#[cfg(not(windows))]
pub fn paste_plain(_app: &AppHandle) {}

/// Windows: takes the tracking off links as they are copied, when that is switched on.
#[cfg(windows)]
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last = nomnom_win::clipboard::sequence();
        loop {
            if !app.state::<AppState>().settings.get().mini.clean_links_auto {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                last = nomnom_win::clipboard::sequence();
                continue;
            }
            tokio::time::sleep(std::time::Duration::from_millis(700)).await;
            let seq = nomnom_win::clipboard::sequence();
            if seq == last {
                continue;
            }
            last = seq;
            if nomnom_win::clipboard::private_copy() {
                continue;
            }
            let Ok(text) = app.clipboard().read_text() else {
                continue;
            };
            if let Some(clean) = links::clean(&text) {
                crate::clipboard::suppress(1500);
                if app.clipboard().write_text(clean.clone()).is_ok() {
                    last = nomnom_win::clipboard::sequence();
                    say(&app, "success", "Link cleaned", &clean);
                }
            }
        }
    });
}

/// The button on the page: the same as the shortcut.
#[tauri::command]
pub fn mini_clean_now(app: AppHandle) {
    clean_clipboard(&app);
}
