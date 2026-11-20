//! Recording the screen with ffmpeg: the sizes it accepts and the arguments to give it. Starting and
//! stopping the program is the app's job.

/// A rectangle of the screen to record, in pixels from the top left of the whole desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub const MIN_SIDE: u32 = 64;
pub const MAX_SIDE: u32 = 7680;

/// The area made fit for video: even sides (the format needs that), at least `MIN_SIDE`, at most
/// `MAX_SIDE`. `None` when it is too small to record.
pub fn fit(a: Area) -> Option<Area> {
    let even = |n: u32| (n.min(MAX_SIDE)) & !1;
    let (w, h) = (even(a.w), even(a.h));
    (w >= MIN_SIDE && h >= MIN_SIDE).then_some(Area { w, h, ..a })
}

/// The arguments for recording `a` to `out` as MP4. The file is written in pieces so that one cut
/// short by a crash still plays. `max_secs` stops it by itself.
pub fn record_args(a: Area, fps: u32, max_secs: u32, out: &str) -> Vec<String> {
    let fps = fps.clamp(5, 60);
    [
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "gdigrab",
        "-framerate",
        &fps.to_string(),
        "-offset_x",
        &a.x.to_string(),
        "-offset_y",
        &a.y.to_string(),
        "-video_size",
        &format!("{}x{}", a.w, a.h),
        "-i",
        "desktop",
        "-t",
        &max_secs.clamp(1, 4 * 3600).to_string(),
        "-c:v",
        "libx264",
        "-preset",
        "ultrafast",
        "-crf",
        "23",
        "-pix_fmt",
        "yuv420p",
        "-movflags",
        "+frag_keyframe+empty_moov+default_base_moof",
        out,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// The arguments to turn a recording into a small GIF with its own colour palette.
pub fn gif_args(src: &str, dst: &str, fps: u32, width: u32) -> Vec<String> {
    let filter = format!(
        "fps={},scale={}:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4",
        fps.clamp(5, 20),
        width.clamp(160, 1280)
    );
    [
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-i",
        src,
        "-vf",
        &filter,
        "-loop",
        "0",
        dst,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// "Recording 2026-11-20 10-30-00.mp4"
pub fn file_name(stamp: &str, gif: bool) -> String {
    format!("Recording {stamp}.{}", if gif { "gif" } else { "mp4" })
}

/// "02:05" or "1:02:05" for how long a recording has run.
pub fn clock(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_made_even_and_tiny_or_huge_ones_are_handled() {
        assert_eq!(
            fit(Area {
                x: 10,
                y: 20,
                w: 801,
                h: 601
            }),
            Some(Area {
                x: 10,
                y: 20,
                w: 800,
                h: 600
            })
        );
        assert_eq!(
            fit(Area {
                x: 0,
                y: 0,
                w: 63,
                h: 500
            }),
            None
        );
        assert_eq!(
            fit(Area {
                x: 0,
                y: 0,
                w: 500,
                h: 10
            }),
            None
        );
        let big = fit(Area {
            x: 0,
            y: 0,
            w: 20_000,
            h: 20_000,
        })
        .unwrap();
        assert_eq!((big.w, big.h), (MAX_SIDE, MAX_SIDE));
    }

    #[test]
    fn the_recording_arguments_name_the_area_and_end_with_the_file() {
        let args = record_args(
            Area {
                x: -1920,
                y: 0,
                w: 1920,
                h: 1080,
            },
            30,
            600,
            "C:\\v\\out.mp4",
        );
        let text = args.join(" ");
        assert!(text.contains("-offset_x -1920"));
        assert!(text.contains("-video_size 1920x1080"));
        assert!(text.contains("-framerate 30"));
        assert!(text.contains("-t 600"));
        assert!(text.contains("+frag_keyframe+empty_moov"));
        assert_eq!(args.last().unwrap(), "C:\\v\\out.mp4");
        // Odd choices are brought into range.
        assert!(record_args(
            Area {
                x: 0,
                y: 0,
                w: 100,
                h: 100
            },
            500,
            0,
            "o"
        )
        .join(" ")
        .contains("-framerate 60"));
        assert!(record_args(
            Area {
                x: 0,
                y: 0,
                w: 100,
                h: 100
            },
            1,
            0,
            "o"
        )
        .join(" ")
        .contains("-t 1 "));
    }

    #[test]
    fn the_gif_arguments_use_a_palette_and_keep_a_sane_size() {
        let a = gif_args("a.mp4", "a.gif", 12, 640).join(" ");
        assert!(a.contains("palettegen") && a.contains("paletteuse"));
        assert!(a.contains("fps=12,scale=640:-1"));
        assert!(gif_args("a", "b", 100, 5000)
            .join(" ")
            .contains("fps=20,scale=1280"));
    }

    #[test]
    fn names_and_clocks() {
        assert_eq!(
            file_name("2026-11-20 10-30-00", false),
            "Recording 2026-11-20 10-30-00.mp4"
        );
        assert_eq!(file_name("x", true), "Recording x.gif");
        assert_eq!(clock(5), "00:05");
        assert_eq!(clock(125), "02:05");
        assert_eq!(clock(3725), "1:02:05");
    }
}
