//! What can be converted to what, and the options that shape a conversion.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Image,
    Video,
    Audio,
    Text,
    Office,
}

impl Kind {
    pub fn of(path: &Path) -> Option<Kind> {
        let ext = ext_lower(path)?;
        Some(match ext.as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff" | "ico" => Kind::Image,
            "mp4" | "m4v" | "mkv" | "mov" | "webm" | "avi" | "wmv" | "flv" | "mpg" | "mpeg"
            | "3gp" | "ts" => Kind::Video,
            "mp3" | "m4a" | "aac" | "wav" | "flac" | "ogg" | "oga" | "opus" | "wma" | "aiff"
            | "aif" => Kind::Audio,
            "txt" | "md" | "markdown" => Kind::Text,
            "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "rtf" => {
                Kind::Office
            }
            _ => return None,
        })
    }
}

pub fn ext_lower(path: &Path) -> Option<String> {
    Some(path.extension()?.to_str()?.to_ascii_lowercase())
}

pub fn is_markdown(path: &Path) -> bool {
    matches!(ext_lower(path).as_deref(), Some("md" | "markdown"))
}

/// Audio formats that already threw information away.
pub fn is_lossy_audio(path: &Path) -> bool {
    matches!(
        ext_lower(path).as_deref(),
        Some("mp3" | "m4a" | "aac" | "ogg" | "oga" | "opus" | "wma")
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Png,
    Jpg,
    Webp,
    Bmp,
    Tiff,
    Gif,
    Ico,
    Pdf,
    Html,
    Mp4,
    Webm,
    Mkv,
    /// Keeps the video as it is and drops the sound.
    NoAudio,
    Mp3,
    Flac,
    Wav,
    Ogg,
    Opus,
    M4a,
}

impl Target {
    /// Extension of the file written. `NoAudio` keeps the source's container, so it has none.
    pub fn ext(self) -> Option<&'static str> {
        Some(match self {
            Target::Png => "png",
            Target::Jpg => "jpg",
            Target::Webp => "webp",
            Target::Bmp => "bmp",
            Target::Tiff => "tiff",
            Target::Gif => "gif",
            Target::Ico => "ico",
            Target::Pdf => "pdf",
            Target::Html => "html",
            Target::Mp4 => "mp4",
            Target::Webm => "webm",
            Target::Mkv => "mkv",
            Target::NoAudio => return None,
            Target::Mp3 => "mp3",
            Target::Flac => "flac",
            Target::Wav => "wav",
            Target::Ogg => "ogg",
            Target::Opus => "opus",
            Target::M4a => "m4a",
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Target::Png => "PNG",
            Target::Jpg => "JPG",
            Target::Webp => "WebP",
            Target::Bmp => "BMP",
            Target::Tiff => "TIFF",
            Target::Gif => "GIF",
            Target::Ico => "ICO (icon)",
            Target::Pdf => "PDF",
            Target::Html => "HTML",
            Target::Mp4 => "MP4",
            Target::Webm => "WebM",
            Target::Mkv => "MKV",
            Target::NoAudio => "Same video, no sound",
            Target::Mp3 => "MP3",
            Target::Flac => "FLAC",
            Target::Wav => "WAV",
            Target::Ogg => "OGG",
            Target::Opus => "Opus",
            Target::M4a => "M4A",
        }
    }

    pub fn is_audio(self) -> bool {
        matches!(
            self,
            Target::Mp3 | Target::Flac | Target::Wav | Target::Ogg | Target::Opus | Target::M4a
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Image,
    Pdf,
    Ffmpeg,
    Office,
}

/// Every (kind, target) pair NOMNOM knows about. Whether it works right now depends on the tools.
pub fn targets_for(kind: Kind) -> &'static [Target] {
    use Target::*;
    match kind {
        Kind::Image => &[Png, Jpg, Webp, Bmp, Tiff, Gif, Ico, Pdf],
        Kind::Video => &[Mp4, Webm, Mkv, Gif, NoAudio, Mp3, M4a, Wav, Flac, Ogg, Opus],
        Kind::Audio => &[Mp3, Flac, Wav, Ogg, Opus, M4a],
        Kind::Text => &[Pdf, Html],
        Kind::Office => &[Pdf],
    }
}

pub fn backend_for(kind: Kind, target: Target) -> Backend {
    match (kind, target) {
        (Kind::Image, Target::Pdf) => Backend::Pdf,
        (Kind::Image, _) => Backend::Image,
        (Kind::Video | Kind::Audio, _) => Backend::Ffmpeg,
        (Kind::Text, _) => Backend::Pdf,
        (Kind::Office, _) => Backend::Office,
    }
}

/// Whether `target` makes sense for this particular file.
pub fn valid_for(path: &Path, target: Target) -> bool {
    let Some(kind) = Kind::of(path) else {
        return false;
    };
    if !targets_for(kind).contains(&target) {
        return false;
    }
    // Only Markdown has anything to turn into HTML.
    if target == Target::Html && !is_markdown(path) {
        return false;
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoQuality {
    Small,
    Balanced,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfPage {
    /// Each page has the size of its picture.
    Fit,
    /// A4, picture scaled to fit with a margin.
    A4,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// 1 to 100. JPG and lossy WebP. 100 makes WebP lossless.
    pub image_quality: u8,
    pub video_quality: VideoQuality,
    /// Fit inside this box; never makes anything bigger.
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub keep_metadata: bool,
    /// Fills transparent areas when the result has no transparency.
    pub background: [u8; 3],
    pub fps: Option<u32>,
    pub start_secs: Option<f64>,
    pub end_secs: Option<f64>,
    /// Audio bitrate for MP3, M4A, OGG and Opus.
    pub audio_kbps: Option<u32>,
    pub pdf_page: PdfPage,
    /// Several pictures into one PDF.
    pub combine: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            image_quality: 85,
            video_quality: VideoQuality::Balanced,
            max_width: None,
            max_height: None,
            keep_metadata: true,
            background: [255, 255, 255],
            fps: None,
            start_secs: None,
            end_secs: None,
            audio_kbps: None,
            pdf_page: PdfPage::Fit,
            combine: false,
        }
    }
}

impl Options {
    /// Keeps values inside ranges the encoders accept. Anything odd falls back to the default.
    pub fn sanitized(mut self) -> Options {
        self.image_quality = self.image_quality.clamp(1, 100);
        let fix_dim = |v: Option<u32>| v.filter(|n| (1..=16384).contains(n));
        self.max_width = fix_dim(self.max_width);
        self.max_height = fix_dim(self.max_height);
        self.fps = self.fps.filter(|n| (1..=120).contains(n));
        self.audio_kbps = self.audio_kbps.filter(|n| (32..=512).contains(n));
        let ok_time = |v: Option<f64>| v.filter(|t| t.is_finite() && *t >= 0.0 && *t < 86_400.0);
        self.start_secs = ok_time(self.start_secs);
        self.end_secs = ok_time(self.end_secs);
        if let (Some(s), Some(e)) = (self.start_secs, self.end_secs) {
            if e <= s {
                self.end_secs = None;
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn kinds_follow_the_extension_case_insensitively() {
        assert_eq!(Kind::of(&PathBuf::from("a.PNG")), Some(Kind::Image));
        assert_eq!(Kind::of(&PathBuf::from("a.mkv")), Some(Kind::Video));
        assert_eq!(Kind::of(&PathBuf::from("a.flac")), Some(Kind::Audio));
        assert_eq!(Kind::of(&PathBuf::from("a.md")), Some(Kind::Text));
        assert_eq!(Kind::of(&PathBuf::from("a.docx")), Some(Kind::Office));
        assert_eq!(Kind::of(&PathBuf::from("a.exe")), None);
        assert_eq!(Kind::of(&PathBuf::from("noext")), None);
    }

    #[test]
    fn html_is_offered_for_markdown_only() {
        assert!(valid_for(&PathBuf::from("a.md"), Target::Html));
        assert!(!valid_for(&PathBuf::from("a.txt"), Target::Html));
        assert!(valid_for(&PathBuf::from("a.txt"), Target::Pdf));
        assert!(!valid_for(&PathBuf::from("a.mp3"), Target::Mp4));
    }

    #[test]
    fn options_are_clamped_to_safe_ranges() {
        let o = Options {
            image_quality: 0,
            max_width: Some(0),
            max_height: Some(99999),
            fps: Some(0),
            audio_kbps: Some(1),
            start_secs: Some(10.0),
            end_secs: Some(5.0),
            ..Options::default()
        }
        .sanitized();
        assert_eq!(o.image_quality, 1);
        assert_eq!(o.max_width, None);
        assert_eq!(o.max_height, None);
        assert_eq!(o.fps, None);
        assert_eq!(o.audio_kbps, None);
        assert_eq!(o.end_secs, None);
    }
}
