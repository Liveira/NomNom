//! Screenshots you can mark up. Press the shortcut, drag a rectangle on the frozen screen, and the
//! picture opens in a small editor: arrows, boxes, a marker, blurred parts for what must not be seen,
//! numbered steps and text. Then copy it or save it. The picture is kept in memory only, and is
//! written to disk only when you choose to save it.

#![allow(clippy::chunks_exact_to_as_chunks)] // the newer `as_chunks` is not in every supported Rust

use crate::i18n::{lang_of, tr};
use crate::state::AppState;
use crate::toast::{self, Toast};
use tauri::{AppHandle, Manager};

type CmdResult<T> = Result<T, String>;

/// The biggest picture the editor takes, in pixels on a side, so a mistake can't use all the memory.
const MAX_SIDE: u32 = 8192;

fn say(app: &AppHandle, kind: &str, title: &str, body: &str) {
    let lang = lang_of(&app.state::<AppState>().settings.get());
    toast::show(
        app,
        Toast::new(kind, tr(lang, title), tr(lang, body)).opens("shots"),
    );
}

/// Decodes a PNG into 8-bit RGBA, refusing anything absurdly big or a different kind of file.
fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|_| "That is not a picture NomNom can read.".to_string())?;
    let info = reader.info();
    if info.width == 0 || info.height == 0 || info.width > MAX_SIDE || info.height > MAX_SIDE {
        return Err("That picture is too big.".into());
    }
    let size = reader
        .output_buffer_size()
        .ok_or("That picture is too big.".to_string())?;
    let mut buf = vec![0; size];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|_| "That is not a picture NomNom can read.".to_string())?;
    buf.truncate(frame.buffer_size());
    let rgba = match frame.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Indexed => return Err("That is not a picture NomNom can read.".into()),
    };
    Ok((frame.width, frame.height, rgba))
}

