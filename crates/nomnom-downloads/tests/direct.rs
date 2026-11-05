use async_trait::async_trait;
use nomnom_core::db::Db;
use nomnom_core::tasks::TaskManager;
use nomnom_downloads::analyze::MediaInfo;
use nomnom_downloads::backend::{DownloadJob, MediaBackend, ProgressFn};
use nomnom_downloads::direct::{Fetched, FileFetcher};
use nomnom_downloads::{DirectRequest, DlError, DownloadManager, EnqueueOutcome};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use url::Url;

struct NoVideo;
#[async_trait]
impl MediaBackend for NoVideo {
    async fn analyze(&self, _: &str, _: bool) -> Result<MediaInfo, DlError> {
        Ok(MediaInfo::default())
    }
    async fn download(&self, _: &DownloadJob, _: ProgressFn<'_>) -> Result<PathBuf, DlError> {
        Err(DlError::Unsupported)
    }
}

/// Pretends to download: writes a small file with the content type chosen per URL.
struct FakeFetcher {
    fail: bool,
    seen: Mutex<Vec<String>>,
}
#[async_trait]
impl FileFetcher for FakeFetcher {
    async fn fetch(
        &self,
        url: &Url,
        staging: &Path,
        stem: &str,
        _: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Result<Fetched, DlError> {
        self.seen.lock().unwrap().push(url.to_string());
        if self.fail {
            return Err(DlError::Failed("The site didn't give the file.".into()));
        }
        std::fs::create_dir_all(staging).unwrap();
        let (ct, ext) = if url.path().ends_with(".mp4") {
            ("video/mp4", "mp4")
        } else {
            ("image/png", "png")
        };
        let path = staging.join(format!("{stem}.{ext}"));
        std::fs::write(&path, b"bytes").unwrap();
        Ok(Fetched {
            path,
            content_type: ct.into(),
        })
    }
}

struct H {
    mgr: DownloadManager,
    fetcher: Arc<FakeFetcher>,
    dest: tempfile::TempDir,
}

fn harness(fail: bool) -> H {
    let fetcher = Arc::new(FakeFetcher {
        fail,
        seen: Mutex::new(vec![]),
    });
    let mgr = DownloadManager::new(
        Arc::new(NoVideo),
        TaskManager::new(),
        Arc::new(Db::open_in_memory().unwrap()),
    )
    .with_fetcher(fetcher.clone());
    H {
        mgr,
        fetcher,
        dest: tempfile::tempdir().unwrap(),
    }
}

fn req(h: &H, media: &[&str], sort: bool) -> DirectRequest {
    DirectRequest {
        source: "https://x.com/someone/status/1789012345".into(),
        media: media.iter().map(|s| s.to_string()).collect(),
        dest_dir: h.dest.path().to_path_buf(),
        sort,
        allow_duplicates: false,
    }
}

async fn all_done(h: &H, n: usize) -> Vec<nomnom_downloads::JobRow> {
    for _ in 0..300 {
        let rows = h.mgr.list(50);
        if rows.len() >= n
            && rows
                .iter()
                .all(|r| matches!(r.status.as_str(), "done" | "failed"))
        {
            return rows;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!(
        "jobs never finished: {:?}",
        h.mgr.list(50).iter().map(|r| &r.status).collect::<Vec<_>>()
    );
}

const A: &str = "https://pbs.twimg.com/media/AAA?format=jpg&name=orig";
const B: &str = "https://pbs.twimg.com/media/BBB?format=jpg&name=orig";
const V: &str = "https://video.twimg.com/ext_tw_video/1/vid/clip.mp4";

#[tokio::test]
async fn pictures_land_in_images_with_the_post_in_the_name() {
    let h = harness(false);
    let out = h.mgr.enqueue_direct(req(&h, &[A, B], true)).unwrap();
    assert!(out
        .iter()
        .all(|o| matches!(o, EnqueueOutcome::Queued { .. })));
    let rows = all_done(&h, 2).await;
    assert!(rows.iter().all(|r| r.status == "done"), "{rows:?}");
    let images = h.dest.path().join("Images");
    let mut names: Vec<_> = std::fs::read_dir(&images)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["X post 1789012345 - 1.png", "X post 1789012345 - 2.png"]
    );
    assert!(!h.dest.path().join(".nomnom-staging").exists());
}

#[tokio::test]
async fn a_video_clip_goes_to_videos() {
    let h = harness(false);
    h.mgr.enqueue_direct(req(&h, &[V], true)).unwrap();
    all_done(&h, 1).await;
    assert!(h
        .dest
        .path()
        .join("Videos")
        .join("X post 1789012345.mp4")
        .exists());
}

#[tokio::test]
async fn with_sorting_off_the_file_stays_in_the_save_folder() {
    let h = harness(false);
    h.mgr.enqueue_direct(req(&h, &[A], false)).unwrap();
    all_done(&h, 1).await;
    assert!(h.dest.path().join("X post 1789012345.png").exists());
}

#[tokio::test]
async fn one_bad_link_queues_nothing_and_nothing_is_fetched() {
    let h = harness(false);
    for bad in [
        "https://evil.example/a.jpg",
        "http://pbs.twimg.com/media/AAA.jpg",
        "file:///C:/Windows/win.ini",
        "https://pbs.twimg.com.evil.example/a.jpg",
    ] {
        assert!(
            h.mgr.enqueue_direct(req(&h, &[A, bad], true)).is_err(),
            "{bad}"
        );
    }
    assert!(h.mgr.list(10).is_empty());
    assert!(h.fetcher.seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_bad_post_link_is_refused() {
    let h = harness(false);
    let mut r = req(&h, &[A], true);
    r.source = "http://127.0.0.1/post".into();
    assert!(h.mgr.enqueue_direct(r).is_err());
}

#[tokio::test]
async fn the_same_picture_in_another_size_is_a_duplicate() {
    let h = harness(false);
    h.mgr.enqueue_direct(req(&h, &[A], true)).unwrap();
    all_done(&h, 1).await;
    let again = h
        .mgr
        .enqueue_direct(req(
            &h,
            &["https://pbs.twimg.com/media/AAA?format=jpg&name=small"],
            true,
        ))
        .unwrap();
    assert!(
        matches!(again[0], EnqueueOutcome::AlreadyDownloaded { .. }),
        "{again:?}"
    );
    assert_eq!(
        h.fetcher.seen.lock().unwrap().len(),
        1,
        "nothing was downloaded twice"
    );
}

#[tokio::test]
async fn a_failed_fetch_is_recorded_for_the_history() {
    let h = harness(true);
    h.mgr.enqueue_direct(req(&h, &[A], true)).unwrap();
    let rows = all_done(&h, 1).await;
    assert_eq!(rows[0].status, "failed");
    assert!(rows[0]
        .error
        .as_deref()
        .unwrap()
        .contains("didn't give the file"));
    assert!(!h.dest.path().join(".nomnom-staging").exists());
}
