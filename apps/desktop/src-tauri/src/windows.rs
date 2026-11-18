use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const MAIN: &str = "main";

/// Debug builds on Windows only: lets automated UI checks attach to WebView2 on a local port.
/// Compiled out of release builds, so shipped NOMNOM never opens a debugging port.
#[cfg(all(debug_assertions, windows))]
pub fn shared_webview_args<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    // `NOMNOM_WEBVIEW_ARGS` lets a test try extra browser flags (memory experiments).
    let port = std::env::var("NOMNOM_DEBUG_PORT")
        .ok()
        .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    let extra = std::env::var("NOMNOM_WEBVIEW_ARGS").unwrap_or_default();
    if port.is_none() && extra.is_empty() {
        return builder;
    }
    let mut args = Vec::new();
    if let Some(port) = port {
        args.push(format!("--remote-debugging-port={port}"));
    }
    if !extra.contains("--disable-features=") {
        args.push("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection".to_string());
    }
    if !extra.is_empty() {
        args.push(extra);
    }
    builder.additional_browser_args(&args.join(" "))
}

#[cfg(not(all(debug_assertions, windows)))]
pub fn shared_webview_args<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    builder
}

/// Shows the main window, creating it if needed. The window is destroyed when closed (not
/// hidden), so a background-only NOMNOM carries no WebView at all.
pub fn open_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let builder = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html".into()))
        .title("NomNom")
        .inner_size(1040.0, 700.0)
        .min_inner_size(760.0, 520.0)
        // NomNom draws its own title bar (see TitleBar.svelte).
        .decorations(false)
        .center();

    if let Err(e) = shared_webview_args(builder).build() {
        tracing::error!("couldn't create the main window: {e}");
    }
}
