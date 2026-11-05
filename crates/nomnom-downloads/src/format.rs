//! What the user asked for, translated into yt-dlp arguments. Arguments are always a vector,
//! and the URL always comes last after `--`, so nothing in a URL can be read as an option.

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Video,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    Original,
    Mp4,
    Webm,
    Mkv,
    Mp3,
    M4a,
    Wav,
    Flac,
    Ogg,
    Opus,
}

impl Container {
    pub fn as_str(self) -> &'static str {
        match self {
            Container::Original => "original",
            Container::Mp4 => "mp4",
            Container::Webm => "webm",
            Container::Mkv => "mkv",
            Container::Mp3 => "mp3",
            Container::M4a => "m4a",
            Container::Wav => "wav",
            Container::Flac => "flac",
            Container::Ogg => "ogg",
            Container::Opus => "opus",
        }
    }

    pub fn parse(s: &str) -> Option<Container> {
        use Container::*;
        [Original, Mp4, Webm, Mkv, Mp3, M4a, Wav, Flac, Ogg, Opus]
            .into_iter()
            .find(|c| c.as_str() == s)
    }

    fn is_audio(self) -> bool {
        matches!(
            self,
            Container::Mp3
                | Container::M4a
                | Container::Wav
                | Container::Flac
                | Container::Ogg
                | Container::Opus
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub kind: Kind,
    pub container: Container,
    /// `None` means best available.
    pub height: Option<u32>,
}

impl Choice {
    /// The default everywhere: best video, whatever container the source uses.
    pub const BEST_VIDEO: Choice = Choice {
        kind: Kind::Video,
        container: Container::Original,
        height: None,
    };

    pub fn validate(&self) -> Result<(), String> {
        match (self.kind, self.container.is_audio()) {
            (Kind::Video, true) => Err("Pick a video format such as MP4 or WebM.".into()),
            (Kind::Audio, false) => Err("Pick an audio format such as MP3 or M4A.".into()),
            (Kind::Audio, true) if self.height.is_some() => Err("Audio has no resolution.".into()),
            _ => Ok(()),
        }
    }

    /// True when the output needs re-encoding from a lossy source into a lossless container.
    pub fn is_lossless_target(&self) -> bool {
        matches!(self.container, Container::Wav | Container::Flac)
    }
}

pub struct ArgsContext<'a> {
    pub ffmpeg: Option<&'a Path>,
    pub staging: &'a Path,
    pub firefox_cookies: bool,
}

fn os(s: &str) -> OsString {
    OsString::from(s)
}

/// Options shared by analysis and download.
pub fn common_args(firefox_cookies: bool) -> Vec<OsString> {
    let mut a: Vec<OsString> = [
        "--no-warnings",
        "--no-exec",
        "--no-playlist",
        "--ignore-config",
    ]
    .into_iter()
    .map(os)
    .collect();
    if firefox_cookies {
        a.push(os("--cookies-from-browser"));
        a.push(os("firefox"));
    }
    a
}

