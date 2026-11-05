//! Safe file names and collision-proof placement of finished downloads.

use nomnom_core::fs::move_no_replace;
use std::io;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

const MAX_UNITS: usize = 180;
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn truncate_units(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for ch in s.chars() {
        let n = ch.len_utf16();
        if used + n > max {
            break;
        }
        out.push(ch);
        used += n;
    }
    out
}

/// Turns arbitrary text into a name that is valid on Windows. Never returns an empty string.
pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .nfc()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    let (stem, ext) = match cleaned.rfind('.') {
        Some(i) if i > 0 && cleaned.len() - i <= 12 => (&cleaned[..i], &cleaned[i..]),
        _ => (cleaned.as_str(), ""),
    };
    let ext = ext.trim_end_matches([' ', '.']);
    let mut stem = stem.trim_matches([' ', '.']).to_string();

    if stem.is_empty() {
        stem = "download".into();
    }
    if RESERVED.contains(&stem.to_ascii_uppercase().as_str()) {
        stem.push('_');
    }
    let room = MAX_UNITS.saturating_sub(utf16_len(ext)).max(1);
    let stem = truncate_units(&stem, room);
    let stem = stem.trim_end_matches([' ', '.']);
    let stem = if stem.is_empty() { "download" } else { stem };
    format!("{stem}{ext}")
}

/// Moves `staged` into `dest_dir` as `desired_name`, adding " (2)", " (3)"... if the name is
/// taken. Never overwrites: the OS refuses to replace, so racing writers are handled too.
pub fn finalize(staged: &Path, dest_dir: &Path, desired_name: &str) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dest_dir)?;
    let name = sanitize_filename(desired_name);
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()),
        _ => (name.clone(), String::new()),
    };
    for n in 1..=9999u32 {
        let candidate = if n == 1 {
            dest_dir.join(&name)
        } else {
            dest_dir.join(format!("{stem} ({n}){ext}"))
        };
        match move_no_replace(staged, &candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many files with this name",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_forbidden_characters() {
        assert_eq!(
            sanitize_filename("a<b>c:d\"e/f\\g|h?i*j.mp4"),
            "a_b_c_d_e_f_g_h_i_j.mp4"
        );
    }

    #[test]
    fn handles_reserved_names_and_trailing_dots() {
        assert_eq!(sanitize_filename("CON.mp4"), "CON_.mp4");
        assert_eq!(sanitize_filename("nul"), "nul_");
        assert_eq!(sanitize_filename("movie. . "), "movie");
        assert_eq!(sanitize_filename("  .. "), "download");
        assert_eq!(sanitize_filename(""), "download");
    }

    #[test]
    fn collapses_control_characters_and_whitespace() {
        assert_eq!(sanitize_filename("a\n\tb\u{0}c  d.txt"), "a b c d.txt");
    }

    #[test]
    fn caps_length_in_utf16_units_and_keeps_extension() {
        let long = format!("{}.mp4", "é".repeat(500));
        let out = sanitize_filename(&long);
        assert!(utf16_len(&out) <= MAX_UNITS);
        assert!(out.ends_with(".mp4"));
        let emoji = format!("{}.webm", "😀".repeat(300));
        let out = sanitize_filename(&emoji);
        assert!(utf16_len(&out) <= MAX_UNITS && out.ends_with(".webm"));
    }

    #[test]
    fn normalizes_unicode() {
        assert_eq!(sanitize_filename("e\u{301}.mp3"), "\u{e9}.mp3");
    }

    #[test]
    fn never_produces_invalid_names_for_random_junk() {
        let long = "x".repeat(400);
        let junk = [
            "...",
            "<<<>>>",
            "a.b.c.d",
            ".hidden",
            long.as_str(),
            "COM1.txt",
            "a . ",
        ];
        for j in junk {
            let out = sanitize_filename(j);
            assert!(
                !out.is_empty() && utf16_len(&out) <= MAX_UNITS,
                "{j:?} -> {out:?}"
            );
            assert!(!out.ends_with('.') && !out.ends_with(' '));
            assert!(!out.contains(['<', '>', ':', '"', '/', '\\', '|', '?', '*']));
        }
    }

    #[test]
    fn finalize_never_overwrites_and_numbers_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("Clip.mp4"), "original").unwrap();

        let staged = dir.path().join("staged.bin");
        std::fs::write(&staged, "new").unwrap();
        let p = finalize(&staged, &dest, "clip.MP4").unwrap();
        assert_eq!(p.file_name().unwrap(), "clip (2).MP4");
        assert_eq!(
            std::fs::read_to_string(dest.join("Clip.mp4")).unwrap(),
            "original"
        );

        std::fs::write(&staged, "newer").unwrap();
        let p = finalize(&staged, &dest, "clip.MP4").unwrap();
        assert_eq!(p.file_name().unwrap(), "clip (3).MP4");
    }
}
