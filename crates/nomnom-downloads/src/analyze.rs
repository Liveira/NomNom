//! Turns yt-dlp's `-J` output into what the UI shows. Nothing is invented: a resolution, size or
//! duration appears only when the source reports it.

use crate::error::DlError;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AudioFormat {
    pub ext: String,
    pub codec: Option<String>,
    pub bitrate_kbps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PlaylistEntry {
    pub title: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct MediaInfo {
    pub extractor: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub webpage_url: String,
    pub uploader: Option<String>,
    pub duration_secs: Option<u64>,
    pub is_live: bool,
    /// Available video heights, highest first. Empty for audio-only sources.
    pub heights: Vec<u32>,
    /// Estimated total size (best video at that height plus best audio) when the source says.
    pub size_by_height: BTreeMap<u32, u64>,
    pub audio_size: Option<u64>,
    pub audio_formats: Vec<AudioFormat>,
    /// Present when the link points to a playlist or gallery.
    pub entries: Option<Vec<PlaylistEntry>>,
}

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
}

fn size_of(f: &Value) -> Option<u64> {
    ["filesize", "filesize_approx"]
        .iter()
        .find_map(|k| f.get(k).and_then(Value::as_f64))
        .filter(|n| *n > 0.0)
        .map(|n| n as u64)
}

fn has_codec(f: &Value, key: &str) -> bool {
    f.get(key)
        .and_then(Value::as_str)
        .is_some_and(|c| c != "none")
}

pub fn parse_media_info(json: &Value, requested_url: &str) -> Result<MediaInfo, DlError> {
    let mut info = MediaInfo {
        extractor: s(json, "extractor_key")
            .or_else(|| s(json, "extractor"))
            .unwrap_or_default(),
        title: s(json, "title").unwrap_or_else(|| requested_url.to_string()),
        thumbnail: s(json, "thumbnail"),
        webpage_url: s(json, "webpage_url").unwrap_or_else(|| requested_url.to_string()),
        uploader: s(json, "uploader").or_else(|| s(json, "channel")),
        duration_secs: json
            .get("duration")
            .and_then(Value::as_f64)
            .filter(|d| *d > 0.0)
            .map(|d| d as u64),
        is_live: json
            .get("is_live")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        ..Default::default()
    };

    if json.get("_type").and_then(Value::as_str) == Some("playlist") {
        let entries: Vec<PlaylistEntry> = json
            .get("entries")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|e| {
                        let url = s(e, "webpage_url").or_else(|| s(e, "url"))?;
                        Some(PlaylistEntry {
                            title: s(e, "title"),
                            url,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        if entries.is_empty() {
            return Err(DlError::Failed(
                "This playlist has nothing to download.".into(),
            ));
        }
        info.entries = Some(entries);
        return Ok(info);
    }

    let formats = json
        .get("formats")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut best_audio_size: Option<u64> = None;
    let mut best_audio_abr: f64 = -1.0;
    let mut heights: BTreeMap<u32, Option<u64>> = BTreeMap::new();

    for f in &formats {
        let has_video = has_codec(f, "vcodec");
        let has_audio = has_codec(f, "acodec");
        if has_video {
            if let Some(h) = f.get("height").and_then(Value::as_u64).filter(|h| *h > 0) {
                let entry = heights.entry(h as u32).or_insert(None);
                if let Some(sz) = size_of(f) {
                    *entry = Some(entry.map_or(sz, |old| old.max(sz)));
                }
            }
        } else if has_audio {
            let abr = f.get("abr").and_then(Value::as_f64).unwrap_or(0.0);
            info.audio_formats.push(AudioFormat {
                ext: s(f, "ext").unwrap_or_default(),
                codec: s(f, "acodec"),
                bitrate_kbps: (abr > 0.0).then_some(abr.round() as u32),
            });
            if abr > best_audio_abr {
                best_audio_abr = abr;
                best_audio_size = size_of(f);
            }
        }
    }

    info.audio_size = best_audio_size;
    info.heights = heights.keys().rev().copied().collect();
    for (h, size) in heights {
        if let Some(v) = size {
            info.size_by_height
                .insert(h, v + best_audio_size.unwrap_or(0));
        }
    }
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "extractor_key": "Youtube", "title": "A video", "thumbnail": "https://i.test/t.jpg",
            "webpage_url": "https://youtu.be/x", "uploader": "Someone", "duration": 61.0, "is_live": false,
            "formats": [
                {"vcodec":"avc1","acodec":"none","height":1080,"filesize":1000},
                {"vcodec":"vp9","acodec":"none","height":1080,"filesize_approx":1500.0},
                {"vcodec":"avc1","acodec":"none","height":720},
                {"vcodec":"none","acodec":"mp4a","abr":128.0,"ext":"m4a","filesize":200},
                {"vcodec":"none","acodec":"opus","abr":160.0,"ext":"webm","filesize":300},
                {"vcodec":"none","acodec":"none","ext":"mhtml","height":90}
            ]
        })
    }

    #[test]
    fn lists_only_real_heights_highest_first() {
        let i = parse_media_info(&sample(), "u").unwrap();
        assert_eq!(i.heights, vec![1080, 720]);
        assert_eq!(i.title, "A video");
        assert_eq!(i.duration_secs, Some(61));
    }

    #[test]
    fn sizes_only_when_reported_and_include_best_audio() {
        let i = parse_media_info(&sample(), "u").unwrap();
        assert_eq!(
            i.size_by_height.get(&1080),
            Some(&1800),
            "largest video 1500 + best audio 300"
        );
        assert_eq!(
            i.size_by_height.get(&720),
            None,
            "720p reports no size, so none is invented"
        );
    }

    #[test]
    fn collects_audio_formats() {
        let i = parse_media_info(&sample(), "u").unwrap();
        assert_eq!(i.audio_formats.len(), 2);
        assert_eq!(i.audio_formats[1].bitrate_kbps, Some(160));
    }

    #[test]
    fn audio_only_source_has_no_heights() {
        let j = json!({"title":"Song","formats":[{"vcodec":"none","acodec":"mp3","abr":96.0,"ext":"mp3"}]});
        let i = parse_media_info(&j, "u").unwrap();
        assert!(i.heights.is_empty());
        assert_eq!(i.audio_formats.len(), 1);
    }

    #[test]
    fn missing_fields_stay_missing() {
        let i = parse_media_info(&json!({"formats":[]}), "https://x.test/v").unwrap();
        assert_eq!(i.title, "https://x.test/v");
        assert_eq!(i.duration_secs, None);
        assert_eq!(i.thumbnail, None);
    }

    #[test]
    fn playlists_expose_entries() {
        let j = json!({"_type":"playlist","title":"List","entries":[
            {"title":"One","url":"https://x.test/1"}, {"url":"https://x.test/2"}, {"title":"no url"}]});
        let i = parse_media_info(&j, "u").unwrap();
        assert_eq!(i.entries.as_ref().unwrap().len(), 2);
        assert!(parse_media_info(&json!({"_type":"playlist","entries":[]}), "u").is_err());
    }
}
