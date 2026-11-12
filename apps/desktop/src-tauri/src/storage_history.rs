//! Disk Analyzer history: snapshots of a finished scan, comparing them, and the optional scheduled
//! scans. Scheduled scans only read folder information and write to NomNom's own database; they
//! never change or delete any of your files.

use crate::state::AppState;
use crate::storage_tools::{checked_target, gate, now, write_new};
use nomnom_storage::drives;
use nomnom_storage::report::{self, Format};
use nomnom_storage::scan::State as ScanState;
use nomnom_storage::snapshot::{self, Captured, Comparison, Meta, NewSnapshot, TrendPoint};
use nomnom_storage::{ScanOptions, Session};
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct ScheduleState {
    /// Folders already scanned by the schedule since NomNom started (for "when NomNom starts").
    pub(crate) started: std::sync::Mutex<std::collections::HashSet<String>>,
    pub(crate) busy: std::sync::Mutex<Option<String>>,
    pub(crate) notes: std::sync::Mutex<Vec<RunNote>>,
}

#[derive(Serialize, Clone)]
pub struct RunNote {
    pub root: String,
    pub at: i64,
    pub ok: bool,
    pub message: String,
}

fn db_err(e: nomnom_core::Error) -> String {
    format!("The history couldn't be read or saved: {e}")
}

fn capture_of(s: &Session, depth: u32, big_min: u64) -> Captured {
    let idx = s.index.read().unwrap();
    snapshot::capture(&idx, depth, big_min)
}

fn save_session(state: &AppState, s: &Session, label: &str, source: &str) -> Result<Meta, String> {
    let cfg = state.settings.get().storage;
    let root = snapshot::root_key(&s.root);
    let c = capture_of(
        s,
        u32::from(cfg.snapshot_depth),
        u64::from(cfg.big_file_mb) * 1_048_576,
    );
    let errors = s.stats.errors.load(Ordering::Relaxed);
    let id = state
        .db
        .with(|conn| {
            snapshot::save(
                conn,
                &NewSnapshot {
                    root: &root,
                    label,
                    source,
                    created_at: now(),
                    errors,
                },
                &c,
            )
        })
        .map_err(db_err)?;
    state
        .db
        .with(|conn| snapshot::meta(conn, id))
        .map_err(db_err)?
        .ok_or_else(|| "The snapshot couldn't be read back.".to_string())
}

