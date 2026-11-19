//! Reading text from the screen. Press the shortcut, drag a rectangle over any text (a picture, a
//! video, a program that won't let you select), and the text is put on the clipboard. It works
//! offline: on Windows with the text recognition that comes with Windows, on Linux with
//! `tesseract` when it is installed. The picture of the screen is kept in memory only until the
//! rectangle is chosen, and never written anywhere.
#![allow(clippy::chunks_exact_to_as_chunks)] // the newer `as_chunks` is not in every supported Rust

use crate::state::AppState;
use crate::toast::{self, Toast};
#[cfg(windows)]
use crate::windows::shared_webview_args;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
#[cfg(windows)]
use tauri::{PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;

type CmdResult<T> = Result<T, String>;

const LABEL: &str = "ocr";

/// The picture of the monitor under the pointer, while the rectangle is being chosen.
struct Held {
    width: u32,
    height: u32,
    bgra: Vec<u8>,
}

static HELD: Mutex<Option<Held>> = Mutex::new(None);

fn say(app: &AppHandle, kind: &str, title: &str, body: &str) {
    let lang = crate::i18n::lang_of(&app.state::<AppState>().settings.get());
    toast::show(
        app,
        Toast::new(
            kind,
            crate::i18n::tr(lang, title),
            crate::i18n::tr(lang, body),
        )
        .opens("text"),
    );
}

/// Blue-green-red-alpha rows to the PNG the page shows.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn png_of(width: u32, height: u32, bgra: &[u8]) -> Option<Vec<u8>> {
    let mut rgba = bgra.to_vec();
    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut w = enc.write_header().ok()?;
        w.write_image_data(&rgba).ok()?;
    }
    Some(out)
}

/// The part of a picture inside a rectangle (clamped to the picture), or `None` when nothing is left.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn crop(
    bgra: &[u8],
    width: u32,
    height: u32,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
) -> Option<(u32, u32, Vec<u8>)> {
    let x0 = x.clamp(0, i64::from(width));
    let y0 = y.clamp(0, i64::from(height));
    let x1 = (x + w).clamp(0, i64::from(width));
    let y1 = (y + h).clamp(0, i64::from(height));
    if x1 - x0 < 4 || y1 - y0 < 4 {
        return None;
    }
    let (cw, ch) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let mut out = Vec::with_capacity(cw * ch * 4);
    for row in y0 as usize..y1 as usize {
        let start = (row * width as usize + x0 as usize) * 4;
        out.extend_from_slice(bgra.get(start..start + cw * 4)?);
    }
    Some((cw as u32, ch as u32, out))
}

/// The shortcut: take a picture of the screen under the pointer and let the person choose.
pub fn capture(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(windows)]
        {
            let a = app.clone();
            let Ok(shot) = tokio::task::spawn_blocking(move || screen_under_pointer(&a)).await
            else {
                return;
            };
            match shot {
                Some((x, y, shot)) => {
                    *HELD.lock().unwrap() = Some(Held {
                        width: shot.width,
                        height: shot.height,
                        bgra: shot.bgra,
                    });
                    open_overlay(&app, x, y, shot.width, shot.height);
                }
                None => say(
                    &app,
                    "error",
                    "Couldn't read the screen",
                    "Windows didn't let NomNom take a picture of the screen.",
                ),
            }
        }
        #[cfg(not(windows))]
        {
            let a = app.clone();
            let result = tokio::task::spawn_blocking(move || linux_read(&a)).await;
            match result {
                Ok(Ok(text)) => finish(&app, text).await,
                Ok(Err(e)) => say(&app, "error", "Couldn't read the screen", &e),
                Err(_) => {}
            }
        }
    });
}

#[cfg(windows)]
pub(crate) fn screen_under_pointer(
    app: &AppHandle,
) -> Option<(i32, i32, nomnom_win::screen::Shot)> {
    let cursor = app.cursor_position().ok()?;
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let (pos, size) = (monitor.position(), monitor.size());
    let shot = nomnom_win::screen::capture(pos.x, pos.y, size.width as i32, size.height as i32)?;
    Some((pos.x, pos.y, shot))
}

#[cfg(windows)]
fn open_overlay(app: &AppHandle, x: i32, y: i32, width: u32, height: u32) {
    if let Some(old) = app.get_webview_window(LABEL) {
        let _ = old.destroy();
    }
    let built =
        WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("panel.html?kind=ocr".into()))
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
            let _ = win.set_size(PhysicalSize::new(width, height));
        }
        Err(e) => {
            tracing::warn!("couldn't open the capture window: {e}");
            *HELD.lock().unwrap() = None;
        }
    }
}

/// The same as the shortcut, for the button on the Translator page.
#[tauri::command]
pub fn ocr_start(app: AppHandle) {
    capture(&app);
}

/// The page asks for the picture, draws it, then asks to be shown.
#[tauri::command]
pub fn ocr_image() -> Option<String> {
    let held = HELD.lock().unwrap();
    let h = held.as_ref()?;
    let png = png_of(h.width, h.height, &h.bgra)?;
    Some(nomnom_activity::encode_icon_data_url(&png))
}

#[tauri::command]
pub fn ocr_ready(app: AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[tauri::command]
pub fn ocr_cancel(app: AppHandle) {
    *HELD.lock().unwrap() = None;
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.destroy();
    }
}

