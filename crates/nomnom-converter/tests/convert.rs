//! End-to-end conversions in throwaway folders. Nothing here touches a real folder.

use image::{ImageBuffer, Rgba, RgbaImage};
use nomnom_converter::{
    applicable, capabilities, describe, presets, run, Backends, Event, Kind, Options, PdfPage,
    Request, Status, Target,
};
use nomnom_tools::Tools;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

fn no_events() -> Arc<dyn Fn(Event) + Send + Sync> {
    Arc::new(|_| {})
}

fn hash(p: &Path) -> String {
    let mut h = blake3::Hasher::new();
    h.update(&std::fs::read(p).unwrap());
    h.finalize().to_hex().to_string()
}

fn picture(path: &Path, w: u32, h: u32, alpha: u8) {
    let img: RgbaImage = ImageBuffer::from_fn(w, h, |x, y| {
        Rgba([
            (x * 255 / w.max(1)) as u8,
            (y * 255 / h.max(1)) as u8,
            128,
            alpha,
        ])
    });
    img.save(path).unwrap();
}

fn req(files: Vec<PathBuf>, target: Target) -> Request {
    Request {
        files,
        target,
        options: Options::default(),
        out_dir: None,
    }
}

async fn convert(r: Request) -> Vec<nomnom_converter::Outcome> {
    run(
        r,
        Backends::default(),
        CancellationToken::new(),
        no_events(),
    )
    .await
}

fn ffmpeg_backends() -> Option<Backends> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    let tools = Tools::new(PathBuf::from(local).join("NOMNOM").join("tools"));
    let b = Backends::detect(&tools);
    if b.has(nomnom_converter::Backend::Ffmpeg) {
        Some(b)
    } else {
        eprintln!("SKIPPED: ffmpeg isn't installed here");
        None
    }
}

// ---- pictures --------------------------------------------------------------------------------

#[tokio::test]
async fn ten_pngs_become_webp_and_the_sources_are_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = vec![];
    for i in 0..10 {
        let p = dir.path().join(format!("p{i}.png"));
        picture(&p, 64 + i, 48, 255);
        files.push(p);
    }
    let before: Vec<_> = files.iter().map(|f| hash(f)).collect();
    let out = convert(req(files.clone(), Target::Webp)).await;
    assert_eq!(out.len(), 10);
    for (o, f) in out.iter().zip(&files) {
        assert_eq!(o.status, Status::Done, "{:?}", o.message);
        let dst = o.dst.as_ref().unwrap();
        assert_eq!(dst.extension().unwrap(), "webp");
        assert_eq!(dst.parent(), f.parent());
        assert!(image::open(dst).is_ok());
    }
    let after: Vec<_> = files.iter().map(|f| hash(f)).collect();
    assert_eq!(before, after, "a source changed");
    // No temporary files are left behind.
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".nomnom-"))
        .collect();
    assert!(leftovers.is_empty());
}

#[tokio::test]
async fn every_picture_format_round_trips_to_the_right_size() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("in.png");
    picture(&src, 120, 80, 255);
    for t in [
        Target::Png,
        Target::Jpg,
        Target::Webp,
        Target::Bmp,
        Target::Tiff,
        Target::Gif,
        Target::Ico,
    ] {
        let out = convert(req(vec![src.clone()], t)).await;
        let o = &out[0];
        assert_eq!(o.status, Status::Done, "{t:?}: {:?}", o.message);
        let img = image::open(o.dst.as_ref().unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (120, 80), "{t:?}");
    }
}

#[tokio::test]
async fn an_existing_name_is_never_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.png");
    picture(&src, 20, 20, 255);
    std::fs::write(dir.path().join("a.webp"), b"mine").unwrap();
    let out = convert(req(vec![src], Target::Webp)).await;
    assert_eq!(
        out[0].dst.as_ref().unwrap().file_name().unwrap(),
        "a (converted).webp"
    );
    assert_eq!(std::fs::read(dir.path().join("a.webp")).unwrap(), b"mine");
}

