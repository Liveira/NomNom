//! QR codes: make one from text or a Wi-Fi network, and read one that is on the screen.

#![allow(clippy::chunks_exact_to_as_chunks)] // the newer `as_chunks` is not in every supported Rust

use nomnom_core::qr as rules;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

/// A code as a small picture the page can show or save.
fn svg_of(text: &str) -> CmdResult<String> {
    if text.trim().is_empty() {
        return Err("Type something to put in the code.".into());
    }
    if text.len() > rules::MAX_TEXT {
        return Err("That is too long for a code that a phone can read.".into());
    }
    let code = qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::M)
        .map_err(|_| "That is too long for a code that a phone can read.".to_string())?;
    Ok(code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(260, 260)
        .quiet_zone(true)
        .dark_color(qrcode::render::svg::Color("#000000"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build())
}

#[tauri::command]
pub fn qr_make(text: String) -> CmdResult<String> {
    svg_of(&text)
}

/// The code for joining a Wi-Fi network. Returns the picture and the text inside it.
#[tauri::command]
pub fn qr_wifi(
    ssid: String,
    password: String,
    security: String,
    hidden: bool,
) -> CmdResult<(String, String)> {
    let payload = rules::wifi_payload(&ssid, &password, &security, hidden)?;
    Ok((svg_of(&payload)?, payload))
}

/// Saves the picture the page is showing. The path comes from the Save window and must end in `.svg`;
/// what is written has to be the SVG this file made, so this can't write anything else.
#[tauri::command]
pub fn qr_save(path: String, svg: String) -> CmdResult<()> {
    let target = std::path::Path::new(&path);
    if target.is_dir()
        || target
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("svg"))
    {
        return Err("Choose a file name that ends in .svg.".into());
    }
    if !(svg.starts_with("<?xml") || svg.starts_with("<svg"))
        || svg.contains("<script")
        || svg.len() > 400_000
    {
        return Err("That is not a code NomNom made.".into());
    }
    std::fs::write(target, svg).map_err(|e| format!("Couldn't save it: {e}"))
}

#[derive(Debug, Serialize)]
pub struct Scanned {
    pub text: String,
    pub kind: &'static str,
    pub openable: bool,
}

/// Looks for QR codes in a grey picture (one byte a pixel).
pub fn decode_grey(width: usize, height: usize, grey: &[u8]) -> Vec<String> {
    if grey.len() < width * height || width < 21 || height < 21 {
        return Vec::new();
    }
    let mut img =
        rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| grey[y * width + x]);
    let mut out: Vec<String> = Vec::new();
    for grid in img.detect_grids() {
        if let Ok((_, content)) = grid.decode() {
            if !out.contains(&content) {
                out.push(content);
            }
        }
    }
    out
}

/// Takes a picture of the screen under the pointer and reads the codes on it. The picture is not kept.
#[tauri::command]
pub async fn qr_read_screen(app: AppHandle) -> CmdResult<Vec<Scanned>> {
    #[cfg(windows)]
    {
        let shot = tokio::task::spawn_blocking(move || crate::ocr::screen_under_pointer(&app))
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Windows didn't let NomNom take a picture of the screen.")?;
        let (_, _, shot) = shot;
        let found = tokio::task::spawn_blocking(move || {
            let (w, h) = (shot.width as usize, shot.height as usize);
            let grey: Vec<u8> = shot
                .bgra
                .chunks_exact(4)
                .map(|p| {
                    ((u32::from(p[2]) * 299 + u32::from(p[1]) * 587 + u32::from(p[0]) * 114) / 1000)
                        as u8
                })
                .collect();
            decode_grey(w, h, &grey)
        })
        .await
        .map_err(|e| e.to_string())?;
        Ok(found
            .into_iter()
            .map(|text| Scanned {
                kind: rules::kind_of(&text),
                openable: rules::is_openable_link(&text),
                text,
            })
            .collect())
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Reading a code from the screen works on Windows for now.".into())
    }
}

/// Opens a web or mail link that was read from a code. Anything else is refused.
#[tauri::command]
pub fn qr_open(app: AppHandle, link: String) -> CmdResult<()> {
    if !rules::is_openable_link(&link) {
        return Err("Only web and mail links can be opened from here.".into());
    }
    app.opener()
        .open_url(link.trim(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A code made here is read back by the reader, byte for byte: both halves are exercised.
    #[test]
    fn a_code_made_here_reads_back_the_same() {
        for text in [
            "https://example.com/a?b=1&c=2",
            "WIFI:T:WPA;S:Home;P:secret123;;",
            "Olá, mundo — 🐱",
        ] {
            let code =
                qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::M)
                    .unwrap();
            let width = code.width();
            let scale = 6usize;
            let quiet = 4usize;
            let side = (width + quiet * 2) * scale;
            let mut grey = vec![255u8; side * side];
            let colors = code.to_colors();
            for y in 0..width {
                for x in 0..width {
                    if colors[y * width + x] == qrcode::Color::Dark {
                        for dy in 0..scale {
                            for dx in 0..scale {
                                grey[((y + quiet) * scale + dy) * side
                                    + (x + quiet) * scale
                                    + dx] = 0;
                            }
                        }
                    }
                }
            }
            assert_eq!(
                decode_grey(side, side, &grey),
                vec![text.to_string()],
                "{text}"
            );
        }
    }

    #[test]
    fn a_blank_picture_has_no_code() {
        assert!(decode_grey(200, 200, &vec![255u8; 40_000]).is_empty());
        assert!(decode_grey(10, 10, &[0u8; 100]).is_empty());
    }

    #[test]
    fn the_picture_is_an_svg_and_too_much_text_is_refused() {
        assert!(svg_of("hello").unwrap().contains("<svg"));
        assert!(svg_of("   ").is_err());
        assert!(svg_of(&"x".repeat(5000)).is_err());
    }

    #[test]
    fn saving_writes_only_an_svg_to_an_svg_name() {
        let d = tempfile::tempdir().unwrap();
        let ok = d.path().join("code.svg");
        qr_save(ok.to_str().unwrap().into(), svg_of("hi").unwrap()).unwrap();
        assert!(ok.exists());
        assert!(qr_save(
            d.path().join("code.txt").to_str().unwrap().into(),
            svg_of("hi").unwrap()
        )
        .is_err());
        assert!(qr_save(
            d.path().join("x.svg").to_str().unwrap().into(),
            "<html><script>alert(1)</script>".into()
        )
        .is_err());
        assert!(!d.path().join("x.svg").exists());
    }
}
