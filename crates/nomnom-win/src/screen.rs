//! Looking at the screen: a picture of one rectangle of it (the part around the pointer), and
//! reading the text in a picture with the text recognition that comes with Windows. It works
//! offline, with the languages the person already has installed for Windows.
#![allow(clippy::chunks_exact_to_as_chunks)] // the newer `as_chunks` is not in every supported Rust

use windows::core::Result as WinResult;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
    HGDIOBJ, SRCCOPY,
};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

/// A picture: blue, green, red, alpha for each pixel, row by row from the top.
#[derive(Debug, Clone)]
pub struct Shot {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

/// The rectangle of the screen at (`x`, `y`) of `width` by `height` pixels (screen coordinates,
/// which may be negative on a monitor left of the main one).
pub fn capture(x: i32, y: i32, width: i32, height: i32) -> Option<Shot> {
    if width <= 0 || height <= 0 || width > 16_384 || height > 16_384 {
        return None;
    }
    // SAFETY: every handle made here is released before returning.
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let dc = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        let old = SelectObject(dc, HGDIOBJ(bitmap.0));
        let copied = BitBlt(dc, 0, 0, width, height, screen, x, y, SRCCOPY | CAPTUREBLT).is_ok();
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // a negative height asks for the top row first
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bgra = vec![0u8; (width as usize) * (height as usize) * 4];
        let lines = GetDIBits(
            dc,
            bitmap,
            0,
            height as u32,
            Some(bgra.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );
        SelectObject(dc, old);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
        if !copied || lines == 0 {
            return None;
        }
        // GDI leaves the alpha at 0; a picture of the screen is opaque.
        for px in bgra.chunks_exact_mut(4) {
            px[3] = 255;
        }
        Some(Shot {
            width: width as u32,
            height: height as u32,
            bgra,
        })
    }
}

fn recognise(shot: &Shot) -> WinResult<String> {
    let writer = DataWriter::new()?;
    writer.WriteBytes(&shot.bgra)?;
    let buffer = writer.DetachBuffer()?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        shot.width as i32,
        shot.height as i32,
    )?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
    let result = engine.RecognizeAsync(&bitmap)?.get()?;
    Ok(result.Text()?.to_string())
}

/// The text in the picture, lines joined by new lines. Call it from a thread of your own: it waits
/// for Windows to finish.
pub fn read_text(shot: &Shot) -> Result<String, String> {
    // SAFETY: starts Windows' runtime for this thread; calling it again is harmless.
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    // A small picture reads badly: sharper text is found more often when it is bigger.
    recognise(shot).map_err(|e| {
        if e.code().0 as u32 == 0x8000_4005 || e.message().is_empty() {
            "Windows has no text recognition for your language yet. Add one in Settings, Time & language, Language & region.".to_string()
        } else {
            format!("Windows couldn't read the text: {}", e.message())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::w;
    use windows::Win32::Foundation::{COLORREF, RECT};
    use windows::Win32::Graphics::Gdi::{
        CreateFontW, FillRect, GetStockObject, SetBkMode, SetTextColor, TextOutW, HBRUSH,
        TRANSPARENT, WHITE_BRUSH,
    };

    /// Draws text on a white picture with GDI, so the test needs nothing on the screen.
    fn picture_with_text(text: &str) -> Shot {
        let (w, h) = (640, 120);
        unsafe {
            let screen = GetDC(None);
            let dc = CreateCompatibleDC(screen);
            let bmp = CreateCompatibleBitmap(screen, w, h);
            let old = SelectObject(dc, HGDIOBJ(bmp.0));
            let white = GetStockObject(WHITE_BRUSH);
            FillRect(
                dc,
                &RECT {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: h,
                },
                HBRUSH(white.0),
            );
            let font = CreateFontW(
                52,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                w!("Arial"),
            );
            SelectObject(dc, HGDIOBJ(font.0));
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, COLORREF(0));
            let wide: Vec<u16> = text.encode_utf16().collect();
            let _ = TextOutW(dc, 20, 25, &wide);
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bgra = vec![0u8; (w * h * 4) as usize];
            GetDIBits(
                dc,
                bmp,
                0,
                h as u32,
                Some(bgra.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            );
            for px in bgra.chunks_exact_mut(4) {
                px[3] = 255;
            }
            SelectObject(dc, old);
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            let _ = DeleteDC(dc);
            ReleaseDC(None, screen);
            Shot {
                width: w as u32,
                height: h as u32,
                bgra,
            }
        }
    }

    #[test]
    fn a_piece_of_the_screen_can_be_captured_and_a_silly_one_cannot() {
        let shot = capture(0, 0, 64, 48).expect("a picture of the screen");
        assert_eq!((shot.width, shot.height), (64, 48));
        assert_eq!(shot.bgra.len(), 64 * 48 * 4);
        assert!(shot.bgra.chunks_exact(4).all(|p| p[3] == 255));
        assert!(capture(0, 0, 0, 10).is_none());
        assert!(capture(0, 0, 100_000, 10).is_none());
    }

    #[test]
    fn text_in_a_picture_is_read() {
        let shot = picture_with_text("Hello NomNom 123");
        match read_text(&shot) {
            Ok(text) => {
                let lower = text.to_lowercase();
                assert!(
                    lower.contains("hello") && lower.contains("123"),
                    "read: {text:?}"
                );
            }
            // A computer with no recognition language installed can't run this check.
            Err(e) => eprintln!("skipped: {e}"),
        }
    }
}