#[tauri::command]
pub async fn st_snapshot_save(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u64,
    label: String,
) -> Result<Meta, String> {
    let s = gate(&state, id)?;
    if s.state() != ScanState::Done {
        return Err("Only a finished scan can be saved. This one was stopped.".into());
    }
    let label = label.trim().chars().take(80).collect::<String>();
    let a = app.clone();
    let meta = tauri::async_runtime::spawn_blocking(move || {
        let state = a.state::<AppState>();
        save_session(&state, &s, &label, "manual")
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit("storage://snapshots", &meta.root);
    Ok(meta)
}

#[tauri::command]
pub fn st_snapshots(state: State<'_, AppState>, root: String) -> Result<Vec<Meta>, String> {
    let key = snapshot::root_key(Path::new(&root));
    state.db.with(|c| snapshot::list(c, &key)).map_err(db_err)
}

#[tauri::command]
pub fn st_snapshot_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    snapshot: i64,
) -> Result<(), String> {
    let m = state
        .db
        .with(|c| snapshot::meta(c, snapshot))
        .map_err(db_err)?;
    state
        .db
        .with(|c| snapshot::delete(c, snapshot))
        .map_err(db_err)?;
    if let Some(m) = m {
        let _ = app.emit("storage://snapshots", &m.root);
    }
    Ok(())
}

#[derive(Serialize)]
pub struct CompareView {
    pub before: Meta,
    /// `None` when the second side is the scan that is open now.
    pub after: Option<Meta>,
    pub comparison: Comparison,
}

struct Sides {
    before: Meta,
    after: Option<Meta>,
    comparison: Comparison,
}

fn same_folder(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Compares snapshot `a` with snapshot `b`, or with the open scan `id` when `b` is not given.
fn compare_sides(
    state: &AppState,
    a: i64,
    b: Option<i64>,
    id: Option<u64>,
    limit: usize,
) -> Result<Sides, String> {
    let load = |n: i64| -> Result<(Meta, Captured), String> {
        let m = state
            .db
            .with(|c| snapshot::meta(c, n))
            .map_err(db_err)?
            .ok_or("That snapshot no longer exists.")?;
        let c = state
            .db
            .with(|c| snapshot::load(c, n))
            .map_err(db_err)?
            .ok_or("That snapshot no longer exists.")?;
        Ok((m, c))
    };
    let (ma, ca) = load(a)?;
    match b {
        Some(b) => {
            let (mb, cb) = load(b)?;
            if !same_folder(&ma.root, &mb.root) {
                return Err("These two snapshots are of different folders.".into());
            }
            // The older one is "before".
            let ((m1, c1), (m2, c2)) = if (ma.created_at, ma.id) <= (mb.created_at, mb.id) {
                ((ma, ca), (mb, cb))
            } else {
                ((mb, cb), (ma, ca))
            };
            let comparison = snapshot::compare(&c1, &c2, limit);
            Ok(Sides {
                before: m1,
                after: Some(m2),
                comparison,
            })
        }
        None => {
            let id = id.ok_or("Open a scan to compare with.")?;
            let s = gate(state, id)?;
            if s.state() != ScanState::Done {
                return Err("Wait for the scan to finish, then compare.".into());
            }
            if !same_folder(&snapshot::root_key(&s.root), &ma.root) {
                return Err("That snapshot is of a different folder than the open scan.".into());
            }
            let now_c = capture_of(&s, ma.depth, ma.big_min);
            let comparison = snapshot::compare(&ca, &now_c, limit);
            Ok(Sides {
                before: ma,
                after: None,
                comparison,
            })
        }
    }
}

#[tauri::command]
pub async fn st_snapshot_compare(
    app: AppHandle,
    a: i64,
    b: Option<i64>,
    id: Option<u64>,
    limit: usize,
) -> Result<CompareView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let s = compare_sides(&state, a, b, id, limit.clamp(1, 5000))?;
        Ok(CompareView {
            before: s.before,
            after: s.after,
            comparison: s.comparison,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn when_text(t: i64) -> String {
    // The report titles name the moments; keep it simple and unambiguous.
    let days = t.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[tauri::command]
pub async fn st_export_comparison(
    app: AppHandle,
    a: i64,
    b: Option<i64>,
    id: Option<u64>,
    format: Format,
    path: String,
) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<u64, String> {
        let state = app.state::<AppState>();
        let target = checked_target(&state, &path, format.ext())?;
        let s = compare_sides(&state, a, b, id, 100_000)?;
        let before = when_text(s.before.created_at);
        let after = s.after.as_ref().map_or_else(
            || "the scan open now".to_string(),
            |m| when_text(m.created_at),
        );
        let t = report::comparison_table(&s.comparison, &before, &after);
        let out = report::render(&t, format).map_err(|e| e.to_string())?;
        write_new(&target, &out)?;
        Ok(out.len() as u64)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn st_snapshot_trend(
    state: State<'_, AppState>,
    root: String,
    folder: String,
) -> Result<Vec<TrendPoint>, String> {
    let key = snapshot::root_key(Path::new(&root));
    state
        .db
        .with(|c| snapshot::trend(c, &key, folder.trim_matches('\\')))
        .map_err(db_err)
}

// ---- schedule ------------------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ScheduleView {
    /// The folder being scanned right now, if any.
    pub running: Option<String>,
    pub notes: Vec<RunNote>,
}

#[tauri::command]
pub fn st_schedule_status(state: State<'_, AppState>) -> ScheduleView {
    ScheduleView {
        running: state.storage.schedule.busy.lock().unwrap().clone(),
        notes: state.storage.schedule.notes.lock().unwrap().clone(),
    }
}

fn note(state: &AppState, root: &str, ok: bool, message: String) {
    let mut n = state.storage.schedule.notes.lock().unwrap();
    n.push(RunNote {
        root: root.into(),
        at: now(),
        ok,
        message,
    });
    let extra = n.len().saturating_sub(10);
    n.drain(..extra);
}

/// One gentle scan of `root`, saved as a scheduled snapshot. Nothing on disk is changed.
fn scan_and_save(state: &AppState, root: &Path) -> Result<Meta, String> {
    let drive = drives::drive_of(root);
    let opts = ScanOptions {
        workers: 2,
        cluster: drive.as_ref().map(|d| d.cluster).unwrap_or(4096),
        volume_serial: drive.as_ref().map(|d| d.serial).unwrap_or(0),
        ..ScanOptions::default()
    };
    // Not put in the list of open scans, so the window never shows it.
    let id = u64::MAX - state.storage.next.fetch_add(1, Ordering::SeqCst);
    let session: Arc<Session> = Session::start(id, root.to_path_buf(), opts);
    while !session.is_finished() {
        std::thread::sleep(Duration::from_millis(500));
    }
    if session.state() != ScanState::Done {
        return Err("The scan was stopped.".into());
    }
    let meta = save_session(state, &session, "", "scheduled")?;
    let keep = state.settings.get().storage.keep_scheduled as usize;
    let _ = state
        .db
        .with(|c| snapshot::prune(c, &meta.root, "scheduled", keep));
    Ok(meta)
}

fn tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let cfg = state.settings.get().storage;
    if cfg.schedule == "off" || cfg.schedule_roots.is_empty() {
        return;
    }
    // Don't compete with a scan the person started.
    let user_scan_running = state
        .storage
        .sessions
        .lock()
        .unwrap()
        .values()
        .any(|s| !s.is_finished());
    if user_scan_running {
        return;
    }
    for root in &cfg.schedule_roots {
        let key = snapshot::root_key(root);
        let ran = state
            .storage
            .schedule
            .started
            .lock()
            .unwrap()
            .contains(&key.to_lowercase());
        let last = state
            .db
            .with(|c| snapshot::last_time(c, &key, "scheduled"))
            .ok()
            .flatten();
        if !snapshot::schedule_due(&cfg.schedule, last, now(), ran) {
            continue;
        }
        if !root.is_dir() {
            // A drive that isn't plugged in is not an error worth repeating every two minutes.
            if !ran {
                note(
                    &state,
                    &key,
                    false,
                    "The folder wasn't there (is the drive connected?).".into(),
                );
                state
                    .storage
                    .schedule
                    .started
                    .lock()
                    .unwrap()
                    .insert(key.to_lowercase());
            }
            continue;
        }
        *state.storage.schedule.busy.lock().unwrap() = Some(key.clone());
        let _ = app.emit("storage://schedule", &key);
        let result = scan_and_save(&state, root);
        *state.storage.schedule.busy.lock().unwrap() = None;
        state
            .storage
            .schedule
            .started
            .lock()
            .unwrap()
            .insert(key.to_lowercase());
        match result {
            Ok(m) => {
                note(
                    &state,
                    &key,
                    true,
                    format!("{} files, {} bytes.", m.files, m.bytes),
                );
                let _ = app.emit("storage://snapshots", &key);
            }
            Err(e) => note(&state, &key, false, e),
        }
        let _ = app.emit("storage://schedule", &key);
    }
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            let a = app.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || tick(&a)).await;
            tokio::time::sleep(Duration::from_secs(120)).await;
        }
    });
}
