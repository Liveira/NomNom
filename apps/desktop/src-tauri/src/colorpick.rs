//! Picking a colour from the screen. Press the shortcut, the screen freezes under a magnifier, click
//! the colour you want, and its code is on the clipboard. Like reading text from the screen, the
//! picture is kept in memory only until the click, and is never written anywhere.

use crate::i18n::{lang_of, tr};
use crate::state::AppState;
use crate::toast::{self, Toast};
use tauri::{AppHandle, Manager};

/// A colour code the page may send back: `#` and six hex digits.
#[cfg_attr(not(windows), allow(dead_code))]
fn valid_hex(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(not(windows))]
pub fn start(app: &AppHandle) {
    let lang = lang_of(&app.state::<AppState>().settings.get());
    toast::show(
        app,
        Toast::new(
            "info",
            "NomNom",
            tr(lang, "Picking a colour works on Windows only."),
        ),
    );
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::windows::shared_webview_args;
    use std::sync::Mutex;
    use tauri::{PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
    use tauri_plugin_clipboard_manager::ClipboardExt;

    const LABEL: &str = "picker";

    struct Held {
        width: u32,
        height: u32,
        bgra: Vec<u8>,
    }
    static HELD: Mutex<Option<Held>> = Mutex::new(None);

    pub fn start(app: &AppHandle) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if app.get_webview_window(LABEL).is_some() {
                return;
            }
            let a = app.clone();
            let shot = tokio::task::spawn_blocking(move || crate::ocr::screen_under_pointer(&a))
                .await
                .ok()
                .flatten();
            let lang = lang_of(&app.state::<AppState>().settings.get());
            let Some((x, y, shot)) = shot else {
                toast::show(
                    &app,
                    Toast::new(
                        "error",
                        "NomNom",
                        tr(
                            lang,
                            "Windows didn't let NomNom take a picture of the screen.",
                        ),
                    ),
                );
                return;
            };
            let (w, h) = (shot.width, shot.height);
            *HELD.lock().unwrap() = Some(Held {
                width: w,
                height: h,
                bgra: shot.bgra,
            });
            let built = WebviewWindowBuilder::new(
                &app,
                LABEL,
                WebviewUrl::App("panel.html?kind=picker".into()),
            )
            .title("NomNom")
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .visible(false);
            match shared_webview_args(built).build() {
                Ok(win) => {
                    let _ = win.set_position(PhysicalPosition::new(x, y));
                    let _ = win.set_size(PhysicalSize::new(w, h));
                }
                Err(e) => {
                    tracing::warn!("couldn't open the colour picker: {e}");
                    *HELD.lock().unwrap() = None;
                }
            }
        });
    }

    pub fn image() -> Option<String> {
        let held = HELD.lock().unwrap();
        let h = held.as_ref()?;
        let png = crate::ocr::png_of(h.width, h.height, &h.bgra)?;
        Some(nomnom_activity::encode_icon_data_url(&png))
    }

    pub fn ready(app: &AppHandle) {
        if let Some(win) = app.get_webview_window(LABEL) {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }

    pub fn cancel(app: &AppHandle) {
        *HELD.lock().unwrap() = None;
        if let Some(win) = app.get_webview_window(LABEL) {
            let _ = win.destroy();
        }
    }

    pub fn done(app: &AppHandle, hex: &str) {
        cancel(app);
        if !valid_hex(hex) {
            return;
        }
        crate::clipboard::suppress(1500);
        if app.clipboard().write_text(hex.to_string()).is_ok() {
            let lang = lang_of(&app.state::<AppState>().settings.get());
            toast::show(
                app,
                Toast::new("success", tr(lang, "Colour copied"), hex.to_string()).opens("mini"),
            );
        }
    }
}

#[cfg(windows)]
pub use imp::start;

/// The page asks for the picture, draws it, then asks to be shown.
#[tauri::command]
pub fn pk_image() -> Option<String> {
    #[cfg(windows)]
    {
        imp::image()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[tauri::command]
pub fn pk_ready(app: AppHandle) {
    #[cfg(windows)]
    imp::ready(&app);
    #[cfg(not(windows))]
    let _ = app;
}

#[tauri::command]
pub fn pk_cancel(app: AppHandle) {
    #[cfg(windows)]
    imp::cancel(&app);
    #[cfg(not(windows))]
    let _ = app;
}

/// The colour that was clicked, as `#rrggbb`. It goes on the clipboard.
#[tauri::command]
pub fn pk_done(app: AppHandle, hex: String) {
    #[cfg(windows)]
    imp::done(&app, &hex);
    #[cfg(not(windows))]
    let _ = (app, hex);
}

/// The same as the shortcut, for the button on the page.
#[tauri::command]
pub fn mini_pick(app: AppHandle) {
    start(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_hash_and_six_hex_digits_is_a_colour_code() {
        assert!(valid_hex("#ffa970"));
        assert!(valid_hex("#FFA970"));
        assert!(!valid_hex("ffa970"));
        assert!(!valid_hex("#ffa97"));
        assert!(!valid_hex("#ffa97g"));
        assert!(!valid_hex("#ffa970; calc"));
    }
}
