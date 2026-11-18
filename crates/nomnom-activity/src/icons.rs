//! Extracts native PNG icons (and `.desktop` display names on Linux) for tracked apps.
//!
//! - **Windows**: Extracts the 32-bit BGRA icon from the executable via `SHGetFileInfoW` +
//!   `GetIconInfo` + `GetDIBits`, converts to RGBA (applying the 1-bit `hbmMask` when an older
//!   executable has an empty alpha channel), and encodes a compact PNG.
//! - **Linux**: Scans freedesktop `.desktop` entries across XDG, Flatpak, Snap and system
//!   application directories, matching `StartupWMClass`, desktop ID or `Exec`, and resolves the
//!   corresponding PNG icon from `hicolor` or `pixmaps`.

use base64::Engine as _;

#[derive(Debug, Clone, Default)]
pub struct ExtractedMeta {
    pub icon_png: Option<Vec<u8>>,
    pub display_name: Option<String>,
}

pub fn encode_icon_data_url(png_bytes: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    format!("data:image/png;base64,{b64}")
}

pub fn extract_app_meta(exe_path: Option<&str>, wm_class: Option<&str>) -> ExtractedMeta {
    #[cfg(windows)]
    {
        let _ = wm_class;
        ExtractedMeta {
            icon_png: exe_path.and_then(win_icons::extract_exe_png),
            display_name: None,
        }
    }
    #[cfg(target_os = "linux")]
    {
        linux_icons::extract_linux_meta(exe_path, wm_class)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (exe_path, wm_class);
        ExtractedMeta::default()
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn extract_desktop_name(
    exe_path: Option<&str>,
    wm_class: Option<&str>,
) -> Option<String> {
    linux_icons::find_desktop_entry(exe_path, wm_class).and_then(|(name, _)| name)
}

#[allow(dead_code)]
pub(crate) fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || rgba.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    let mut out = Vec::with_capacity(2048);
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
    }
    Some(out)
}

