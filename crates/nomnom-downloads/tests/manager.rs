use async_trait::async_trait;
use nomnom_core::db::Db;
use nomnom_core::tasks::TaskManager;
use nomnom_downloads::analyze::MediaInfo;
use nomnom_downloads::backend::{DownloadJob, MediaBackend, ProgressFn};
use nomnom_downloads::progress::Progress;
use nomnom_downloads::{Choice, DlError, DownloadManager, EnqueueOutcome, EnqueueRequest};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy)]
enum Behavior {
    /// Writes a file called `Video.mp4` and finishes.
    Succeed,
    Fail,
    /// Writes a partial file, then never finishes.
    Hang,
    /// Hangs on the first run; finishes only if the first run's partial file is still there.
    NeedsPartial,
    /// Takes a moment and records how many ran at once.
    Slow,
}

struct Fake {
    behavior: Behavior,
    running: AtomicUsize,
    peak: AtomicUsize,
}

#[async_trait]
impl MediaBackend for Fake {
    async fn analyze(&self, _: &str, _: bool) -> Result<MediaInfo, DlError> {
        Ok(MediaInfo::default())
    }

    async fn download(
        &self,
        job: &DownloadJob,
        on_progress: ProgressFn<'_>,
    ) -> Result<PathBuf, DlError> {
        std::fs::create_dir_all(&job.staging).unwrap();
        let progress = |d: u64| Progress {
            finished: false,
            downloaded: d,
            total: Some(100),
            speed_bps: Some(10),
            eta_secs: Some(1),
        };
        match self.behavior {
            Behavior::Succeed => {
                on_progress(progress(50));
                let p = job.staging.join("Video.mp4");
                std::fs::write(&p, "data").unwrap();
                Ok(p)
            }
            Behavior::Fail => Err(DlError::Failed("boom".into())),
            Behavior::Hang => {
                std::fs::write(job.staging.join("Video.mp4.part"), "partial").unwrap();
                std::future::pending().await
            }
            Behavior::NeedsPartial => {
                let marker = job.staging.join("Video.mp4.part");
                if marker.exists() {
                    let p = job.staging.join("Video.mp4");
                    std::fs::write(&p, "resumed").unwrap();
                    Ok(p)
                } else {
                    std::fs::write(&marker, "partial").unwrap();
                    std::future::pending().await
                }
            }
            Behavior::Slow => {
                let now = self.running.fetch_add(1, Ordering::SeqCst) + 1;
                self.peak.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(60)).await;
                self.running.fetch_sub(1, Ordering::SeqCst);
                let p = job.staging.join("Slow.mp4");
                std::fs::write(&p, "x").unwrap();
                Ok(p)
            }
        }
    }
}

struct Harness {
    mgr: DownloadManager,
    dest: tempfile::TempDir,
    fake: Arc<Fake>,
    _tasks: TaskManager,
}

fn harness(behavior: Behavior, limit: usize) -> Harness {
    let fake = Arc::new(Fake {
        behavior,
        running: AtomicUsize::new(0),
        peak: AtomicUsize::new(0),
    });
    let tasks = TaskManager::new();
    tasks.set_limit("downloads", limit);
    let mgr = DownloadManager::new(
        fake.clone(),
        tasks.clone(),
        Arc::new(Db::open_in_memory().unwrap()),
    );
    Harness {
        mgr,
        dest: tempfile::tempdir().unwrap(),
        fake,
        _tasks: tasks,
    }
}

fn req(h: &Harness, url: &str) -> EnqueueRequest {
    EnqueueRequest {
        url: url.into(),
        choice: Choice::BEST_VIDEO,
        dest_dir: h.dest.path().to_path_buf(),
        title: None,
        firefox_cookies: false,
        allow_duplicates: false,
    }
}

fn queued(o: EnqueueOutcome) -> String {
    match o {
        EnqueueOutcome::Queued { job_id } => job_id,
        other => panic!("expected Queued, got {other:?}"),
    }
}

