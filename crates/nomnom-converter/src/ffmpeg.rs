//! Audio and video through ffmpeg. Arguments are built as a list and never go through a shell.

use crate::error::ConvertError;
use crate::model::{ext_lower, Options, Target, VideoQuality};
use nomnom_core::process;
use serde::Deserialize;
use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Probe {
    pub duration: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Deserialize)]
struct ProbeJson {
    #[serde(default)]
    streams: Vec<StreamJson>,
    format: Option<FormatJson>,
}
#[derive(Deserialize)]
struct StreamJson {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    duration: Option<String>,
    #[serde(default)]
    disposition: Disposition,
}
#[derive(Deserialize, Default)]
struct Disposition {
    #[serde(default)]
    attached_pic: u8,
}
#[derive(Deserialize)]
struct FormatJson {
    duration: Option<String>,
}

pub fn parse_probe(json: &str) -> Result<Probe, ConvertError> {
    let p: ProbeJson =
        serde_json::from_str(json).map_err(|_| ConvertError::NotReadable("audio or video"))?;
    let mut out = Probe::default();
    for s in &p.streams {
        match s.codec_type.as_deref() {
            // Cover art inside an MP3 shows up as a "video" stream; it isn't one.
            Some("video") if s.disposition.attached_pic == 0 && out.video_codec.is_none() => {
                out.video_codec = s.codec_name.clone();
                out.width = s.width;
                out.height = s.height;
            }
            Some("audio") if out.audio_codec.is_none() => out.audio_codec = s.codec_name.clone(),
            _ => {}
        }
    }
    let secs = |v: &Option<String>| v.as_deref().and_then(|d| d.parse::<f64>().ok());
    out.duration = p
        .format
        .as_ref()
        .and_then(|f| secs(&f.duration))
        .or_else(|| p.streams.iter().find_map(|s| secs(&s.duration)))
        .filter(|d| d.is_finite() && *d > 0.0);
    Ok(out)
}

pub async fn probe(ffprobe: &Path, file: &Path) -> Result<Probe, ConvertError> {
    let args: Vec<OsString> = vec![
        "-v".into(),
        "error".into(),
        "-print_format".into(),
        "json".into(),
        "-show_streams".into(),
        "-show_format".into(),
        "--".into(),
        file.as_os_str().to_os_string(),
    ];
    let mut mc = process::spawn(ffprobe, args, true)
        .map_err(|e| ConvertError::Tool(format!("couldn't start ffprobe: {e}")))?;
    let mut out = mc.child.stdout.take();
    let read = async {
        let mut text = String::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_string(&mut text).await;
        }
        let _ = mc.child.wait().await;
        text
    };
    match tokio::time::timeout(Duration::from_secs(60), read).await {
        Ok(text) => parse_probe(&text),
        Err(_) => Err(ConvertError::Tool("ffprobe took too long.".into())),
    }
}

pub struct Plan {
    pub args: Vec<OsString>,
    /// True when the streams are copied without re-encoding.
    pub copy: bool,
    /// Length of what will be written, for the progress bar.
    pub expected_secs: Option<f64>,
    pub notes: Vec<String>,
}

fn crf_x264(q: VideoQuality) -> &'static str {
    match q {
        VideoQuality::Small => "28",
        VideoQuality::Balanced => "23",
        VideoQuality::High => "18",
    }
}
fn crf_vp9(q: VideoQuality) -> &'static str {
    match q {
        VideoQuality::Small => "40",
        VideoQuality::Balanced => "33",
        VideoQuality::High => "26",
    }
}

const MP4_VIDEO: &[&str] = &["h264", "hevc", "av1", "mpeg4", "vp9"];
const MP4_AUDIO: &[&str] = &["aac", "mp3", "ac3", "eac3", "opus", "alac"];
const WEBM_VIDEO: &[&str] = &["vp8", "vp9", "av1"];
const WEBM_AUDIO: &[&str] = &["opus", "vorbis"];

fn one_of(list: &[&str], codec: &Option<String>) -> bool {
    codec.as_deref().is_some_and(|c| list.contains(&c))
}