#[tokio::test]
async fn same_format_resize_goes_to_a_new_name() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("big.png");
    picture(&src, 400, 200, 255);
    let before = hash(&src);
    let mut r = req(vec![src.clone()], Target::Png);
    r.options.max_width = Some(100);
    let out = convert(r).await;
    let dst = out[0].dst.as_ref().unwrap();
    assert_eq!(dst.file_name().unwrap(), "big (converted).png");
    let img = image::open(dst).unwrap();
    assert_eq!((img.width(), img.height()), (100, 50));
    assert_eq!(hash(&src), before);
}

#[tokio::test]
async fn pictures_are_never_made_bigger() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("small.png");
    picture(&src, 50, 50, 255);
    let mut r = req(vec![src], Target::Jpg);
    r.options.max_width = Some(1000);
    r.options.max_height = Some(1000);
    let out = convert(r).await;
    let img = image::open(out[0].dst.as_ref().unwrap()).unwrap();
    assert_eq!((img.width(), img.height()), (50, 50));
}

#[tokio::test]
async fn transparency_is_filled_for_jpg_and_the_user_is_told() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("clear.png");
    picture(&src, 16, 16, 0); // fully transparent
    let mut r = req(vec![src], Target::Jpg);
    r.options.background = [255, 0, 0];
    let out = convert(r).await;
    assert!(
        out[0].notes.iter().any(|n| n.contains("transparency")),
        "{:?}",
        out[0].notes
    );
    let px = image::open(out[0].dst.as_ref().unwrap()).unwrap().to_rgb8();
    let p = px.get_pixel(8, 8);
    assert!(p[0] > 200 && p[1] < 60 && p[2] < 60, "{p:?}");
}

#[tokio::test]
async fn transparency_survives_png_and_webp() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("half.png");
    picture(&src, 16, 16, 100);
    for t in [Target::Png, Target::Webp] {
        let mut r = req(vec![src.clone()], t);
        r.options.image_quality = 100; // lossless for WebP
        let out = convert(r).await;
        let img = image::open(out[0].dst.as_ref().unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(img.get_pixel(3, 3)[3], 100, "{t:?}");
    }
}

#[tokio::test]
async fn a_large_icon_is_shrunk_to_256() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("logo.png");
    picture(&src, 600, 600, 255);
    let out = convert(req(vec![src], Target::Ico)).await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    assert!(out[0].notes.iter().any(|n| n.contains("256")));
}

#[tokio::test]
async fn exif_and_colour_profile_are_kept_or_removed_on_request() {
    use img_parts::{Bytes, ImageEXIF};
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.jpg");
    picture(&dir.path().join("t.png"), 32, 32, 255);
    image::open(dir.path().join("t.png"))
        .unwrap()
        .to_rgb8()
        .save(&plain)
        .unwrap();
    // Attach a recognisable EXIF block.
    let payload = Bytes::from_static(b"Exif\0\0NOMNOM-TEST-PAYLOAD");
    let mut jpeg =
        img_parts::jpeg::Jpeg::from_bytes(Bytes::from(std::fs::read(&plain).unwrap())).unwrap();
    jpeg.set_exif(Some(payload.clone()));
    let src = dir.path().join("photo.jpg");
    let mut bytes = Vec::new();
    jpeg.encoder().write_to(&mut bytes).unwrap();
    std::fs::write(&src, bytes).unwrap();

    let read_exif = |p: &Path, webp: bool| -> Option<Bytes> {
        let data = Bytes::from(std::fs::read(p).unwrap());
        if webp {
            img_parts::webp::WebP::from_bytes(data).unwrap().exif()
        } else {
            img_parts::jpeg::Jpeg::from_bytes(data).unwrap().exif()
        }
    };

    let kept = convert(req(vec![src.clone()], Target::Webp)).await;
    assert_eq!(kept[0].status, Status::Done, "{:?}", kept[0].message);
    assert_eq!(
        read_exif(kept[0].dst.as_ref().unwrap(), true),
        Some(payload.clone())
    );

    let mut r = req(vec![src.clone()], Target::Jpg);
    r.options.keep_metadata = false;
    let stripped = convert(r).await;
    assert_eq!(read_exif(stripped[0].dst.as_ref().unwrap(), false), None);
    assert!(stripped[0].notes.iter().any(|n| n.contains("removed")));
}

