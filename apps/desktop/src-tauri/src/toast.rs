//! Notifications that look like NomNom: a small themed card in the corner of the screen, with an
//! icon, a title, a line of text and a time bar. Clicking one opens the page it is about.
//!
//! The card is its own small window, created when there is something to show and removed a few
//! seconds later, so it costs nothing while idle. Where a desktop doesn't let an app place its
//! windows (Wayland), or when the person prefers it, the system's own notification is used.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_notification::NotificationExt;

const LABEL: &str = "toast";
const WIDTH: f64 = 388.0;
/// Cards on screen at once; more wait their turn.
pub const MAX_SHOWN: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Toast {
    /// "info", "success", "warning" or "error".
    pub kind: String,
    pub title: String,
    pub body: String,
    /// A page to open when the card is clicked: "downloads", "activity", "clipboard"...
    pub action: Option<String>,
}

impl Toast {
    pub fn new(kind: &str, title: impl Into<String>, body: impl Into<String>) -> Self {
        Toast {
            kind: kind.into(),
            title: title.into(),
            body: body.into(),
            action: None,
        }
    }

    pub fn opens(mut self, page: &str) -> Self {
        self.action = Some(page.into());
        self
    }
}

/// Cards waiting for the window to be ready, or for a free place on screen.
static PENDING: Mutex<Vec<Toast>> = Mutex::new(Vec::new());
/// The window is being made right now (a second card in the same moment must not make another).
static MAKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Is the themed card available here, and wanted?
fn pretty(app: &AppHandle) -> bool {
    let wanted = app
        .try_state::<AppState>()
        .is_none_or(|s| s.settings.get().general.pretty_notifications);
    // A Wayland desktop places windows itself, so a card can't sit in the corner.
    let placeable = !(cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some());
    wanted && placeable
}

/// Shows a notification: the themed card where possible, the system's own otherwise.
pub fn show(app: &AppHandle, toast: Toast) {
    crate::notifications::record(app, &toast);
    // While a game is in front the notice waits in the history instead of covering the game.
    if crate::game_mode::holds_back(&toast) {
        return;
    }
    if !pretty(app) {
        let _ = app
            .notification()
            .builder()
            .title(&toast.title)
            .body(&toast.body)
            .show();
        return;
    }
    PENDING.lock().unwrap().push(toast);
    // Windows are made off the main thread: a command that waits for the main thread to build one
    // would wait for itself.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(win) = app.get_webview_window(LABEL) {
            let _ = win.emit("toast://wake", ());
            return;
        }
        // The page takes everything that is waiting once it has loaded.
        if MAKING.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let built =
            WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("panel.html?kind=toast".into()))
                .title("NomNom")
                .inner_size(WIDTH, 120.0)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .focusable(false)
                .focused(false)
                .visible(false);
        let result = crate::windows::shared_webview_args(built).build();
        MAKING.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Err(e) = result {
            tracing::warn!("couldn't make the notification window: {e}");
            // No card could be made: fall back to the system's notification.
            for t in PENDING.lock().unwrap().drain(..) {
                let _ = app
                    .notification()
                    .builder()
                    .title(t.title)
                    .body(t.body)
                    .show();
            }
        }
    });
}

/// The cards the page should show now (at most `MAX_SHOWN`), taken off the waiting list.
#[tauri::command]
pub fn toast_take() -> Vec<Toast> {
    let mut pending = PENDING.lock().unwrap();
    let n = pending.len().min(MAX_SHOWN);
    pending.drain(..n).collect()
}

/// The page reports how tall its cards are; the window follows and sits in the corner.
#[tauri::command]
pub fn toast_layout(app: AppHandle, height: f64) {
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    let scale = win.scale_factor().unwrap_or(1.0);
    let _ = win.set_size(PhysicalSize::new(
        (WIDTH * scale) as u32,
        (height.clamp(40.0, 600.0) * scale) as u32,
    ));
    if let Ok(Some(monitor)) = win.primary_monitor() {
        let area = monitor.work_area();
        let margin = (14.0 * scale) as i32;
        let w = (WIDTH * scale) as i32;
        let h = (height.clamp(40.0, 600.0) * scale) as i32;
        let x = area.position.x + area.size.width as i32 - w - margin;
        let y = area.position.y + area.size.height as i32 - h - margin;
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }
    if !win.is_visible().unwrap_or(false) {
        let _ = win.show();
    }
}

/// A card was clicked: open NomNom on the page it is about.
#[tauri::command]
pub fn toast_open(app: AppHandle, action: String) {
    crate::windows::open_main(&app);
    let _ = app.emit("nav://go", action);
}

/// Shows an example, so the person can see how notifications look (Settings).
#[tauri::command]
pub fn toast_test(app: AppHandle) {
    let lang = crate::i18n::lang_of(&app.state::<AppState>().settings.get());
    let t = |s: &str| crate::i18n::tr(lang, s);
    show(
        &app,
        Toast::new(
            "success",
            t("This is how notifications look"),
            t("They go away by themselves. Click one to open its page."),
        ),
    );
}

#[tauri::command]
pub fn toast_close(app: AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toast_can_name_the_page_it_opens() {
        let t = Toast::new("success", "Done", "All saved").opens("downloads");
        assert_eq!(t.action.as_deref(), Some("downloads"));
        assert_eq!(Toast::new("info", "a", "b").action, None);
    }

    #[test]
    fn only_a_few_cards_are_taken_at_once() {
        PENDING.lock().unwrap().clear();
        for i in 0..5 {
            PENDING
                .lock()
                .unwrap()
                .push(Toast::new("info", format!("t{i}"), ""));
        }
        let first = toast_take();
        assert_eq!(first.len(), MAX_SHOWN);
        assert_eq!(first[0].title, "t0");
        assert_eq!(toast_take().len(), 2, "the rest wait for their turn");
        assert!(toast_take().is_empty());
    }
}