/// Builds the ffmpeg arguments for one file. `out` is a temporary path with the right extension.
pub fn plan(
    target: Target,
    o: &Options,
    probe: &Probe,
    src: &Path,
    out: &Path,
) -> Result<Plan, ConvertError> {
    let mut a: Vec<OsString> = Vec::new();
    let mut push = |items: &[&str]| a.extend(items.iter().map(OsString::from));
    let mut notes = Vec::new();

    push(&[
        "-hide_banner",
        "-nostdin",
        "-y",
        "-loglevel",
        "error",
        "-nostats",
        "-progress",
        "pipe:1",
    ]);

    let trimming = o.start_secs.is_some() || o.end_secs.is_some();
    if let Some(s) = o.start_secs {
        push(&["-ss", &format!("{s}")]);
    }
    if let Some(e) = o.end_secs {
        push(&["-to", &format!("{e}")]);
    }
    a.push("-i".into());
    a.push(src.as_os_str().to_os_string());

    let scaling = o.max_width.is_some() || o.max_height.is_some();
    let scale = {
        let w = o
            .max_width
            .map(|v| v.to_string())
            .unwrap_or_else(|| "iw".into());
        let h = o
            .max_height
            .map(|v| v.to_string())
            .unwrap_or_else(|| "ih".into());
        format!(
            "scale=w='min({w}\\,iw)':h='min({h}\\,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2"
        )
    };
    let filters_needed = scaling || o.fps.is_some();
    let mut vf: Vec<String> = Vec::new();
    if scaling {
        vf.push(scale.clone());
    }
    if let Some(f) = o.fps {
        vf.push(format!("fps={f}"));
    }
    let kbps = |default: u32| format!("{}k", o.audio_kbps.unwrap_or(default));
    let mut copy = false;

    let mut push = |items: &[&str]| a.extend(items.iter().map(OsString::from));
    match target {
        Target::Mp4 | Target::Webm | Target::Mkv => {
            if probe.video_codec.is_none() {
                return Err(ConvertError::Other("This file has no video in it.".into()));
            }
            push(&["-map", "0:v:0", "-map", "0:a:0?"]);
            let can_copy = !filters_needed
                && !trimming
                && match target {
                    Target::Mp4 => {
                        one_of(MP4_VIDEO, &probe.video_codec)
                            && (probe.audio_codec.is_none()
                                || one_of(MP4_AUDIO, &probe.audio_codec))
                    }
                    Target::Webm => {
                        one_of(WEBM_VIDEO, &probe.video_codec)
                            && (probe.audio_codec.is_none()
                                || one_of(WEBM_AUDIO, &probe.audio_codec))
                    }
                    _ => true,
                };
            if can_copy {
                copy = true;
                push(&["-c", "copy"]);
                if target == Target::Mp4 {
                    push(&["-movflags", "+faststart"]);
                }
                notes.push(
                    "The picture and sound were copied as they are, so nothing was lost.".into(),
                );
            } else {
                if !vf.is_empty() {
                    push(&["-vf", &vf.join(",")]);
                }
                match target {
                    Target::Webm => {
                        push(&[
                            "-c:v",
                            "libvpx-vp9",
                            "-crf",
                            crf_vp9(o.video_quality),
                            "-b:v",
                            "0",
                            "-row-mt",
                            "1",
                        ]);
                        push(&["-c:a", "libopus", "-b:a", &kbps(128)]);
                    }
                    _ => {
                        push(&[
                            "-c:v",
                            "libx264",
                            "-preset",
                            "medium",
                            "-crf",
                            crf_x264(o.video_quality),
                            "-pix_fmt",
                            "yuv420p",
                        ]);
                        push(&["-c:a", "aac", "-b:a", &kbps(160)]);
                        if target == Target::Mp4 {
                            push(&["-movflags", "+faststart"]);
                        }
                    }
                }
            }
        }
        Target::Gif => {
            if probe.video_codec.is_none() {
                return Err(ConvertError::Other("This file has no video in it.".into()));
            }
            let fps = o.fps.unwrap_or(12);
            let width = o.max_width.unwrap_or(480);
            let graph = format!(
                "[0:v]fps={fps},scale='min({width}\\,iw)':-2:flags=lanczos,split[a][b];\
                 [a]palettegen[p];[b][p]paletteuse"
            );
            push(&["-an", "-filter_complex", &graph, "-loop", "0"]);
            notes.push("GIF has at most 256 colours and no sound.".into());
        }
        Target::NoAudio => {
            if probe.video_codec.is_none() {
                return Err(ConvertError::Other("This file has no video in it.".into()));
            }
            push(&["-map", "0:v", "-an", "-c:v", "copy"]);
            copy = true;
            if filters_needed || trimming {
                notes.push(
                    "Size, frame rate and trim settings don't apply when only removing the sound."
                        .into(),
                );
            }
        }
        Target::Mp3 | Target::M4a | Target::Flac | Target::Wav | Target::Ogg | Target::Opus => {
            if probe.audio_codec.is_none() {
                return Err(ConvertError::Other("This file has no sound in it.".into()));
            }
            push(&["-vn", "-map", "0:a:0"]);
            match target {
                Target::Mp3 => push(&["-c:a", "libmp3lame", "-b:a", &kbps(192)]),
                Target::M4a => {
                    if !trimming && probe.audio_codec.as_deref() == Some("aac") {
                        copy = true;
                        push(&["-c:a", "copy"]);
                        notes.push("The sound was copied as it is, so nothing was lost.".into());
                    } else {
                        push(&["-c:a", "aac", "-b:a", &kbps(192)]);
                    }
                }
                Target::Flac => push(&["-c:a", "flac"]),
                Target::Wav => push(&["-c:a", "pcm_s16le"]),
                Target::Ogg => push(&["-c:a", "libvorbis", "-b:a", &kbps(160)]),
                _ => push(&["-c:a", "libopus", "-b:a", &kbps(128)]),
            }
        }
        _ => {
            return Err(ConvertError::Other(
                "ffmpeg can't write that format.".into(),
            ))
        }
    }
    // The output is always an absolute path, so it can never be read as an option.
    a.push(out.as_os_str().to_os_string());

    let span = |p: &Probe| -> Option<f64> {
        let total = p.duration?;
        let start = o.start_secs.unwrap_or(0.0).min(total);
        let end = o.end_secs.unwrap_or(total).min(total);
        (end > start).then_some(end - start)
    };
    Ok(Plan {
        args: a,
        copy,
        expected_secs: span(probe),
        notes,
    })
}