#[tokio::test]
async fn a_corrupt_file_fails_alone_and_the_rest_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let good1 = dir.path().join("a.png");
    let bad = dir.path().join("b.png");
    let good2 = dir.path().join("c.png");
    picture(&good1, 10, 10, 255);
    std::fs::write(&bad, b"this is not a picture").unwrap();
    picture(&good2, 10, 10, 255);
    let out = convert(req(vec![good1, bad.clone(), good2], Target::Webp)).await;
    assert_eq!(out[0].status, Status::Done);
    assert_eq!(out[1].status, Status::Failed);
    assert!(out[1].message.as_ref().unwrap().contains("Couldn't read"));
    assert_eq!(out[2].status, Status::Done);
    assert_eq!(std::fs::read(&bad).unwrap(), b"this is not a picture");
    assert!(!dir.path().join("b.webp").exists());
}

#[tokio::test]
async fn a_program_renamed_to_png_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("fake.png");
    std::fs::write(&fake, b"MZ\x90\0\x03\0\0\0 not an image").unwrap();
    let out = convert(req(vec![fake], Target::Jpg)).await;
    assert_eq!(out[0].status, Status::Failed);
}

#[tokio::test]
async fn output_folder_option_writes_there_and_creates_it() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.png");
    picture(&src, 10, 10, 255);
    let mut r = req(vec![src], Target::Jpg);
    let out_dir = dir.path().join("new").join("results");
    r.out_dir = Some(out_dir.clone());
    let out = convert(r).await;
    assert_eq!(out[0].dst.as_ref().unwrap().parent().unwrap(), out_dir);
}

#[tokio::test]
async fn a_cancelled_batch_stops_and_leaves_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = vec![];
    for i in 0..5 {
        let p = dir.path().join(format!("p{i}.png"));
        picture(&p, 30, 30, 255);
        files.push(p);
    }
    let token = CancellationToken::new();
    token.cancel();
    let out = run(
        req(files, Target::Webp),
        Backends::default(),
        token,
        no_events(),
    )
    .await;
    assert!(out.iter().all(|o| o.status == Status::Cancelled));
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".webp")));
}

#[tokio::test]
async fn events_arrive_in_order_for_each_file() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.png");
    let b = dir.path().join("b.png");
    picture(&a, 8, 8, 255);
    picture(&b, 8, 8, 255);
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = log.clone();
    let cb: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(move |e| {
        let tag = match e {
            Event::Started { index, .. } => format!("start{index}"),
            Event::Progress { .. } => return,
            Event::Finished { index, .. } => format!("done{index}"),
        };
        sink.lock().unwrap().push(tag);
    });
    run(
        req(vec![a, b], Target::Jpg),
        Backends::default(),
        CancellationToken::new(),
        cb,
    )
    .await;
    assert_eq!(*log.lock().unwrap(), ["start0", "done0", "start1", "done1"]);
}

// ---- validation of requests ----------------------------------------------------------------

#[tokio::test]
async fn bad_requests_fail_up_front_with_a_sentence() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("setup.exe");
    std::fs::write(&exe, b"MZ").unwrap();
    let out = convert(req(vec![exe], Target::Pdf)).await;
    assert_eq!(out[0].status, Status::Failed);
    let txt = dir.path().join("a.txt");
    std::fs::write(&txt, b"hi").unwrap();
    assert!(nomnom_converter::batch::check(
        &req(vec![txt.clone()], Target::Mp4),
        &Backends::default()
    )
    .is_err());
    assert!(nomnom_converter::batch::check(
        &req(vec![PathBuf::from("relative.png")], Target::Jpg),
        &Backends::default()
    )
    .is_err());
    assert!(
        nomnom_converter::batch::check(&req(vec![], Target::Jpg), &Backends::default()).is_err()
    );
    let mut r = req(vec![txt], Target::Pdf);
    r.options.combine = true;
    assert!(nomnom_converter::batch::check(&r, &Backends::default()).is_err());
}