async fn wait_status(h: &Harness, id: &str, want: &str) {
    for _ in 0..300 {
        if h.mgr
            .list(50)
            .iter()
            .any(|r| r.id == id && r.status == want)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let got = h
        .mgr
        .list(50)
        .into_iter()
        .find(|r| r.id == id)
        .map(|r| r.status);
    panic!("job never reached {want}; status is {got:?}");
}

fn staging_exists(h: &Harness) -> bool {
    h.dest.path().join(".nomnom-staging").exists()
}

#[tokio::test]
async fn successful_download_lands_in_destination_and_staging_is_removed() {
    let h = harness(Behavior::Succeed, 2);
    let id = queued(h.mgr.enqueue(req(&h, "https://example.com/v1")).unwrap());
    wait_status(&h, &id, "done").await;
    let row = h.mgr.list(10).remove(0);
    assert_eq!(
        std::fs::read_to_string(h.dest.path().join("Video.mp4")).unwrap(),
        "data"
    );
    assert_eq!(row.title, "Video");
    assert_eq!(row.bytes, Some(4));
    assert!(!staging_exists(&h), "staging must be cleaned up");
}

#[tokio::test]
async fn failure_is_recorded_with_message_and_can_be_retried() {
    let h = harness(Behavior::Fail, 2);
    let id = queued(h.mgr.enqueue(req(&h, "https://example.com/v2")).unwrap());
    wait_status(&h, &id, "failed").await;
    assert!(h.mgr.list(10)[0].error.as_deref().unwrap().contains("boom"));
    h.mgr.retry(&id);
    wait_status(&h, &id, "failed").await;
    assert_eq!(
        h.mgr.list(10).len(),
        1,
        "retry reuses the same history entry"
    );
}

#[tokio::test]
async fn existing_file_is_never_overwritten() {
    let h = harness(Behavior::Succeed, 2);
    std::fs::write(h.dest.path().join("Video.mp4"), "precious").unwrap();
    let mut r = req(&h, "https://example.com/v3");
    r.allow_duplicates = true;
    let id = queued(h.mgr.enqueue(r).unwrap());
    wait_status(&h, &id, "done").await;
    assert_eq!(
        std::fs::read_to_string(h.dest.path().join("Video.mp4")).unwrap(),
        "precious"
    );
    assert_eq!(
        std::fs::read_to_string(h.dest.path().join("Video (2).mp4")).unwrap(),
        "data"
    );
}

#[tokio::test]
async fn cancel_stops_the_job_and_removes_partial_files() {
    let h = harness(Behavior::Hang, 2);
    let id = queued(h.mgr.enqueue(req(&h, "https://example.com/v4")).unwrap());
    wait_status(&h, &id, "running").await;
    for _ in 0..100 {
        if staging_exists(&h) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    h.mgr.cancel(&id).await;
    wait_status(&h, &id, "cancelled").await;
    assert!(!staging_exists(&h));
    assert!(!h.dest.path().join("Video.mp4").exists());
}

#[tokio::test]
async fn pause_keeps_partial_data_and_resume_finishes() {
    let h = harness(Behavior::NeedsPartial, 2);
    let id = queued(h.mgr.enqueue(req(&h, "https://example.com/v5")).unwrap());
    wait_status(&h, &id, "running").await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    h.mgr.pause(&id).await;
    wait_status(&h, &id, "paused").await;
    assert!(staging_exists(&h), "paused jobs keep their partial files");
    h.mgr.resume(&id);
    wait_status(&h, &id, "done").await;
    assert_eq!(
        std::fs::read_to_string(h.dest.path().join("Video.mp4")).unwrap(),
        "resumed"
    );
}

#[tokio::test]
async fn same_video_via_different_links_is_deduplicated() {
    let h = harness(Behavior::Hang, 2);
    let first = queued(
        h.mgr
            .enqueue(req(&h, "https://www.youtube.com/watch?v=abc123"))
            .unwrap(),
    );
    let second = h
        .mgr
        .enqueue(req(&h, "https://youtu.be/abc123?si=zzz"))
        .unwrap();
    assert_eq!(second, EnqueueOutcome::AlreadyActive { job_id: first });
    assert_eq!(h.mgr.list(10).len(), 1);
}

#[tokio::test]
async fn completed_download_is_reported_as_duplicate_unless_allowed() {
    let h = harness(Behavior::Succeed, 2);
    let id = queued(h.mgr.enqueue(req(&h, "https://example.com/v6")).unwrap());
    wait_status(&h, &id, "done").await;
    let again = h.mgr.enqueue(req(&h, "https://example.com/v6")).unwrap();
    assert!(matches!(again, EnqueueOutcome::AlreadyDownloaded { .. }));

    let mut r = req(&h, "https://example.com/v6");
    r.allow_duplicates = true;
    assert!(matches!(
        h.mgr.enqueue(r).unwrap(),
        EnqueueOutcome::Queued { .. }
    ));

    std::fs::remove_file(h.dest.path().join("Video.mp4")).unwrap();
    // The first job's file is gone, but the second job is active, so it is still a duplicate.
    assert!(matches!(
        h.mgr.enqueue(req(&h, "https://example.com/v6")).unwrap(),
        EnqueueOutcome::AlreadyActive { .. }
    ));
}

#[tokio::test]
async fn respects_concurrency_limit() {
    let h = harness(Behavior::Slow, 2);
    let ids: Vec<_> = (0..6)
        .map(|i| {
            queued(
                h.mgr
                    .enqueue(req(&h, &format!("https://example.com/s{i}")))
                    .unwrap(),
            )
        })
        .collect();
    for id in &ids {
        wait_status(&h, id, "done").await;
    }
    assert_eq!(h.fake.peak.load(Ordering::SeqCst), 2);
    assert!(
        h.dest.path().join("Slow (6).mp4").exists(),
        "six files with one name all survive"
    );
}

#[tokio::test]
async fn invalid_input_is_rejected_before_queueing() {
    let h = harness(Behavior::Succeed, 2);
    assert!(h.mgr.enqueue(req(&h, "file:///C:/secret.txt")).is_err());
    assert!(h.mgr.enqueue(req(&h, "http://127.0.0.1/x")).is_err());
    let mut r = req(&h, "https://example.com/x");
    r.choice = Choice {
        kind: nomnom_downloads::Kind::Audio,
        container: nomnom_downloads::Container::Mp4,
        height: None,
    };
    assert!(h.mgr.enqueue(r).is_err());
    assert!(h.mgr.list(10).is_empty());
}

#[tokio::test]
async fn jobs_left_running_at_shutdown_become_retryable_after_restart() {
    let h = harness(Behavior::Hang, 2);
    let id = queued(h.mgr.enqueue(req(&h, "https://example.com/v7")).unwrap());
    wait_status(&h, &id, "running").await;
    h.mgr.recover();
    let row = h.mgr.list(10).remove(0);
    assert_eq!(row.status, "failed");
    assert!(row.error.unwrap().contains("closed"));
}

#[tokio::test]
async fn removing_history_never_touches_the_downloaded_file() {
    let h = harness(Behavior::Succeed, 2);
    let id = queued(h.mgr.enqueue(req(&h, "https://example.com/v8")).unwrap());
    wait_status(&h, &id, "done").await;
    h.mgr.remove(&id);
    assert!(h.mgr.list(10).is_empty());
    assert!(h.dest.path().join("Video.mp4").exists());
}