/// What the page sends: the PNG as base64 (with or without the `data:image/png;base64,` start).
fn png_from_base64(text: &str) -> Result<Vec<u8>, String> {
    let body = text.rsplit(',').next().unwrap_or(text).trim();
    let bytes = base64_decode(body).ok_or("The picture didn't arrive whole.")?;
    // A PNG always starts with the same eight bytes.
    if bytes.len() < 8 || bytes[..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Err("That is not a PNG picture.".into());
    }
    Ok(bytes)
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::windows::shared_webview_args;
    use std::sync::Mutex;
    use tauri::{PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
    use tauri_plugin_clipboard_manager::ClipboardExt;

    const OVERLAY: &str = "shot";
    const EDITOR: &str = "shotedit";

    struct Held {
        width: u32,
        height: u32,
        bgra: Vec<u8>,
        /// Where this screen starts on the whole desktop.
        x: i32,
        y: i32,
    }
    static HELD: Mutex<Option<Held>> = Mutex::new(None);
    /// What the rectangle is for: `None` is a screenshot, `Some(gif)` is a recording.
    static PURPOSE: Mutex<Option<bool>> = Mutex::new(None);
    /// The picked part, as PNG, while the editor is open.
    static EDITING: Mutex<Option<Vec<u8>>> = Mutex::new(None);

    pub fn start(app: &AppHandle) {
        *PURPOSE.lock().unwrap() = None;
        begin(app);
    }

    pub fn start_for_recording(app: &AppHandle, gif: bool) {
        *PURPOSE.lock().unwrap() = Some(gif);
        begin(app);
    }

    fn begin(app: &AppHandle) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if app.get_webview_window(OVERLAY).is_some() || app.get_webview_window(EDITOR).is_some()
            {
                return;
            }
            let a = app.clone();
            let shot = tokio::task::spawn_blocking(move || crate::ocr::screen_under_pointer(&a))
                .await
                .ok()
                .flatten();
            let Some((x, y, shot)) = shot else {
                say(
                    &app,
                    "error",
                    "Couldn't read the screen",
                    "Windows didn't let NomNom take a picture of the screen.",
                );
                return;
            };
            let (w, h) = (shot.width, shot.height);
            *HELD.lock().unwrap() = Some(Held {
                width: w,
                height: h,
                bgra: shot.bgra,
                x,
                y,
            });
            let built = WebviewWindowBuilder::new(
                &app,
                OVERLAY,
                WebviewUrl::App("panel.html?kind=shot".into()),
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
                    tracing::warn!("couldn't open the screenshot window: {e}");
                    *HELD.lock().unwrap() = None;
                }
            }
        });
    }

    pub fn overlay_image() -> Option<String> {
        let held = HELD.lock().unwrap();
        let h = held.as_ref()?;
        let png = crate::ocr::png_of(h.width, h.height, &h.bgra)?;
        Some(nomnom_activity::encode_icon_data_url(&png))
    }

    pub fn overlay_ready(app: &AppHandle) {
        if let Some(win) = app.get_webview_window(OVERLAY) {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }

    pub fn overlay_cancel(app: &AppHandle) {
        *HELD.lock().unwrap() = None;
        if let Some(win) = app.get_webview_window(OVERLAY) {
            let _ = win.destroy();
        }
    }

    /// The rectangle was chosen: cut it out, drop the whole-screen picture, open the editor.
    pub fn pick(app: &AppHandle, x: i64, y: i64, w: i64, h: i64) -> CmdResult<()> {
        let held = HELD.lock().unwrap().take();
        if let Some(win) = app.get_webview_window(OVERLAY) {
            let _ = win.destroy();
        }
        let held = held.ok_or("The picture is gone. Press the shortcut again.")?;
        let (cw, ch, part) = crate::ocr::crop(&held.bgra, held.width, held.height, x, y, w, h)
            .ok_or("Drag a bigger rectangle.")?;
        // A rectangle chosen for a recording goes to the recorder, as whole-desktop pixels.
        let purpose = *PURPOSE.lock().unwrap();
        if let Some(gif) = purpose {
            let (cx, cy) = (
                x.clamp(0, i64::from(held.width)) as i32,
                y.clamp(0, i64::from(held.height)) as i32,
            );
            crate::recorder::start_chosen(app, held.x + cx, held.y + cy, cw, ch, gif);
            return Ok(());
        }
        let png = crate::ocr::png_of(cw, ch, &part).ok_or("Couldn't make the picture.")?;
        *EDITING.lock().unwrap() = Some(png);
        let (win_w, win_h) = (
            f64::from(cw).clamp(640.0, 1400.0) + 40.0,
            f64::from(ch).clamp(360.0, 800.0) + 120.0,
        );
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let built = WebviewWindowBuilder::new(
                &app,
                EDITOR,
                WebviewUrl::App("panel.html?kind=shotedit".into()),
            )
            .title("NomNom screenshot")
            .inner_size(win_w, win_h)
            .min_inner_size(560.0, 360.0)
            .resizable(true)
            .center()
            .visible(false);
            if let Err(e) = shared_webview_args(built).build() {
                tracing::warn!("couldn't open the screenshot editor: {e}");
                *EDITING.lock().unwrap() = None;
            }
        });
        Ok(())
    }

    pub fn edit_image() -> Option<String> {
        let png = EDITING.lock().unwrap().clone()?;
        Some(nomnom_activity::encode_icon_data_url(&png))
    }

    pub fn editor_ready(app: &AppHandle) {
        if let Some(win) = app.get_webview_window(EDITOR) {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }

    pub fn close_editor(app: &AppHandle) {
        *EDITING.lock().unwrap() = None;
        if let Some(win) = app.get_webview_window(EDITOR) {
            let _ = win.destroy();
        }
    }

    pub fn copy(app: &AppHandle, base64: &str) -> CmdResult<()> {
        let bytes = png_from_base64(base64)?;
        let (w, h, rgba) = decode_png(&bytes)?;
        crate::clipboard::suppress(1500);
        app.clipboard()
            .write_image(&tauri::image::Image::new_owned(rgba, w, h))
            .map_err(|e| format!("Couldn't use the clipboard: {e}"))
    }
}

#[cfg(windows)]
pub use imp::{start, start_for_recording};

#[cfg(not(windows))]
pub fn start(app: &AppHandle) {
    say(
        app,
        "info",
        "NomNom",
        "Screenshots with notes work on Windows only.",
    );
}

