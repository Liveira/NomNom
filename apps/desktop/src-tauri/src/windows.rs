use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const MAIN: &str = "main";

/// Debug builds only: lets automated UI checks attach to the WebView on a local port.
/// Compiled out of release builds, so shipped NOMNOM never opens a debugging port.
///
/// Every window must get the same arguments: WebView2 refuses to create a second window in a
/// shared profile with different browser arguments.
#[cfg(debug_assertions)]
pub fn shared_webview_args<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    match std::env::var("NOMNOM_DEBUG_PORT") {
        Ok(port) if port.chars().all(|c| c.is_ascii_digit()) => builder.additional_browser_args(&format!(
            "--remote-debugging-port={port} --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"
        )),
        _ => builder,
    }
}

#[cfg(not(debug_assertions))]
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
        .center();

    if let Err(e) = shared_webview_args(builder).build() {
        tracing::error!("couldn't create the main window: {e}");
    }
}
