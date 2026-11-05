//! Ready-made settings for common jobs.

use crate::model::{Kind, Options, PdfPage, Target, VideoQuality};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub kind: Kind,
    pub target: Target,
    pub options: Options,
}

pub fn presets() -> Vec<Preset> {
    vec![
        Preset {
            id: "web-video",
            name: "Web video",
            description: "MP4, up to 1080p, a good size for sharing.",
            kind: Kind::Video,
            target: Target::Mp4,
            options: Options {
                max_height: Some(1080),
                video_quality: VideoQuality::Balanced,
                ..Options::default()
            },
        },
        Preset {
            id: "small-gif",
            name: "Small GIF",
            description: "480 pixels wide, 12 frames a second.",
            kind: Kind::Video,
            target: Target::Gif,
            options: Options {
                max_width: Some(480),
                fps: Some(12),
                ..Options::default()
            },
        },
        Preset {
            id: "podcast-mp3",
            name: "Podcast MP3",
            description: "MP3 at 128 kbps.",
            kind: Kind::Audio,
            target: Target::Mp3,
            options: Options {
                audio_kbps: Some(128),
                ..Options::default()
            },
        },
        Preset {
            id: "lossless-flac",
            name: "Lossless FLAC",
            description: "FLAC keeps everything the source has.",
            kind: Kind::Audio,
            target: Target::Flac,
            options: Options::default(),
        },
        Preset {
            id: "webp-80",
            name: "WebP, quality 80",
            description: "Smaller pictures for the web.",
            kind: Kind::Image,
            target: Target::Webp,
            options: Options {
                image_quality: 80,
                ..Options::default()
            },
        },
        Preset {
            id: "photos-pdf",
            name: "Photos to PDF",
            description: "All pictures in one A4 PDF.",
            kind: Kind::Image,
            target: Target::Pdf,
            options: Options {
                pdf_page: PdfPage::A4,
                combine: true,
                ..Options::default()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_targets_something_its_kind_can_make() {
        for p in presets() {
            assert!(
                crate::model::targets_for(p.kind).contains(&p.target),
                "{}",
                p.id
            );
        }
    }

    #[test]
    fn preset_ids_are_unique() {
        let mut ids: Vec<_> = presets().iter().map(|p| p.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }
}