/// Writes the picture the editor made. The path comes from the system's Save window; it must end
/// in `.png`, and what is written is checked to be a PNG, so this can't be used to write anything else.
fn save_png(path: &str, base64: &str) -> CmdResult<()> {
    let target = std::path::Path::new(path);
    if target.is_dir()
        || target
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("png"))
    {
        return Err("Choose a file name that ends in .png.".into());
    }
    let bytes = png_from_base64(base64)?;
    decode_png(&bytes)?;
    std::fs::write(target, bytes).map_err(|e| format!("Couldn't save the picture: {e}"))
}

#[tauri::command]
pub fn sh_image() -> Option<String> {
    #[cfg(windows)]
    {
        imp::overlay_image()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[tauri::command]
pub fn sh_ready(app: AppHandle) {
    #[cfg(windows)]
    imp::overlay_ready(&app);
    #[cfg(not(windows))]
    let _ = app;
}

#[tauri::command]
pub fn sh_cancel(app: AppHandle) {
    #[cfg(windows)]
    imp::overlay_cancel(&app);
    #[cfg(not(windows))]
    let _ = app;
}

/// The rectangle chosen on the frozen screen, in picture pixels.
#[tauri::command]
pub fn sh_pick(app: AppHandle, x: i64, y: i64, w: i64, h: i64) -> CmdResult<()> {
    #[cfg(windows)]
    {
        imp::pick(&app, x, y, w, h)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, x, y, w, h);
        Ok(())
    }
}

#[tauri::command]
pub fn sh_edit_image() -> Option<String> {
    #[cfg(windows)]
    {
        imp::edit_image()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[tauri::command]
pub fn sh_editor_ready(app: AppHandle) {
    #[cfg(windows)]
    imp::editor_ready(&app);
    #[cfg(not(windows))]
    let _ = app;
}

#[tauri::command]
pub fn sh_close(app: AppHandle) {
    #[cfg(windows)]
    imp::close_editor(&app);
    #[cfg(not(windows))]
    let _ = app;
}

/// Puts the finished picture on the clipboard.
#[tauri::command]
pub fn sh_copy(app: AppHandle, png: String) -> CmdResult<()> {
    #[cfg(windows)]
    {
        imp::copy(&app, &png)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, png);
        Err("Screenshots with notes work on Windows only.".into())
    }
}

#[tauri::command]
pub fn sh_save(path: String, png: String) -> CmdResult<()> {
    save_png(&path, &png)
}

/// The same as the shortcut, for the button on the page.
#[tauri::command]
pub fn sh_start(app: AppHandle) {
    start(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, 2, 2);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 9, 9, 0])
                .unwrap();
        }
        out
    }

    fn b64(bytes: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::new();
        for c in bytes.chunks(3) {
            let n = (u32::from(c[0]) << 16)
                | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
                | u32::from(*c.get(2).unwrap_or(&0));
            s.push(T[(n >> 18) as usize & 63] as char);
            s.push(T[(n >> 12) as usize & 63] as char);
            s.push(if c.len() > 1 {
                T[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            s.push(if c.len() > 2 {
                T[n as usize & 63] as char
            } else {
                '='
            });
        }
        s
    }

    #[test]
    fn a_data_url_png_is_read_back_whole() {
        let png = tiny_png();
        let url = format!("data:image/png;base64,{}", b64(&png));
        assert_eq!(png_from_base64(&url).unwrap(), png);
        let (w, h, rgba) = decode_png(&png).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgba[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn anything_that_is_not_a_png_is_refused() {
        assert!(png_from_base64("!!!!").is_err());
        assert!(png_from_base64(&b64(b"just some text, not a picture")).is_err());
        assert!(decode_png(b"not a png at all").is_err());
    }

    #[test]
    fn saving_writes_only_a_png_to_a_png_name() {
        let d = tempfile::tempdir().unwrap();
        let ok = d.path().join("shot.png");
        save_png(ok.to_str().unwrap(), &b64(&tiny_png())).unwrap();
        assert!(ok.exists());
        let txt = d.path().join("shot.txt");
        assert!(save_png(txt.to_str().unwrap(), &b64(&tiny_png())).is_err());
        assert!(!txt.exists());
        assert!(save_png(d.path().to_str().unwrap(), &b64(&tiny_png())).is_err());
        let bad = d.path().join("bad.png");
        assert!(save_png(bad.to_str().unwrap(), &b64(b"MZ not a picture")).is_err());
        assert!(!bad.exists());
    }
}