/// Parses ffmpeg's `-progress` output. Returns seconds written so far.
pub fn parse_progress_line(line: &str) -> Option<f64> {
    let (k, v) = line.split_once('=')?;
    match k {
        // Despite the name, out_time_ms is in microseconds.
        "out_time_us" | "out_time_ms" => v
            .trim()
            .parse::<i64>()
            .ok()
            .map(|n| n.max(0) as f64 / 1_000_000.0),
        _ => None,
    }
}

pub async fn run(
    ffmpeg: &Path,
    plan: &Plan,
    cancel: &CancellationToken,
    progress: &(dyn Fn(f32) + Send + Sync),
) -> Result<(), ConvertError> {
    let mut mc = process::spawn(ffmpeg, plan.args.iter(), true)
        .map_err(|e| ConvertError::Tool(format!("couldn't start ffmpeg: {e}")))?;
    let stdout = mc.child.stdout.take();
    let stderr = mc.child.stderr.take();
    let err_task = tokio::spawn(async move {
        let mut text = String::new();
        if let Some(mut e) = stderr {
            let _ = e.read_to_string(&mut text).await;
        }
        text
    });
    let expected = plan.expected_secs;
    let outcome = {
        let work = async {
            if let Some(out) = stdout {
                let mut lines = BufReader::new(out).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let (Some(done), Some(total)) = (parse_progress_line(&line), expected) {
                        progress((done / total).clamp(0.0, 0.99) as f32);
                    }
                }
            }
            mc.child.wait().await
        };
        tokio::select! {
            _ = cancel.cancelled() => None,
            r = work => Some(r),
        }
    };
    match outcome {
        None => {
            mc.kill_tree().await;
            err_task.abort();
            Err(ConvertError::Cancelled)
        }
        Some(Ok(status)) if status.success() => Ok(()),
        Some(other) => {
            let text = err_task.await.unwrap_or_default();
            tracing::warn!(?other, stderr = %text.trim(), "ffmpeg failed");
            Err(ConvertError::Tool(friendly_error(&text)))
        }
    }
}

