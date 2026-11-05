//! Live tests against the real yt-dlp and the network. Run on purpose with:
//!   cargo test -p nomnom-downloads --test live -- --ignored --nocapture --test-threads=1
//! They install yt-dlp and ffmpeg into NOMNOM's own tools folder if missing.

use nomnom_core::db::Db;
use nomnom_core::paths::Paths;
use nomnom_core::tasks::TaskManager;
use nomnom_downloads::{
    Choice, Container, DownloadManager, EnqueueOutcome, EnqueueRequest, Kind, MediaBackend,
    YtDlpBackend,
};
use nomnom_tools::{Tool, Tools};
use std::sync::Arc;
use std::time::Duration;

/// The first video ever uploaded to YouTube: 19 seconds, public.
const URL: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

async fn tools() -> Tools {
    let paths = Paths::resolve().unwrap();
    let tools = Tools::new(paths.tools_dir());
    if tools.locate(Tool::YtDlp).is_none() {
        println!("installing yt-dlp...");
        tools.install_ytdlp(&|_, _| {}).await.unwrap();
    }
    if tools.locate(Tool::Ffmpeg).is_none() {
        println!("installing ffmpeg...");
        tools.install_ffmpeg(&|_, _| {}).await.unwrap();
    }
    println!("yt-dlp {:?}", tools.status(Tool::YtDlp).await.version);
    println!("ffmpeg {:?}", tools.status(Tool::Ffmpeg).await.version);
    tools
}

async fn wait_done(mgr: &DownloadManager, id: &str) -> nomnom_downloads::JobRow {
    for _ in 0..600 {
        if let Some(row) = mgr.list(20).into_iter().find(|r| r.id == id) {
            if matches!(row.status.as_str(), "done" | "failed") {
                return row;
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("download did not finish in time");
}

fn manager(tools: Tools) -> DownloadManager {
    let tasks = TaskManager::new();
    DownloadManager::new(
        Arc::new(YtDlpBackend::new(tools)),
        tasks,
        Arc::new(Db::open_in_memory().unwrap()),
    )
}

fn request(dest: &std::path::Path, choice: Choice) -> EnqueueRequest {
    EnqueueRequest {
        url: URL.into(),
        choice,
        dest_dir: dest.to_path_buf(),
        title: None,
        firefox_cookies: false,
        allow_duplicates: true,
    }
}

#[tokio::test]
#[ignore = "needs network and downloads tools"]
async fn live_analyze_reports_real_formats() {
    let backend = YtDlpBackend::new(tools().await);
    let info = backend.analyze(URL, false).await.unwrap();
    println!("{info:#?}");
    assert!(info.title.to_lowercase().contains("zoo"));
    assert!(
        !info.heights.is_empty(),
        "a normal video offers at least one resolution"
    );
    assert!(
        info.heights.windows(2).all(|w| w[0] > w[1]),
        "highest first, no duplicates"
    );
    assert!(info.duration_secs.is_some());
    assert!(!info.audio_formats.is_empty());
}

#[tokio::test]
#[ignore = "needs network and downloads tools"]
async fn live_download_mp4_and_mp3() {
    let mgr = manager(tools().await);
    let dest = tempfile::tempdir().unwrap();

    let mp4 = Choice {
        kind: Kind::Video,
        container: Container::Mp4,
        height: Some(360),
    };
    let EnqueueOutcome::Queued { job_id } = mgr.enqueue(request(dest.path(), mp4)).unwrap() else {
        panic!()
    };
    let row = wait_done(&mgr, &job_id).await;
    assert_eq!(row.status, "done", "error: {:?}", row.error);
    let path = std::path::PathBuf::from(row.dest_path.unwrap());
    let head = std::fs::read(&path).unwrap();
    println!("video: {} ({} bytes)", path.display(), head.len());
    assert!(head.len() > 10_000);
    assert_eq!(&head[4..8], b"ftyp", "output must really be an MP4");
    assert_eq!(path.extension().unwrap(), "mp4");

    let mp3 = Choice {
        kind: Kind::Audio,
        container: Container::Mp3,
        height: None,
    };
    let EnqueueOutcome::Queued { job_id } = mgr.enqueue(request(dest.path(), mp3)).unwrap() else {
        panic!()
    };
    let row = wait_done(&mgr, &job_id).await;
    assert_eq!(row.status, "done", "error: {:?}", row.error);
    let path = std::path::PathBuf::from(row.dest_path.unwrap());
    let head = std::fs::read(&path).unwrap();
    println!("audio: {} ({} bytes)", path.display(), head.len());
    assert!(head.len() > 10_000);
    assert!(
        head.starts_with(b"ID3") || head[0] == 0xFF,
        "output must really be an MP3"
    );

    assert!(
        !dest.path().join(".nomnom-staging").exists(),
        "staging cleaned up"
    );
}

#[tokio::test]
#[ignore = "needs network and downloads tools"]
async fn live_unsupported_link_gives_friendly_error() {
    let backend = YtDlpBackend::new(tools().await);
    let err = backend
        .analyze("https://example.com/not-a-video", false)
        .await
        .unwrap_err();
    println!("{err}");
    assert!(!err.to_string().is_empty());
}
