//! The quick-download popup: a small frameless window opened by the global shortcut.
//!
//! Creating a WebView2 window from nothing costs about a second on a typical PC, because with no
//! window open there is no WebView process at all. So there are two modes:
//! - default: create the window when the shortcut is pressed (slower, uses no memory when idle);
//! - "keep ready": keep one hidden, already-loaded window and just show it (instant, costs memory).

use crate::modules::downloads::analyze_cached;
use crate::state::AppState;
use nomnom_downloads::url_check::{check_clipboard, site_label};
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_notification::NotificationExt;

pub const LABEL: &str = "quick";
const WIDTH: f64 = 480.0;
const HEIGHT: f64 = 360.0;
/// Ignore focus loss right after opening; the window is still receiving focus then.
const BLUR_GRACE: Duration = Duration::from_millis(500);

#[derive(Serialize, Clone)]
pub struct QuickContext {
    url: String,
    site: String,
}

fn keep_ready(app: &AppHandle) -> bool {
    app.state::<AppState>().settings.get().shortcuts.keep_ready
}

/// Called by the global shortcut. Reads the clipboard once, right now, and nothing else.
pub fn trigger(app: &AppHandle) {
    let pressed = Instant::now();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let text = app.clipboard().read_text().unwrap_or_default();
        match check_clipboard(&text) {
            Ok(url) => open(&app, url.to_string(), pressed),
            Err(reason) => {
                tracing::info!(?reason, "quick download ignored");
                let _ = app
                    .notification()
                    .builder()
                    .title("No supported link on the clipboard")
                    .body(reason.message())
                    .show();
            }
        }
    });
}

fn open(app: &AppHandle, url: String, pressed: Instant) {
    let state = app.state::<AppState>();
    let site = nomnom_downloads::url_check::validate_url(&url)
        .map(|u| site_label(&u))
        .unwrap_or_default();
    *state.quick_shown.lock().unwrap() = None;
    *state.quick.lock().unwrap() = Some((
        QuickContext {
            url: url.clone(),
            site,
        },
        pressed,
    ));

    // Start the lookup now so the popup's own request finds it in the cache.
    let warm = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = analyze_cached(&warm.state::<AppState>(), &url).await;
    });

    if let Some(win) = app.get_webview_window(LABEL) {
        // Already created (visible, or hidden and waiting in keep-ready mode): hand it the new
        // link and show it. The page resets itself when it receives the context.
        let _ = app.emit_to(LABEL, "quick://context", ());
        present(app, &win);
    } else {
        // The page calls `quick_ready` after its first paint, which shows the window.
        build_window(app);
    }
}

fn build_window(app: &AppHandle) -> Option<WebviewWindow> {
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("quick.html".into()))
        .title("NomNom quick download")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false);
    match crate::windows::shared_webview_args(builder).build() {
        Ok(win) => {
            let watched = win.clone();
            win.on_window_event(move |event| {
                if let WindowEvent::Focused(false) = event {
                    let app = watched.app_handle();
                    let opened = app
                        .state::<AppState>()
                        .quick_shown
                        .lock()
                        .unwrap()
                        .map(|t| t.elapsed());
                    if opened.is_some_and(|e| e > BLUR_GRACE) {
                        dismiss(app);
                    }
                }
            });
            Some(win)
        }
        Err(e) => {
            tracing::error!("couldn't create the quick popup: {e}");
            None
        }
    }
}

/// Hides the popup (keep-ready mode) or destroys it, freeing its memory.
fn dismiss(app: &AppHandle) {
    *app.state::<AppState>().quick_shown.lock().unwrap() = None;
    if let Some(win) = app.get_webview_window(LABEL) {
        if keep_ready(app) {
            let _ = win.hide();
        } else {
            let _ = win.destroy();
        }
    }
}

/// Creates or removes the hidden, pre-loaded popup to match the setting.
pub fn apply_keep_ready(app: &AppHandle) {
    let existing = app.get_webview_window(LABEL);
    if keep_ready(app) {
        if existing.is_none() {
            build_window(app);
        }
    } else if let Some(win) = existing {
        if !win.is_visible().unwrap_or(false) {
            let _ = win.destroy();
        }
    }
}

fn center_on_cursor_monitor(app: &AppHandle, win: &WebviewWindow) {
    let Ok(cursor) = app.cursor_position() else {
        return;
    };
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let (Some(m), Ok(size)) = (monitor, win.outer_size()) else {
        return;
    };
    let x = m.position().x + (m.size().width as i32 - size.width as i32) / 2;
    // A little above the middle feels more natural for a palette.
    let y = m.position().y + (m.size().height as i32 - size.height as i32) / 3;
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

/// Places, shows and focuses the popup, and logs how long the whole thing took.
fn present(app: &AppHandle, win: &WebviewWindow) {
    let state = app.state::<AppState>();
    center_on_cursor_monitor(app, win);
    let _ = win.show();
    let _ = win.set_focus();
    *state.quick_shown.lock().unwrap() = Some(Instant::now());
    let pressed = state.quick.lock().unwrap().as_ref().map(|(_, p)| *p);
    if let Some(pressed) = pressed {
        tracing::info!(
            ms = pressed.elapsed().as_millis() as u64,
            keep_ready = keep_ready(app),
            "quick popup visible"
        );
    }
}

#[tauri::command]
pub fn quick_context(state: State<'_, AppState>) -> Option<QuickContext> {
    state.quick.lock().unwrap().as_ref().map(|(c, _)| c.clone())
}

/// The page calls this after its first paint of a new link.
#[tauri::command]
pub fn quick_ready(app: AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        present(&app, &win);
    }
}

#[tauri::command]
pub fn quick_close(app: AppHandle) {
    dismiss(&app);
}