#[cfg(windows)]
mod win_icons {
    use super::encode_rgba_png;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    pub fn extract_exe_png(exe_path: &str) -> Option<Vec<u8>> {
        if !Path::new(exe_path).is_file() {
            return None;
        }
        let wide: Vec<u16> = std::ffi::OsStr::new(exe_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut shfi = SHFILEINFOW::default();
            let res = SHGetFileInfoW(
                PCWSTR(wide.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut shfi),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if res == 0 || shfi.hIcon.0.is_null() {
                return None;
            }
            let png = hicon_to_png(shfi.hIcon);
            let _ = DestroyIcon(shfi.hIcon);
            png
        }
    }

    unsafe fn hicon_to_png(hicon: HICON) -> Option<Vec<u8>> {
        let mut info = ICONINFO::default();
        GetIconInfo(hicon, &mut info).ok()?;

        let result = (|| {
            let hbm_color = info.hbmColor;
            if hbm_color.0.is_null() {
                return None;
            }
            let mut bm = BITMAP::default();
            if GetObjectW(
                hbm_color,
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bm as *mut BITMAP as *mut _),
            ) == 0
            {
                return None;
            }
            let w = bm.bmWidth.clamp(1, 256) as u32;
            let h = bm.bmHeight.unsigned_abs().clamp(1, 256);

            let hdc = GetDC(HWND::default());
            if hdc.0.is_null() {
                return None;
            }

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    biHeight: -(h as i32), // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bgra = vec![0u8; (w * h * 4) as usize];
            let lines = GetDIBits(
                hdc,
                hbm_color,
                0,
                h,
                Some(bgra.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // Check whether the icon already carries 32-bit per-pixel alpha.
            let has_alpha = bgra.as_chunks::<4>().0.iter().any(|px| px[3] != 0);
            if !has_alpha && !info.hbmMask.0.is_null() {
                let mut mask_bgra = vec![0u8; (w * h * 4) as usize];
                let mut mask_bmi = bmi;
                let mask_lines = GetDIBits(
                    hdc,
                    info.hbmMask,
                    0,
                    h,
                    Some(mask_bgra.as_mut_ptr() as *mut _),
                    &mut mask_bmi,
                    DIB_RGB_COLORS,
                );
                if mask_lines > 0 {
                    for (px, m) in bgra
                        .as_chunks_mut::<4>()
                        .0
                        .iter_mut()
                        .zip(mask_bgra.as_chunks::<4>().0.iter())
                    {
                        px[3] = if m[0] == 0 { 255 } else { 0 };
                    }
                } else {
                    for px in bgra.as_chunks_mut::<4>().0 {
                        px[3] = 255;
                    }
                }
            }

            let _ = ReleaseDC(HWND::default(), hdc);
            if lines <= 0 {
                return None;
            }

            // Convert BGRA -> RGBA
            for px in bgra.as_chunks_mut::<4>().0 {
                px.swap(0, 2);
            }
            encode_rgba_png(w, h, &bgra)
        })();

        if !info.hbmColor.0.is_null() {
            let _ = DeleteObject(info.hbmColor);
        }
        if !info.hbmMask.0.is_null() {
            let _ = DeleteObject(info.hbmMask);
        }
        result
    }
}

#[cfg(target_os = "linux")]
mod linux_icons {
    use super::ExtractedMeta;
    use std::path::{Path, PathBuf};

    const MAX_ICON_BYTES: u64 = 512 * 1024;

    pub fn extract_linux_meta(exe_path: Option<&str>, wm_class: Option<&str>) -> ExtractedMeta {
        let Some((name, icon_key)) = find_desktop_entry(exe_path, wm_class) else {
            // Try direct icon lookup from wm_class or exe stem even without a .desktop match
            let fallback_icon = wm_class.and_then(resolve_icon_png).or_else(|| {
                exe_path
                    .and_then(|p| Path::new(p).file_stem())
                    .and_then(|s| s.to_str())
                    .and_then(resolve_icon_png)
            });
            return ExtractedMeta {
                icon_png: fallback_icon,
                display_name: None,
            };
        };
        let icon_png = icon_key.as_deref().and_then(resolve_icon_png);
        ExtractedMeta {
            icon_png,
            display_name: name,
        }
    }

    fn desktop_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            let h = PathBuf::from(&home);
            dirs.push(h.join(".local/share/applications"));
            dirs.push(h.join(".local/share/flatpak/exports/share/applications"));
        }
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(xdg).join("applications"));
        }
        dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
        dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
        dirs.push(PathBuf::from("/usr/local/share/applications"));
        dirs.push(PathBuf::from("/usr/share/applications"));
        dirs
    }

    pub fn find_desktop_entry(
        exe_path: Option<&str>,
        wm_class: Option<&str>,
    ) -> Option<(Option<String>, Option<String>)> {
        let class_l = wm_class
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty());
        let stem_l = exe_path
            .and_then(|p| Path::new(p).file_name())
            .and_then(|s| s.to_str())
            .map(|s| {
                s.strip_suffix(".exe")
                    .or_else(|| s.strip_suffix(".AppImage"))
                    .unwrap_or(s)
                    .to_ascii_lowercase()
            })
            .filter(|s| !s.is_empty());

        if class_l.is_none() && stem_l.is_none() {
            return None;
        }

        let mut best: Option<(u8, Option<String>, Option<String>)> = None;

        for dir in desktop_dirs() {
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in rd.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                    continue;
                }
                let file_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let parsed = parse_desktop_file(&text);
                if parsed.no_display {
                    continue;
                }

                let mut score = 0u8;
                if let Some(ref cl) = class_l {
                    if parsed.startup_wm_class.as_deref() == Some(cl.as_str()) {
                        score = 100;
                    } else if file_stem == *cl || file_stem.ends_with(&format!(".{cl}")) {
                        score = 90;
                    }
                }
                if score == 0 {
                    if let Some(ref st) = stem_l {
                        if parsed.exec_bin.as_deref() == Some(st.as_str()) {
                            score = 80;
                        } else if file_stem == *st {
                            score = 70;
                        }
                    }
                }
                if score > best.as_ref().map(|b| b.0).unwrap_or(0) {
                    best = Some((score, parsed.name, parsed.icon));
                    if score == 100 {
                        break;
                    }
                }
            }
            if best.as_ref().is_some_and(|b| b.0 == 100) {
                break;
            }
        }

        best.map(|(_, name, icon)| (name, icon))
    }

    struct DesktopParsed {
        name: Option<String>,
        icon: Option<String>,
        startup_wm_class: Option<String>,
        exec_bin: Option<String>,
        no_display: bool,
    }

    fn parse_desktop_file(text: &str) -> DesktopParsed {
        let mut in_main = false;
        let mut name = None;
        let mut icon = None;
        let mut startup_wm_class = None;
        let mut exec_bin = None;
        let mut no_display = false;

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                in_main = line == "[Desktop Entry]";
                continue;
            }
            if !in_main {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let key = k.trim();
            let val = v.trim();
            match key {
                "Name" if name.is_none() => name = Some(val.to_string()),
                "Icon" if icon.is_none() => icon = Some(val.to_string()),
                "StartupWMClass" => startup_wm_class = Some(val.to_ascii_lowercase()),
                "NoDisplay" => no_display = val.eq_ignore_ascii_case("true"),
                "Exec" if exec_bin.is_none() => {
                    let first = val
                        .split_whitespace()
                        .find(|tok| !tok.contains('=') && *tok != "env")
                        .unwrap_or_default()
                        .trim_matches('"');
                    if let Some(bin) = Path::new(first).file_name().and_then(|s| s.to_str()) {
                        exec_bin = Some(bin.to_ascii_lowercase());
                    }
                }
                _ => {}
            }
        }

        DesktopParsed {
            name,
            icon,
            startup_wm_class,
            exec_bin,
            no_display,
        }
    }

    fn resolve_icon_png(icon_name: &str) -> Option<Vec<u8>> {
        let trimmed = icon_name.trim();
        if trimmed.is_empty() {
            return None;
        }
        let direct = Path::new(trimmed);
        if direct.is_absolute() && direct.is_file() {
            if trimmed.to_ascii_lowercase().ends_with(".png") {
                return read_capped(direct);
            }
            return None;
        }

        let base = trimmed.strip_suffix(".png").unwrap_or(trimmed);
        let mut roots = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            let h = PathBuf::from(home);
            roots.push(h.join(".local/share/icons/hicolor"));
            roots.push(h.join(".icons/hicolor"));
            roots.push(h.join(".local/share/flatpak/exports/share/icons/hicolor"));
        }
        roots.push(PathBuf::from(
            "/var/lib/flatpak/exports/share/icons/hicolor",
        ));
        roots.push(PathBuf::from("/usr/share/icons/hicolor"));
        roots.push(PathBuf::from("/usr/local/share/icons/hicolor"));

        for size in [
            "128x128", "64x64", "256x256", "96x96", "48x48", "512x512", "32x32",
        ] {
            for root in &roots {
                let candidate = root.join(size).join("apps").join(format!("{base}.png"));
                if candidate.is_file() {
                    if let Some(bytes) = read_capped(&candidate) {
                        return Some(bytes);
                    }
                }
            }
        }

        let pixmap = PathBuf::from(format!("/usr/share/pixmaps/{base}.png"));
        if pixmap.is_file() {
            return read_capped(&pixmap);
        }
        None
    }

    fn read_capped(path: &Path) -> Option<Vec<u8>> {
        let meta = std::fs::metadata(path).ok()?;
        if meta.len() == 0 || meta.len() > MAX_ICON_BYTES {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        // Verify PNG magic signature
        bytes.starts_with(b"\x89PNG\r\n\x1a\n").then_some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_rgba_pixels_into_valid_png_data_url() {
        let rgba = vec![255u8; 4 * 4 * 4];
        let png = encode_rgba_png(4, 4, &rgba).unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        let url = encode_icon_data_url(&png);
        assert!(url.starts_with("data:image/png;base64,"));
    }
}
