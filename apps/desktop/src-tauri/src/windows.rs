use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const MAIN: &str = "main";

/// Shows the main window, creating it if needed. The window is destroyed when closed (not
/// hidden), so a background-only NOMNOM carries no WebView at all.
pub fn open_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let result = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html".into()))
        .title("NOMNOM")
        .inner_size(1040.0, 700.0)
        .min_inner_size(760.0, 520.0)
        .center()
        .build();
    if let Err(e) = result {
        tracing::error!("couldn't create the main window: {e}");
    }
}