/// Turns ffmpeg's last complaint into a sentence.
pub fn friendly_error(stderr: &str) -> String {
    let last = stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let lower = last.to_ascii_lowercase();
    if lower.contains("invalid data found") || lower.contains("moov atom not found") {
        "This file looks damaged or isn't a media file.".into()
    } else if lower.contains("no such file") {
        "The file couldn't be found.".into()
    } else if lower.contains("permission denied") {
        "NOMNOM isn't allowed to read or write that file.".into()
    } else if lower.contains("no space left") {
        "The disk is full.".into()
    } else if lower.contains("unknown encoder")
        || lower.contains("encoder") && lower.contains("not found")
    {
        "This copy of FFmpeg can't write that format.".into()
    } else if last.is_empty() {
        "FFmpeg stopped without saying why.".into()
    } else {
        let short: String = last.chars().take(200).collect();
        format!("FFmpeg said: {short}")
    }
}

/// Checks the written file with ffprobe.
pub async fn validate(ffprobe: &Path, path: &Path, target: Target) -> Result<(), ConvertError> {
    let p = probe(ffprobe, path)
        .await
        .map_err(|_| ConvertError::BadOutput("The result can't be opened as audio or video."))?;
    let wants_video = matches!(
        target,
        Target::Mp4 | Target::Webm | Target::Mkv | Target::Gif | Target::NoAudio
    );
    if wants_video && p.video_codec.is_none() {
        return Err(ConvertError::BadOutput("The result has no picture in it."));
    }
    if target.is_audio() && p.audio_codec.is_none() {
        return Err(ConvertError::BadOutput("The result has no sound in it."));
    }
    // GIFs report no duration in some builds; everything else must have one.
    if target != Target::Gif && p.duration.is_none() {
        return Err(ConvertError::BadOutput("The result has no length."));
    }
    let _ = ext_lower(path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn h264_aac() -> Probe {
        Probe {
            duration: Some(60.0),
            video_codec: Some("h264".into()),
            audio_codec: Some("aac".into()),
            width: Some(1920),
            height: Some(1080),
        }
    }

    fn args_of(p: &Plan) -> Vec<String> {
        p.args
            .iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect()
    }

    fn mk(target: Target, o: &Options, probe: &Probe) -> Plan {
        plan(
            target,
            o,
            probe,
            &PathBuf::from("C:\\in.mkv"),
            &PathBuf::from("C:\\out.tmp"),
        )
        .unwrap()
    }

    #[test]
    fn h264_aac_to_mp4_is_copied_not_re_encoded() {
        let p = mk(Target::Mp4, &Options::default(), &h264_aac());
        assert!(p.copy);
        let a = args_of(&p);
        assert!(a.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(!a.contains(&"libx264".to_string()));
    }

    #[test]
    fn any_change_forces_a_re_encode() {
        let o = Options {
            max_height: Some(720),
            ..Options::default()
        };
        let p = mk(Target::Mp4, &o, &h264_aac());
        assert!(!p.copy);
        let a = args_of(&p);
        assert!(a.contains(&"libx264".to_string()));
        let vf = a.iter().position(|x| x == "-vf").unwrap();
        assert!(a[vf + 1].contains("min(720\\,ih)"), "{}", a[vf + 1]);
    }

    #[test]
    fn mkv_copies_any_codecs() {
        let weird = Probe {
            video_codec: Some("mpeg2video".into()),
            audio_codec: Some("mp2".into()),
            duration: Some(5.0),
            ..Probe::default()
        };
        assert!(mk(Target::Mkv, &Options::default(), &weird).copy);
    }

    #[test]
    fn webm_needs_vp9_and_opus_to_copy() {
        assert!(!mk(Target::Webm, &Options::default(), &h264_aac()).copy);
        let vp9 = Probe {
            video_codec: Some("vp9".into()),
            audio_codec: Some("opus".into()),
            duration: Some(5.0),
            ..Probe::default()
        };
        assert!(mk(Target::Webm, &Options::default(), &vp9).copy);
    }

    #[test]
    fn trimming_puts_seek_before_the_input_and_blocks_copy() {
        let o = Options {
            start_secs: Some(5.0),
            end_secs: Some(15.0),
            ..Options::default()
        };
        let p = mk(Target::Mp4, &o, &h264_aac());
        assert!(!p.copy);
        let a = args_of(&p);
        let ss = a.iter().position(|x| x == "-ss").unwrap();
        let i = a.iter().position(|x| x == "-i").unwrap();
        assert!(ss < i);
        assert_eq!(p.expected_secs, Some(10.0));
    }

    #[test]
    fn gif_uses_a_palette_and_defaults_to_12_fps_480_wide() {
        let p = mk(Target::Gif, &Options::default(), &h264_aac());
        let a = args_of(&p);
        let g = a.iter().position(|x| x == "-filter_complex").unwrap();
        assert!(a[g + 1].contains("fps=12"));
        assert!(a[g + 1].contains("min(480\\,iw)"));
        assert!(a[g + 1].contains("palettegen"));
        assert!(a.contains(&"-an".to_string()));
    }

    #[test]
    fn audio_targets_drop_the_picture_and_pick_encoders() {
        let audio_only = Probe {
            audio_codec: Some("flac".into()),
            duration: Some(30.0),
            ..Probe::default()
        };
        for (t, enc) in [
            (Target::Mp3, "libmp3lame"),
            (Target::Flac, "flac"),
            (Target::Wav, "pcm_s16le"),
            (Target::Ogg, "libvorbis"),
            (Target::Opus, "libopus"),
            (Target::M4a, "aac"),
        ] {
            let a = args_of(&mk(t, &Options::default(), &audio_only));
            assert!(a.contains(&"-vn".to_string()), "{t:?}");
            assert!(a.contains(&enc.to_string()), "{t:?} {a:?}");
        }
        let o = Options {
            audio_kbps: Some(128),
            ..Options::default()
        };
        let a = args_of(&mk(Target::Mp3, &o, &audio_only));
        assert!(a.contains(&"128k".to_string()));
    }

    #[test]
    fn aac_to_m4a_is_copied() {
        let aac = Probe {
            audio_codec: Some("aac".into()),
            duration: Some(30.0),
            ..Probe::default()
        };
        assert!(mk(Target::M4a, &Options::default(), &aac).copy);
    }

    #[test]
    fn missing_streams_are_refused_with_a_clear_message() {
        let silent = Probe {
            video_codec: Some("h264".into()),
            duration: Some(5.0),
            ..Probe::default()
        };
        let err = plan(
            Target::Mp3,
            &Options::default(),
            &silent,
            Path::new("C:\\a.mp4"),
            Path::new("C:\\o.mp3"),
        );
        assert!(err.err().unwrap().to_string().contains("no sound"));
        let audio = Probe {
            audio_codec: Some("mp3".into()),
            duration: Some(5.0),
            ..Probe::default()
        };
        let err = plan(
            Target::Mp4,
            &Options::default(),
            &audio,
            Path::new("C:\\a.mp3"),
            Path::new("C:\\o.mp4"),
        );
        assert!(err.err().unwrap().to_string().contains("no video"));
    }

    #[test]
    fn input_and_output_come_last_and_unquoted() {
        let p = mk(Target::Mp4, &Options::default(), &h264_aac());
        let a = args_of(&p);
        assert_eq!(a.last().unwrap(), "C:\\out.tmp");
        assert!(a.contains(&"C:\\in.mkv".to_string()));
        assert!(a.contains(&"-nostdin".to_string()));
    }

    #[test]
    fn probe_json_is_parsed_and_cover_art_is_not_video() {
        let json = r#"{"streams":[
            {"codec_type":"audio","codec_name":"mp3"},
            {"codec_type":"video","codec_name":"mjpeg","width":500,"height":500,"disposition":{"attached_pic":1}}],
            "format":{"duration":"212.5"}}"#;
        let p = parse_probe(json).unwrap();
        assert_eq!(p.video_codec, None);
        assert_eq!(p.audio_codec.as_deref(), Some("mp3"));
        assert_eq!(p.duration, Some(212.5));
        assert!(parse_probe("not json").is_err());
    }

    #[test]
    fn progress_lines_are_read_as_seconds() {
        assert_eq!(parse_progress_line("out_time_us=2500000"), Some(2.5));
        assert_eq!(
            parse_progress_line("out_time_us=-9223372036854775807"),
            Some(0.0)
        );
        assert_eq!(parse_progress_line("progress=continue"), None);
        assert_eq!(parse_progress_line("garbage"), None);
    }

    #[test]
    fn errors_become_plain_sentences() {
        assert!(
            friendly_error("x.mp4: Invalid data found when processing input").contains("damaged")
        );
        assert!(friendly_error("").contains("without saying why"));
    }
}