pub fn build_download_args(choice: &Choice, ctx: &ArgsContext<'_>, url: &str) -> Vec<OsString> {
    let mut a = common_args(ctx.firefox_cookies);
    for s in [
        "--newline",
        "--progress",
        "--continue",
        "--windows-filenames",
        "--no-mtime",
        "--progress-template",
        "download:NOMNOM %(progress)j",
        "--print",
        "after_move:NOMNOM_PATH %(filepath)s",
    ] {
        a.push(os(s));
    }
    if let Some(ff) = ctx.ffmpeg {
        a.push(os("--ffmpeg-location"));
        a.push(ff.as_os_str().to_owned());
    }
    a.push(os("--paths"));
    a.push(os(&format!("temp:{}", ctx.staging.display())));
    a.push(os("--paths"));
    a.push(os(&format!("home:{}", ctx.staging.display())));
    a.push(os("-o"));
    a.push(os("%(title).150B.%(ext)s"));

    match choice.kind {
        Kind::Video => {
            let selector = match choice.height {
                None => "bv*+ba/b".to_string(),
                Some(h) => format!("bv*[height<={h}]+ba/b[height<={h}]"),
            };
            a.push(os("-f"));
            a.push(os(&selector));
            match choice.container {
                Container::Mp4 => {
                    a.extend(
                        [
                            "-S",
                            "vcodec:h264,res,acodec:m4a",
                            "--merge-output-format",
                            "mp4",
                        ]
                        .map(os),
                    );
                }
                Container::Webm => {
                    a.extend(
                        [
                            "-S",
                            "vcodec:vp9,acodec:opus",
                            "--merge-output-format",
                            "webm",
                        ]
                        .map(os),
                    );
                }
                Container::Mkv => a.extend(["--merge-output-format", "mkv"].map(os)),
                _ => {}
            }
        }
        Kind::Audio => {
            // Prefer a source that is already in the target codec so nothing is re-encoded.
            let selector = match choice.container {
                Container::M4a => "ba[ext=m4a]/ba/b",
                Container::Opus => "ba[acodec=opus]/ba/b",
                _ => "ba/b",
            };
            a.extend(
                [
                    "-f",
                    selector,
                    "-x",
                    "--audio-format",
                    choice.container.as_str(),
                    "--audio-quality",
                    "0",
                ]
                .map(os),
            );
        }
    }
    a.push(os("--"));
    a.push(os(url));
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn args(choice: Choice) -> Vec<String> {
        let staging = PathBuf::from("C:\\stage");
        let ctx = ArgsContext {
            ffmpeg: Some(Path::new("C:\\ff\\ffmpeg.exe")),
            staging: &staging,
            firefox_cookies: false,
        };
        build_download_args(&choice, &ctx, "https://example.com/v")
            .into_iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect()
    }

    fn has_pair(a: &[String], k: &str, v: &str) -> bool {
        a.windows(2).any(|w| w[0] == k && w[1] == v)
    }

    #[test]
    fn best_video_default() {
        let a = args(Choice::BEST_VIDEO);
        assert!(has_pair(&a, "-f", "bv*+ba/b"));
        assert!(!a.iter().any(|x| x == "--merge-output-format"));
    }

    #[test]
    fn height_limit_applies_to_both_branches() {
        let a = args(Choice {
            kind: Kind::Video,
            container: Container::Original,
            height: Some(720),
        });
        assert!(has_pair(&a, "-f", "bv*[height<=720]+ba/b[height<=720]"));
    }

    #[test]
    fn mp4_webm_mkv_choose_codecs_and_merge_format() {
        let mp4 = args(Choice {
            kind: Kind::Video,
            container: Container::Mp4,
            height: None,
        });
        assert!(
            has_pair(&mp4, "-S", "vcodec:h264,res,acodec:m4a")
                && has_pair(&mp4, "--merge-output-format", "mp4")
        );
        let webm = args(Choice {
            kind: Kind::Video,
            container: Container::Webm,
            height: None,
        });
        assert!(has_pair(&webm, "--merge-output-format", "webm"));
        let mkv = args(Choice {
            kind: Kind::Video,
            container: Container::Mkv,
            height: None,
        });
        assert!(has_pair(&mkv, "--merge-output-format", "mkv"));
    }

    #[test]
    fn audio_uses_extraction_and_prefers_matching_source() {
        let m4a = args(Choice {
            kind: Kind::Audio,
            container: Container::M4a,
            height: None,
        });
        assert!(
            has_pair(&m4a, "-f", "ba[ext=m4a]/ba/b") && has_pair(&m4a, "--audio-format", "m4a")
        );
        assert!(m4a.iter().any(|x| x == "-x"));
        let mp3 = args(Choice {
            kind: Kind::Audio,
            container: Container::Mp3,
            height: None,
        });
        assert!(has_pair(&mp3, "-f", "ba/b") && has_pair(&mp3, "--audio-quality", "0"));
    }

    #[test]
    fn url_is_last_and_follows_separator() {
        let a = args(Choice::BEST_VIDEO);
        let n = a.len();
        assert_eq!(a[n - 2], "--");
        assert_eq!(a[n - 1], "https://example.com/v");
    }

    #[test]
    fn hostile_url_stays_a_single_argument() {
        let staging = PathBuf::from("C:\\s");
        let ctx = ArgsContext {
            ffmpeg: None,
            staging: &staging,
            firefox_cookies: false,
        };
        let a = build_download_args(
            &Choice::BEST_VIDEO,
            &ctx,
            "https://x.test/ --exec calc & whoami",
        );
        assert_eq!(
            a.last().unwrap().to_string_lossy(),
            "https://x.test/ --exec calc & whoami"
        );
        assert!(a.iter().any(|x| x == "--no-exec"));
    }

    #[test]
    fn always_safe_options_present() {
        let a = args(Choice::BEST_VIDEO);
        for flag in [
            "--no-exec",
            "--no-playlist",
            "--ignore-config",
            "--windows-filenames",
        ] {
            assert!(a.iter().any(|x| x == flag), "missing {flag}");
        }
        assert!(!a.iter().any(|x| x == "--cookies-from-browser"));
    }

    #[test]
    fn cookies_only_when_opted_in() {
        let staging = PathBuf::from("C:\\s");
        let ctx = ArgsContext {
            ffmpeg: None,
            staging: &staging,
            firefox_cookies: true,
        };
        let a: Vec<String> = build_download_args(&Choice::BEST_VIDEO, &ctx, "https://x.test/")
            .into_iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(has_pair(&a, "--cookies-from-browser", "firefox"));
    }

    #[test]
    fn rejects_mismatched_kind_and_container() {
        assert!(Choice {
            kind: Kind::Video,
            container: Container::Mp3,
            height: None
        }
        .validate()
        .is_err());
        assert!(Choice {
            kind: Kind::Audio,
            container: Container::Mp4,
            height: None
        }
        .validate()
        .is_err());
        assert!(Choice {
            kind: Kind::Audio,
            container: Container::Mp3,
            height: Some(720)
        }
        .validate()
        .is_err());
        assert!(Choice::BEST_VIDEO.validate().is_ok());
    }

    #[test]
    fn container_names_round_trip() {
        for c in [
            Container::Original,
            Container::Mp4,
            Container::Flac,
            Container::Opus,
        ] {
            assert_eq!(Container::parse(c.as_str()), Some(c));
        }
        assert_eq!(Container::parse("exe"), None);
    }
}