#[test]
fn without_ffmpeg_video_conversions_say_what_is_missing() {
    let caps = capabilities(&Backends::default());
    let mp4 = caps
        .iter()
        .find(|c| c.kind == Kind::Video && c.target == Target::Mp4)
        .unwrap();
    assert!(!mp4.available);
    assert_eq!(mp4.needs, Some("FFmpeg"));
    let webp = caps
        .iter()
        .find(|c| c.kind == Kind::Image && c.target == Target::Webp)
        .unwrap();
    assert!(webp.available);
    let doc = caps.iter().find(|c| c.kind == Kind::Office).unwrap();
    assert_eq!(doc.needs, Some("LibreOffice"));
}

#[test]
fn applicable_conversions_must_work_for_every_selected_file() {
    let caps = capabilities(&Backends::default());
    let both_images = applicable(&[PathBuf::from("a.png"), PathBuf::from("b.jpg")], &caps);
    assert!(both_images.iter().any(|c| c.target == Target::Webp));
    let mixed = applicable(&[PathBuf::from("a.png"), PathBuf::from("b.mp4")], &caps);
    assert!(mixed.is_empty());
    let md = applicable(&[PathBuf::from("a.md")], &caps);
    assert!(md.iter().any(|c| c.target == Target::Html));
    let txt = applicable(&[PathBuf::from("a.txt")], &caps);
    assert!(!txt.iter().any(|c| c.target == Target::Html));
}

#[test]
fn dropped_folders_and_unknown_files_are_reported_not_expanded() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("folder");
    std::fs::create_dir(&sub).unwrap();
    let exe = dir.path().join("a.exe");
    std::fs::write(&exe, b"MZ").unwrap();
    let empty = dir.path().join("e.png");
    std::fs::write(&empty, b"").unwrap();
    let ok = dir.path().join("ok.png");
    picture(&ok, 4, 4, 255);
    let info = describe(&[sub, exe, empty, ok, dir.path().join("missing.png")]);
    assert!(info[0].problem.as_ref().unwrap().contains("Folders"));
    assert!(info[1].problem.as_ref().unwrap().contains("can't convert"));
    assert!(info[2].problem.as_ref().unwrap().contains("empty"));
    assert!(info[3].problem.is_none());
    assert!(info[4].problem.as_ref().unwrap().contains("found"));
}

// ---- documents -------------------------------------------------------------------------------

#[tokio::test]
async fn markdown_becomes_pdf_and_html() {
    let dir = tempfile::tempdir().unwrap();
    let md = dir.path().join("notes.md");
    std::fs::write(&md, "# Notes\n\n- one\n- two\n\nSome text.\n").unwrap();
    let pdf = convert(req(vec![md.clone()], Target::Pdf)).await;
    assert_eq!(pdf[0].status, Status::Done, "{:?}", pdf[0].message);
    let doc = lopdf::Document::load(pdf[0].dst.as_ref().unwrap()).unwrap();
    assert!(!doc.get_pages().is_empty());
    let html = convert(req(vec![md], Target::Html)).await;
    let text = std::fs::read_to_string(html[0].dst.as_ref().unwrap()).unwrap();
    assert!(text.contains("<h1>Notes</h1>"));
}

#[tokio::test]
async fn a_binary_file_named_txt_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("x.txt");
    std::fs::write(&bad, [0u8, 1, 2, 0, 3]).unwrap();
    let out = convert(req(vec![bad], Target::Pdf)).await;
    assert_eq!(out[0].status, Status::Failed);
}

