//! Deciding what a file really is. The name is only a hint: the first bytes must agree with it,
//! or the file is left alone.

use crate::model::Category;

pub fn default_categories() -> Vec<Category> {
    let c = |name: &str, exts: &[&str]| Category {
        name: name.into(),
        exts: exts.iter().map(|s| s.to_string()).collect(),
    };
    vec![
        c(
            "Videos",
            &[
                "mp4", "m4v", "mkv", "mov", "webm", "avi", "wmv", "flv", "mpg", "mpeg", "3gp",
            ],
        ),
        c(
            "Audio",
            &[
                "mp3", "m4a", "aac", "wav", "flac", "ogg", "oga", "opus", "wma", "aiff",
            ],
        ),
        c(
            "Images",
            &[
                "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "heic", "heif", "avif",
                "svg", "ico",
            ],
        ),
        c(
            "Documents",
            &[
                "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "odt", "ods", "odp", "rtf",
                "txt", "md", "csv", "epub",
            ],
        ),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Video,
    Audio,
    /// Windows Media container: either a video (wmv) or audio (wma).
    Asf,
    Image,
    Pdf,
    Zip,
    Ole,
    Rtf,
    Text,
    /// An executable, a script or a similar program file.
    Program,
    Unknown,
}

fn text_like(head: &[u8]) -> bool {
    if head.starts_with(&[0xFF, 0xFE]) || head.starts_with(&[0xFE, 0xFF]) {
        return true; // UTF-16 with a byte order mark
    }
    if head.contains(&0) {
        return false;
    }
    match std::str::from_utf8(head) {
        Ok(_) => true,
        // a multi-byte character cut off at the end of the sample is fine
        Err(e) => e.error_len().is_none(),
    }
}

/// What the first bytes say the file is.
pub fn sniff(head: &[u8]) -> Family {
    use Family::*;
    let starts = |p: &[u8]| head.starts_with(p);
    if starts(b"MZ")
        || starts(b"\x7fELF")
        || starts(b"#!")
        || starts(&[0xCA, 0xFE, 0xBA, 0xBE])
        || starts(&[0xFE, 0xED, 0xFA])
        || starts(&[0xCF, 0xFA, 0xED, 0xFE])
    {
        return Program;
    }
    if starts(&[0x89, b'P', b'N', b'G'])
        || starts(&[0xFF, 0xD8, 0xFF])
        || starts(b"GIF8")
        || starts(b"II*\0")
        || starts(b"MM\0*")
        || starts(&[0, 0, 1, 0])
    {
        return Image;
    }
    if starts(b"BM") && head.len() >= 14 {
        return Image;
    }
    if starts(b"RIFF") && head.len() >= 12 {
        return match &head[8..12] {
            b"WEBP" => Image,
            b"WAVE" => Audio,
            b"AVI " => Video,
            _ => Unknown,
        };
    }
    if head.len() >= 12 && &head[4..8] == b"ftyp" {
        return match &head[8..12] {
            b"M4A " | b"M4B " | b"M4P " => Audio,
            b"heic" | b"heix" | b"mif1" | b"msf1" | b"avif" | b"avis" => Image,
            _ => Video,
        };
    }
    if starts(b"%PDF") {
        return Pdf;
    }
    if starts(b"PK\x03\x04") {
        return Zip;
    }
    if starts(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        return Ole;
    }
    if starts(b"{\\rtf") {
        return Rtf;
    }
    if starts(b"ID3")
        || starts(b"fLaC")
        || starts(b"OggS")
        || (starts(b"FORM") && head.len() >= 12 && &head[8..12] == b"AIFF")
    {
        return Audio;
    }
    if head.len() >= 2 && head[0] == 0xFF && (head[1] & 0xE0) == 0xE0 {
        return Audio; // MP3 or ADTS AAC frame sync
    }
    if starts(&[0x1A, 0x45, 0xDF, 0xA3])
        || starts(b"FLV\x01")
        || starts(&[0, 0, 1, 0xBA])
        || starts(&[0, 0, 1, 0xB3])
    {
        return Video;
    }
    if starts(&[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11]) {
        return Asf;
    }
    if !head.is_empty() && text_like(head) {
        return Text;
    }
    Unknown
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// `Ok` when the file's contents fit its extension, otherwise a sentence saying why not.
pub fn check_content(ext: &str, head: &[u8]) -> Result<(), String> {
    use Family::*;
    if head.is_empty() {
        return Err("The file is empty.".into());
    }
    let family = sniff(head);
    if family == Program {
        return Err(format!(
            "It's named .{ext} but it looks like a program, so it was left alone."
        ));
    }
    let wrong = || Err(format!("The contents don't look like a .{ext} file."));
    match ext {
        "mp4" | "m4v" | "mkv" | "mov" | "webm" | "avi" | "flv" | "mpg" | "mpeg" | "3gp" => {
            (family == Video).then_some(()).map_or_else(wrong, Ok)
        }
        "wmv" => matches!(family, Video | Asf)
            .then_some(())
            .map_or_else(wrong, Ok),
        "mp3" | "m4a" | "aac" | "wav" | "flac" | "ogg" | "oga" | "opus" | "aiff" => {
            (family == Audio).then_some(()).map_or_else(wrong, Ok)
        }
        "wma" => matches!(family, Audio | Asf)
            .then_some(())
            .map_or_else(wrong, Ok),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "webp" | "ico" | "heic"
        | "heif" | "avif" => (family == Image).then_some(()).map_or_else(wrong, Ok),
        "svg" => {
            let lower = head.to_ascii_lowercase();
            if family != Text {
                wrong()
            } else if contains(&lower, b"<script") || contains(&lower, b"onload=") {
                Err("This SVG contains a script, so it was left alone.".into())
            } else {
                Ok(())
            }
        }
        "pdf" => (family == Pdf).then_some(()).map_or_else(wrong, Ok),
        "docx" | "xlsx" | "pptx" => (family == Zip && contains(head, b"[Content_Types].xml"))
            .then_some(())
            .map_or_else(wrong, Ok),
        "odt" | "ods" | "odp" | "epub" => (family == Zip && contains(head, b"mimetype"))
            .then_some(())
            .map_or_else(wrong, Ok),
        "doc" | "xls" | "ppt" => (family == Ole).then_some(()).map_or_else(wrong, Ok),
        "rtf" => (family == Rtf).then_some(()).map_or_else(wrong, Ok),
        "txt" | "md" | "csv" => (family == Text).then_some(()).map_or_else(wrong, Ok),
        _ => Err(format!("NomNom doesn't know how to check .{ext} files.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp4() -> Vec<u8> {
        let mut v = vec![0, 0, 0, 0x18];
        v.extend_from_slice(b"ftypisom\0\0\0\0isomiso2");
        v
    }

    #[test]
    fn real_files_pass_for_their_own_extension() {
        assert!(check_content("mp4", &mp4()).is_ok());
        assert!(check_content("png", &[0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0]).is_ok());
        assert!(check_content("jpg", &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10]).is_ok());
        assert!(check_content("pdf", b"%PDF-1.7\n%").is_ok());
        assert!(check_content("mp3", b"ID3\x03\0\0\0\0\0\0").is_ok());
        assert!(check_content("mp3", &[0xFF, 0xFB, 0x90, 0x00]).is_ok());
        assert!(check_content("flac", b"fLaC\0\0\0\"").is_ok());
        assert!(check_content("wav", b"RIFF\x24\0\0\0WAVEfmt ").is_ok());
        assert!(check_content("mkv", &[0x1A, 0x45, 0xDF, 0xA3, 0x9F]).is_ok());
        assert!(check_content("txt", "olá, mundo — texto normal".as_bytes()).is_ok());
        assert!(check_content("docx", b"PK\x03\x04\x14\0\x06\0[Content_Types].xml").is_ok());
        assert!(check_content("odt", b"PK\x03\x04\x14\0\0\0mimetypeapplication/vnd.oasis").is_ok());
    }

    #[test]
    fn a_program_dressed_up_as_media_is_refused() {
        for ext in ["mp4", "png", "pdf", "mp3", "txt", "docx", "jpg"] {
            let err = check_content(ext, b"MZ\x90\0\x03\0\0\0").unwrap_err();
            assert!(err.contains("program"), "{ext}: {err}");
        }
        assert!(check_content("mp4", b"#!/bin/sh\nrm -rf /")
            .unwrap_err()
            .contains("program"));
        assert!(check_content("png", b"\x7fELF\x02\x01")
            .unwrap_err()
            .contains("program"));
    }

    #[test]
    fn the_wrong_kind_of_content_is_refused() {
        assert!(check_content("mp4", b"%PDF-1.4").is_err());
        assert!(check_content("png", &mp4()).is_err());
        assert!(check_content("pdf", b"just text").is_err());
        assert!(check_content("docx", b"PK\x03\x04plain zip with no office parts").is_err());
        assert!(check_content("txt", b"PK\x03\x04binary").is_err());
        assert!(check_content("txt", &[0x00, 0x01, 0x02, 0x03]).is_err());
    }

    #[test]
    fn empty_and_unknown_are_refused_with_a_reason() {
        assert!(check_content("mp4", b"").unwrap_err().contains("empty"));
        assert!(check_content("xyz", b"anything").is_err());
    }

    #[test]
    fn svg_with_a_script_is_left_alone() {
        assert!(check_content(
            "svg",
            b"<svg xmlns='http://www.w3.org/2000/svg'><rect/></svg>"
        )
        .is_ok());
        assert!(
            check_content("svg", b"<svg><script>alert(1)</script></svg>")
                .unwrap_err()
                .contains("script")
        );
        assert!(check_content("svg", b"<svg onload='x()'></svg>").is_err());
    }

    #[test]
    fn a_text_sample_cut_in_the_middle_of_a_character_is_still_text() {
        let s = "aé".as_bytes();
        assert_eq!(sniff(&s[..2]), Family::Text);
    }

    #[test]
    fn m4a_and_heic_are_told_apart_from_video() {
        let mut m4a = vec![0, 0, 0, 0x20];
        m4a.extend_from_slice(b"ftypM4A \0\0\0\0");
        assert_eq!(sniff(&m4a), Family::Audio);
        let mut heic = vec![0, 0, 0, 0x18];
        heic.extend_from_slice(b"ftypheic\0\0\0\0");
        assert_eq!(sniff(&heic), Family::Image);
        assert!(check_content("m4a", &m4a).is_ok());
        assert!(check_content("mp4", &m4a).is_err());
    }

    #[test]
    fn default_categories_cover_the_usual_suspects_once_each() {
        let cats = default_categories();
        let mut all: Vec<&String> = cats.iter().flat_map(|c| &c.exts).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(
            all.len(),
            n,
            "an extension must belong to one category only"
        );
        assert!(cats
            .iter()
            .any(|c| c.name == "Videos" && c.exts.contains(&"mkv".to_string())));
        assert!(!all
            .iter()
            .any(|e| ["exe", "dll", "bat", "ps1", "msi", "zip"].contains(&e.as_str())));
    }
}
