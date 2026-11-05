//! The download queue. Job records live in SQLite (so history and retry survive restarts);
//! running work goes through the shared `TaskManager` for concurrency limits and cancellation.

use crate::backend::{DownloadJob, MediaBackend};
use crate::error::DlError;
use crate::files::finalize;
use crate::format::{Choice, Container, Kind};
use crate::progress::Progress;
use crate::url_check::{canonical_key, site_label, validate_url};
use nomnom_core::db::Db;
use nomnom_core::tasks::{TaskId, TaskManager, TaskState};
use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use uuid::Uuid;

pub const CATEGORY: &str = "downloads";
pub const STAGING_DIR: &str = ".nomnom-staging";

#[derive(Debug, Clone)]
pub struct EnqueueRequest {
    pub url: String,
    pub choice: Choice,
    pub dest_dir: PathBuf,
    /// Known from analysis; shown until the real file name is known.
    pub title: Option<String>,
    pub firefox_cookies: bool,
    pub allow_duplicates: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum EnqueueOutcome {
    Queued { job_id: String },
    AlreadyActive { job_id: String },
    AlreadyDownloaded { job_id: String, path: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct JobRow {
    pub id: String,
    pub url: String,
    pub title: String,
    pub source: String,
    pub kind: String,
    pub format: String,
    pub quality: String,
    pub status: String,
    pub dest_path: Option<String>,
    pub bytes: Option<u64>,
    pub error: Option<String>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobEvent {
    pub job_id: String,
    pub status: String,
    pub title: String,
    pub error: Option<String>,
}

struct Inner {
    backend: Arc<dyn MediaBackend>,
    tasks: TaskManager,
    db: Arc<Db>,
    running: Mutex<HashMap<String, TaskId>>,
    events: broadcast::Sender<JobEvent>,
}

#[derive(Clone)]
pub struct DownloadManager {
    inner: Arc<Inner>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

const COLS: &str =
    "id,url,title,source,kind,format,quality,status,dest_path,bytes,error,created_at,finished_at";

fn map_row(r: &Row<'_>) -> rusqlite::Result<JobRow> {
    Ok(JobRow {
        id: r.get(0)?,
        url: r.get(1)?,
        title: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
        source: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
        kind: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
        format: r.get::<_, Option<String>>(5)?.unwrap_or_default(),
        quality: r.get::<_, Option<String>>(6)?.unwrap_or_default(),
        status: r.get(7)?,
        dest_path: r.get(8)?,
        bytes: r.get::<_, Option<i64>>(9)?.map(|b| b as u64),
        error: r.get(10)?,
        created_at: r.get(11)?,
        finished_at: r.get(12)?,
    })
}

struct Spec {
    url: String,
    choice: Choice,
    dest_dir: PathBuf,
    cookies: bool,
    title: String,
}

fn is_active(status: &str) -> bool {
    matches!(status, "queued" | "running" | "paused")
}

impl DownloadManager {
    pub fn new(backend: Arc<dyn MediaBackend>, tasks: TaskManager, db: Arc<Db>) -> Self {
        let (events, _) = broadcast::channel(64);
        DownloadManager {
            inner: Arc::new(Inner {
                backend,
                tasks,
                db,
                running: Mutex::new(HashMap::new()),
                events,
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<JobEvent> {
        self.inner.events.subscribe()
    }

    /// Anything that was queued or running when NOMNOM last closed can't continue: mark it so the
    /// user can retry it. Paused jobs keep their partial files and stay resumable.
    pub fn recover(&self) {
        let _ = self.inner.db.with(|c| {
            c.execute(
                "UPDATE downloads SET status='failed', error='NOMNOM closed before this finished.', finished_at=?1 \
                 WHERE status IN ('queued','running')",
                params![now()],
            )
        });
    }

    pub fn analyze_backend(&self) -> Arc<dyn MediaBackend> {
        self.inner.backend.clone()
    }

    pub fn list(&self, limit: u32) -> Vec<JobRow> {
        self.inner
            .db
            .with(|c| {
                let mut stmt = c.prepare(&format!(
                    "SELECT {COLS} FROM downloads ORDER BY created_at DESC, rowid DESC LIMIT ?1"
                ))?;
                let rows = stmt
                    .query_map(params![limit], map_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .unwrap_or_default()
    }

    fn row(&self, id: &str) -> Option<JobRow> {
        self.inner
            .db
            .with(|c| {
                c.query_row(
                    &format!("SELECT {COLS} FROM downloads WHERE id=?1"),
                    params![id],
                    map_row,
                )
                .optional()
            })
            .ok()
            .flatten()
    }

    pub fn enqueue(&self, req: EnqueueRequest) -> Result<EnqueueOutcome, DlError> {
        let url = validate_url(&req.url)?;
        req.choice.validate().map_err(DlError::InvalidUrl)?;
        let key = canonical_key(&url);

        let existing: Option<(String, String, Option<String>)> = self
            .inner
            .db
            .with(|c| {
                c.query_row(
                    "SELECT id,status,dest_path FROM downloads WHERE canonical_key=?1 AND status IN ('queued','running','paused','done') \
                     ORDER BY created_at DESC, rowid DESC LIMIT 1",
                    params![key],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
            })
            .map_err(|e| DlError::Io(e.to_string()))?;
        if let Some((id, status, path)) = existing {
            if is_active(&status) {
                return Ok(EnqueueOutcome::AlreadyActive { job_id: id });
            }
            if !req.allow_duplicates {
                if let Some(p) = path.filter(|p| Path::new(p).is_file()) {
                    return Ok(EnqueueOutcome::AlreadyDownloaded {
                        job_id: id,
                        path: p,
                    });
                }
            }
        }

        let id = Uuid::new_v4().to_string();
        let height = req
            .choice
            .height
            .map_or("best".to_string(), |h| h.to_string());
        let kind = if req.choice.kind == Kind::Video {
            "video"
        } else {
            "audio"
        };
        self.inner
            .db
            .with(|c| {
                c.execute(
                    "INSERT INTO downloads(id,url,canonical_key,title,source,kind,format,quality,status,dest_dir,cookies,created_at) \
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'queued',?9,?10,?11)",
                    params![
                        id,
                        url.as_str(),
                        key,
                        req.title.clone().unwrap_or_else(|| url.as_str().to_string()),
                        site_label(&url),
                        kind,
                        req.choice.container.as_str(),
                        height,
                        req.dest_dir.to_string_lossy(),
                        req.firefox_cookies as i64,
                        now()
                    ],
                )
            })
            .map_err(|e| DlError::Io(e.to_string()))?;
        self.emit(&id);
        self.start(&id);
        Ok(EnqueueOutcome::Queued { job_id: id })
    }

    fn load_spec(&self, id: &str) -> Option<Spec> {
        self.inner
            .db
            .with(|c| {
                c.query_row(
                    "SELECT url,kind,format,quality,dest_dir,cookies,title FROM downloads WHERE id=?1",
                    params![id],
                    |r| {
                        let kind: String = r.get(1)?;
                        let format: String = r.get(2)?;
                        let quality: String = r.get(3)?;
                        let dest: Option<String> = r.get(4)?;
                        Ok(Spec {
                            url: r.get(0)?,
                            choice: Choice {
                                kind: if kind == "audio" { Kind::Audio } else { Kind::Video },
                                container: Container::parse(&format).unwrap_or(Container::Original),
                                height: quality.parse().ok(),
                            },
                            dest_dir: PathBuf::from(dest.unwrap_or_default()),
                            cookies: r.get::<_, i64>(5)? != 0,
                            title: r.get::<_, Option<String>>(6)?.unwrap_or_default(),
                        })
                    },
                )
                .optional()
            })
            .ok()
            .flatten()
    }

    fn set_status(&self, id: &str, status: &str, error: Option<&str>) {
        let finished = matches!(status, "done" | "failed" | "cancelled").then(now);
        let _ = self.inner.db.with(|c| {
            c.execute(
                "UPDATE downloads SET status=?2, error=?3, finished_at=?4 WHERE id=?1",
                params![id, status, error, finished],
            )
        });
        self.emit(id);
    }

    fn emit(&self, id: &str) {
        if let Some(row) = self.row(id) {
            let _ = self.inner.events.send(JobEvent {
                job_id: row.id,
                status: row.status,
                title: row.title,
                error: row.error,
            });
        }
    }

    fn staging_for(spec: &Spec, id: &str) -> PathBuf {
        spec.dest_dir.join(STAGING_DIR).join(id)
    }

    fn start(&self, id: &str) {
        let Some(spec) = self.load_spec(id) else {
            return;
        };
        let staging = Self::staging_for(&spec, id);
        let this = self.clone();
        let job_id = id.to_string();
        let title = if spec.title.is_empty() {
            spec.url.clone()
        } else {
            spec.title.clone()
        };

        let task_id = self.inner.tasks.submit_ref(
            "downloads",
            CATEGORY,
            &title,
            Some(id.to_string()),
            move |handle| {
                let this2 = this.clone();
                async move {
                    this2.set_status(&job_id, "running", None);
                    if let Err(e) = std::fs::create_dir_all(&staging) {
                        let msg = format!(
                            "Couldn't create a working folder in the download location. {e}"
                        );
                        this2.set_status(&job_id, "failed", Some(&msg));
                        return Err(msg);
                    }
                    if let Some(parent) = staging.parent() {
                        nomnom_core::fs::hide(parent);
                    }

                    let finished_streams = AtomicU32::new(0);
                    let expected = if spec.choice.kind == Kind::Video {
                        2.0
                    } else {
                        1.0
                    };
                    let on_progress = |p: Progress| {
                        if p.finished {
                            finished_streams.fetch_add(1, Ordering::Relaxed);
                        }
                        let done = finished_streams.load(Ordering::Relaxed) as f32;
                        let overall = p.fraction().map(|f| {
                            ((done + if p.finished { 0.0 } else { f }) / expected).min(0.99)
                        });
                        handle.progress(overall, p.speed_bps, p.eta_secs);
                    };
                    let job = DownloadJob {
                        url: spec.url.clone(),
                        choice: spec.choice,
                        staging: staging.clone(),
                        firefox_cookies: spec.cookies,
                    };

                    let outcome = match this2.inner.backend.download(&job, &on_progress).await {
                        Ok(staged) => this2.place(&job_id, &staged, &spec.dest_dir),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = std::fs::remove_dir_all(&staging);
                    this2.remove_staging_root_if_empty(&spec.dest_dir);
                    match outcome {
                        Ok(()) => Ok(()),
                        Err(msg) => {
                            this2.set_status(&job_id, "failed", Some(&msg));
                            Err(msg)
                        }
                    }
                }
            },
        );
        self.inner
            .running
            .lock()
            .unwrap()
            .insert(id.to_string(), task_id);
    }

    /// Moves the finished file out of staging without overwriting anything.
    fn place(&self, id: &str, staged: &Path, dest_dir: &Path) -> Result<(), String> {
        let name = staged
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download")
            .to_string();
        let final_path = finalize(staged, dest_dir, &name)
            .map_err(|e| format!("Couldn't save the file. {e}"))?;
        let bytes = std::fs::metadata(&final_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        let title = final_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let _ = self.inner.db.with(|c| {
            c.execute(
                "UPDATE downloads SET status='done', dest_path=?2, bytes=?3, title=?4, error=NULL, finished_at=?5 WHERE id=?1",
                params![id, final_path.to_string_lossy(), bytes, title, now()],
            )
        });
        self.emit(id);
        Ok(())
    }

    fn remove_staging_root_if_empty(&self, dest_dir: &Path) {
        // Only succeeds when no other job is using it; never removes anything with content.
        let _ = std::fs::remove_dir(dest_dir.join(STAGING_DIR));
    }

    pub async fn cancel(&self, id: &str) {
        self.stop(id, "cancelled", false).await;
    }

    /// Stops the process but keeps partial data so `resume` continues where it left off. If the
    /// source can't resume, the download starts over.
    pub async fn pause(&self, id: &str) {
        self.stop(id, "paused", true).await;
    }

    async fn stop(&self, id: &str, final_status: &str, keep_staging: bool) {
        let task = self.inner.running.lock().unwrap().remove(id);
        if let Some(task) = task {
            self.inner.tasks.cancel(task);
            // Wait for the process tree to be gone before touching its files.
            for _ in 0..200 {
                match self.inner.tasks.get(task).map(|t| t.state) {
                    Some(TaskState::Queued | TaskState::Running) => {
                        tokio::time::sleep(Duration::from_millis(50)).await
                    }
                    _ => break,
                }
            }
        }
        let Some(row) = self.row(id) else { return };
        // The job may have finished just before the stop took effect; don't undo that.
        if matches!(row.status.as_str(), "done" | "failed") {
            return;
        }
        if !keep_staging {
            if let Some(spec) = self.load_spec(id) {
                let _ = std::fs::remove_dir_all(Self::staging_for(&spec, id));
                self.remove_staging_root_if_empty(&spec.dest_dir);
            }
        }
        self.set_status(id, final_status, None);
    }

    pub fn resume(&self, id: &str) {
        if self.row(id).is_some_and(|r| r.status == "paused") {
            self.set_status(id, "queued", None);
            self.start(id);
        }
    }

    pub fn retry(&self, id: &str) {
        if self
            .row(id)
            .is_some_and(|r| matches!(r.status.as_str(), "failed" | "cancelled"))
        {
            self.set_status(id, "queued", None);
            self.start(id);
        }
    }

    /// Removes a finished job from history. Never deletes the downloaded file.
    pub fn remove(&self, id: &str) {
        if self
            .row(id)
            .is_some_and(|r| !matches!(r.status.as_str(), "queued" | "running"))
        {
            let _ = self
                .inner
                .db
                .with(|c| c.execute("DELETE FROM downloads WHERE id=?1", params![id]));
        }
    }

    pub fn clear_history(&self) {
        let _ = self.inner.db.with(|c| {
            c.execute(
                "DELETE FROM downloads WHERE status IN ('done','failed','cancelled')",
                [],
            )
        });
    }
}