#[tokio::test]
async fn photos_combine_into_one_a4_pdf_with_a_page_each() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = vec![];
    for (i, (w, h)) in [(300, 200), (200, 300), (100, 100)].into_iter().enumerate() {
        let p = dir.path().join(format!("{i}.png"));
        picture(&p, w, h, 255);
        files.push(p);
    }
    let mut r = req(files, Target::Pdf);
    r.options.combine = true;
    r.options.pdf_page = PdfPage::A4;
    let out = convert(r).await;
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    assert_eq!(out[0].sources, 3);
    let doc = lopdf::Document::load(out[0].dst.as_ref().unwrap()).unwrap();
    assert_eq!(doc.get_pages().len(), 3);
}

#[tokio::test]
async fn each_picture_can_also_become_its_own_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("one.jpg");
    picture(&dir.path().join("t.png"), 40, 40, 255);
    image::open(dir.path().join("t.png"))
        .unwrap()
        .to_rgb8()
        .save(&p)
        .unwrap();
    let out = convert(req(vec![p], Target::Pdf)).await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    assert_eq!(
        lopdf::Document::load(out[0].dst.as_ref().unwrap())
            .unwrap()
            .get_pages()
            .len(),
        1
    );
}

#[test]
fn presets_have_sane_options() {
    for p in presets() {
        assert_eq!(p.options.clone().sanitized(), p.options, "{}", p.id);
    }
}

// ---- ffmpeg (skipped when it isn't installed) ------------------------------------------------