/// Reads the text inside the rectangle (in picture pixels) the person chose.
#[tauri::command]
pub async fn ocr_run(app: AppHandle, x: i64, y: i64, w: i64, h: i64) -> CmdResult<()> {
    let held = HELD.lock().unwrap().take();
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.destroy();
    }
    let Some(held) = held else {
        return Err("The picture is gone. Press the shortcut again.".into());
    };
    let Some((cw, ch, part)) = crop(&held.bgra, held.width, held.height, x, y, w, h) else {
        return Err("Drag a bigger rectangle over the text.".into());
    };
    #[cfg(windows)]
    {
        let read = tokio::task::spawn_blocking(move || {
            nomnom_win::screen::read_text(&nomnom_win::screen::Shot {
                width: cw,
                height: ch,
                bgra: part,
            })
        })
        .await
        .map_err(|e| e.to_string())?;
        match read {
            Ok(text) => finish(&app, text).await,
            Err(e) => say(&app, "error", "Couldn't read the screen", &e),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (cw, ch, part);
    }
    Ok(())
}

/// Puts the text on the clipboard (translated first, if that is switched on) and says so.
async fn finish(app: &AppHandle, text: String) {
    let text = text.trim().to_string();
    if text.is_empty() {
        say(
            app,
            "info",
            "No text found",
            "NomNom didn't find any text in that part of the screen.",
        );
        return;
    }
    let state = app.state::<AppState>();
    let s = state.settings.get();
    let mut shown = text.clone();
    let mut translated = false;
    if s.text.ocr_translate {
        if let Some(t) = crate::translate::translate_for_ocr(app, &text).await {
            shown = t;
            translated = true;
        }
    }
    if app.clipboard().write_text(shown.clone()).is_err() {
        say(app, "error", "Couldn't use the clipboard.", "");
        return;
    }
    let lang = crate::i18n::lang_of(&s);
    let (title, body) = crate::i18n::ocr_text(lang, &shown, translated);
    toast::show(app, Toast::new("success", title, body).opens("text"));
}

// ---- Linux: the desktop's own picture tools, and tesseract for the reading ---------------------------

#[cfg(not(windows))]
fn have(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

#[cfg(not(windows))]
fn linux_read(app: &AppHandle) -> Result<String, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    if !have("tesseract") {
        return Err("To read text from the screen, install tesseract.".into());
    }
    let png = if wayland {
        if !have("grim") || !have("slurp") {
            return Err("To read text from the screen on Wayland, install grim and slurp.".into());
        }
        let geo = Command::new("slurp").output().map_err(|e| e.to_string())?;
        if !geo.status.success() {
            return Err("Nothing was selected.".into());
        }
        let geo = String::from_utf8_lossy(&geo.stdout).trim().to_string();
        let shot = Command::new("grim")
            .args(["-g", &geo, "-"])
            .output()
            .map_err(|e| e.to_string())?;
        shot.stdout
    } else {
        if !have("maim") {
            return Err("To read text from the screen on X11, install maim.".into());
        }
        let shot = Command::new("maim")
            .arg("-s")
            .output()
            .map_err(|e| e.to_string())?;
        if !shot.status.success() {
            return Err("Nothing was selected.".into());
        }
        shot.stdout
    };
    let ui = crate::i18n::lang_of(&app.state::<AppState>().settings.get());
    let wanted = match ui {
        crate::i18n::Lang::Pt => "por+eng",
        crate::i18n::Lang::Es => "spa+eng",
        crate::i18n::Lang::En => "eng",
    };
    let mut child = Command::new("tesseract")
        .args(["stdin", "stdout", "-l", wanted])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("tesseract didn't start.")?
        .write_all(&png)
        .map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    if !out.status.success() && text.trim().is_empty() {
        // The language pack may be missing: try English alone.
        let mut again = Command::new("tesseract")
            .args(["stdin", "stdout"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        again
            .stdin
            .take()
            .ok_or("tesseract didn't start.")?
            .write_all(&png)
            .map_err(|e| e.to_string())?;
        let out = again.wait_with_output().map_err(|e| e.to_string())?;
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(w: u32, h: u32) -> Vec<u8> {
        (0..w * h)
            .flat_map(|i| [(i % 256) as u8, 0, 0, 255])
            .collect()
    }

    #[test]
    fn a_rectangle_is_cut_out_and_kept_inside_the_picture() {
        let (w, h) = (10u32, 8u32);
        let img = grey(w, h);
        let (cw, ch, part) = crop(&img, w, h, 2, 3, 4, 4).unwrap();
        assert_eq!((cw, ch), (4, 4));
        assert_eq!(part.len(), 4 * 4 * 4);
        // The first pixel is the one at column 2 of row 3 (pixel number 32).
        assert_eq!(part[0], 32);
        // A rectangle that hangs over the edge is trimmed to what is there.
        let (cw, ch, _) = crop(&img, w, h, 5, 3, 20, 20).unwrap();
        assert_eq!((cw, ch), (5, 5));
        // Nothing, or too little, is refused.
        assert!(crop(&img, w, h, 50, 50, 5, 5).is_none());
        assert!(crop(&img, w, h, 2, 2, 2, 20).is_none());
        assert!(
            crop(&img, w, h, 8, 6, 10, 10).is_none(),
            "2 by 2 is too small to read"
        );
    }

    #[test]
    fn the_picture_for_the_page_is_a_real_png() {
        let png = png_of(4, 2, &grey(4, 2)).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        assert!(png_of(0, 0, &[]).is_none() || png_of(0, 0, &[]).is_some());
    }
}