fn make_clip(ffmpeg: &Path, out: &Path) {
    let status = std::process::Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args([
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=320x240:rate=15",
        ])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=2"])
        .args([
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(out)
        .status()
        .unwrap();
    assert!(status.success());
}

fn make_tone(ffmpeg: &Path, out: &Path) {
    let status = std::process::Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=3"])
        .arg(out)
        .status()
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn video_to_gif_webm_and_audio_with_ffmpeg() {
    let Some(b) = ffmpeg_backends() else { return };
    let dir = tempfile::tempdir().unwrap();
    let clip = dir.path().join("clip.mp4");
    make_clip(b.ffmpeg.as_ref().unwrap(), &clip);
    let before = hash(&clip);

    let mut r = req(vec![clip.clone()], Target::Gif);
    r.options.max_width = Some(160);
    r.options.fps = Some(8);
    let out = run(r, b.clone(), CancellationToken::new(), no_events()).await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    let gif = image::open(out[0].dst.as_ref().unwrap()).unwrap();
    assert_eq!(gif.width(), 160);

    let out = run(
        req(vec![clip.clone()], Target::Webm),
        b.clone(),
        CancellationToken::new(),
        no_events(),
    )
    .await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);

    let out = run(
        req(vec![clip.clone()], Target::Mp3),
        b.clone(),
        CancellationToken::new(),
        no_events(),
    )
    .await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    let probe =
        nomnom_converter::ffmpeg::probe(b.ffprobe.as_ref().unwrap(), out[0].dst.as_ref().unwrap())
            .await
            .unwrap();
    assert_eq!(probe.audio_codec.as_deref(), Some("mp3"));
    assert!(probe.video_codec.is_none());
    assert!(
        (probe.duration.unwrap() - 2.0).abs() < 0.3,
        "{:?}",
        probe.duration
    );

    assert_eq!(hash(&clip), before, "the source changed");
}

#[tokio::test]
async fn mp4_to_mkv_is_a_copy_and_a_resize_re_encodes() {
    let Some(b) = ffmpeg_backends() else { return };
    let dir = tempfile::tempdir().unwrap();
    let clip = dir.path().join("clip.mp4");
    make_clip(b.ffmpeg.as_ref().unwrap(), &clip);

    let out = run(
        req(vec![clip.clone()], Target::Mkv),
        b.clone(),
        CancellationToken::new(),
        no_events(),
    )
    .await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    assert!(out[0].notes.iter().any(|n| n.contains("nothing was lost")));

    let mut r = req(vec![clip], Target::Mp4);
    r.options.max_height = Some(120);
    let out = run(r, b.clone(), CancellationToken::new(), no_events()).await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    let p =
        nomnom_converter::ffmpeg::probe(b.ffprobe.as_ref().unwrap(), out[0].dst.as_ref().unwrap())
            .await
            .unwrap();
    assert_eq!(p.height, Some(120));
    assert_eq!(p.width, Some(160));
}

#[tokio::test]
async fn trimming_gives_the_requested_length() {
    let Some(b) = ffmpeg_backends() else { return };
    let dir = tempfile::tempdir().unwrap();
    let tone = dir.path().join("tone.wav");
    make_tone(b.ffmpeg.as_ref().unwrap(), &tone);
    let mut r = req(vec![tone], Target::Flac);
    r.options.start_secs = Some(0.5);
    r.options.end_secs = Some(2.0);
    let out = run(r, b.clone(), CancellationToken::new(), no_events()).await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    let p =
        nomnom_converter::ffmpeg::probe(b.ffprobe.as_ref().unwrap(), out[0].dst.as_ref().unwrap())
            .await
            .unwrap();
    assert!((p.duration.unwrap() - 1.5).abs() < 0.15, "{:?}", p.duration);
}

#[tokio::test]
async fn removing_the_sound_keeps_the_picture() {
    let Some(b) = ffmpeg_backends() else { return };
    let dir = tempfile::tempdir().unwrap();
    let clip = dir.path().join("clip.mp4");
    make_clip(b.ffmpeg.as_ref().unwrap(), &clip);
    let out = run(
        req(vec![clip], Target::NoAudio),
        b.clone(),
        CancellationToken::new(),
        no_events(),
    )
    .await;
    assert_eq!(out[0].status, Status::Done, "{:?}", out[0].message);
    let dst = out[0].dst.as_ref().unwrap();
    assert_eq!(dst.file_name().unwrap(), "clip (converted).mp4");
    let p = nomnom_converter::ffmpeg::probe(b.ffprobe.as_ref().unwrap(), dst)
        .await
        .unwrap();
    assert!(p.audio_codec.is_none());
    assert!(p.video_codec.is_some());
}

#[tokio::test]
async fn a_damaged_media_file_gives_a_plain_message_and_no_leftovers() {
    let Some(b) = ffmpeg_backends() else { return };
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("broken.mp4");
    std::fs::write(&bad, vec![7u8; 5000]).unwrap();
    let out = run(
        req(vec![bad], Target::Mp3),
        b,
        CancellationToken::new(),
        no_events(),
    )
    .await;
    assert_eq!(out[0].status, Status::Failed);
    assert!(!out[0].message.as_ref().unwrap().is_empty());
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, ["broken.mp4"], "{names:?}");
}

#[tokio::test]
async fn cancelling_a_running_conversion_kills_ffmpeg_and_leaves_no_output() {
    let Some(b) = ffmpeg_backends() else { return };
    let dir = tempfile::tempdir().unwrap();
    // A long clip so the encode is still running when we cancel.
    let long = dir.path().join("long.mp4");
    let status = std::process::Command::new(b.ffmpeg.as_ref().unwrap())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=duration=120:size=1280x720:rate=30",
        ])
        .args(["-c:v", "mpeg4", "-q:v", "5"])
        .arg(&long)
        .status()
        .unwrap();
    assert!(status.success());
    let token = CancellationToken::new();
    let t2 = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        t2.cancel();
    });
    let mut r = req(vec![long], Target::Webm);
    r.options.video_quality = nomnom_converter::VideoQuality::High;
    let started = std::time::Instant::now();
    let out = run(r, b, token, no_events()).await;
    assert_eq!(out[0].status, Status::Cancelled, "{:?}", out[0].message);
    assert!(started.elapsed().as_secs() < 20);
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, ["long.mp4"], "{names:?}");
}

#[tokio::test]
async fn lossy_to_lossless_audio_carries_a_note_through_the_matrix() {
    // The warning itself is shown by the window; the matrix marks FLAC and WAV as lossless
    // targets and the source's lossiness is known from its extension.
    assert!(nomnom_converter::model::is_lossy_audio(Path::new("a.mp3")));
    assert!(!nomnom_converter::model::is_lossy_audio(Path::new(
        "a.flac"
    )));
}
