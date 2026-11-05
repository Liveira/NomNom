//! The file organizer inside the app: scan and preview, run, undo, history, and the automatic mode.
//! The window never sends paths to move. It sends the id of a plan the app made and kept itself,
//! so nothing the interface says can make NOMNOM move a file that wasn't previewed.

use crate::state::AppState;
use nomnom_organizer::exec::{execute, recover, undo, ExecOptions, Report, UndoReport};
use nomnom_organizer::journal::{BatchRow, Journal};
use nomnom_organizer::model::{ConflictPolicy, Options, Plan, SkipCode};
use nomnom_organizer::pathutil::{is_drive_root, is_inside, is_same};
use nomnom_organizer::scan::{plan, plan_files};
use nomnom_organizer::settle::{Settler, Verdict};
use nomnom_organizer::winfs;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

const PLAN_TTL: Duration = Duration::from_secs(15 * 60);
const SHOWN_LIMIT: usize = 3000;

#[derive(Default)]
pub struct OrganizerState {
    plans: Mutex<HashMap<String, (Plan, Instant)>>,
    stop: AtomicBool,
    /// Only one thing may move files at a time.
    run_lock: Mutex<()>,
    auto: Mutex<Option<Arc<AtomicBool>>>,
}

/// A destination is fine unless it's a whole drive or inside a system or program folder.
pub fn dest_is_acceptable(dest: &Path, zones: &[PathBuf]) -> bool {
    dest.is_absolute() && !is_drive_root(dest) && !zones.iter().any(|z| is_inside(dest, z))
}

pub fn dest_root(state: &AppState) -> PathBuf {
    state
        .settings
        .get()
        .organizer
        .dest_root
        .unwrap_or_else(|| state.paths.documents.join("Files"))
}

/// Where NomNom Downloader should save, and whether to sort into Videos/Audio/Images inside it.
/// When the downloader is linked to the organizer its files go straight to the organizer's folders.
pub fn download_target(state: &AppState) -> (PathBuf, bool) {
    let s = state.settings.get();
    if s.organizer.link_downloads {
        let root = dest_root(state);
        let (zones, _) = state.paths.protected_folders();
        if dest_is_acceptable(&root, &zones) {
            return (root, true);
        }
    }
    let dir = s
        .downloads
        .output_dir
        .unwrap_or_else(|| state.paths.downloads.clone());
    (dir, s.downloads.sort_into_folders)
}

pub fn options(state: &AppState, source: PathBuf, depth: u8) -> Options {
    let s = state.settings.get().organizer;
    let (zones, exact) = state.paths.protected_folders();
    let mut o = Options::new(source, dest_root(state));
    o.max_depth = u32::from(depth.min(3));
    o.conflict = if s.conflict == "keep_both" {
        ConflictPolicy::KeepBoth
    } else {
        ConflictPolicy::Skip
    };
    o.protected_zones = zones;
    o.protected_exact = exact;
    o
}

#[derive(Serialize)]
pub struct MoveOut {
    src: String,
    dst: String,
    category: String,
    cross_volume: bool,
}

#[derive(Serialize)]
pub struct SkipOut {
    path: String,
    is_dir: bool,
    code: SkipCode,
    message: String,
}

#[derive(Serialize)]
pub struct ScanOut {
    plan_id: String,
    source: String,
    dest_root: String,
    moves: Vec<MoveOut>,
    skips: Vec<SkipOut>,
    total_moves: usize,
    total_skips: usize,
    cross_volume_count: usize,
}

#[tauri::command]
pub fn org_defaults(state: State<'_, AppState>) -> serde_json::Value {
    let s = state.settings.get().organizer;
    serde_json::json!({
        "source": state.paths.downloads,
        "dest_root": dest_root(&state),
        "look_inside_folders": s.look_inside_folders,
    })
}

#[tauri::command]
pub async fn org_scan(
    state: State<'_, AppState>,
    source: Option<String>,
    depth: Option<u8>,
) -> Result<ScanOut, String> {
    let source = source
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| state.paths.downloads.clone());
    let depth = depth.unwrap_or_else(|| state.settings.get().organizer.look_inside_folders);
    let opts = options(&state, source, depth);
    let found = tauri::async_runtime::spawn_blocking(move || plan(&opts))
        .await
        .map_err(|e| e.to_string())?;
    let found = found.map_err(|e| e.to_string())?;

    let out = ScanOut {
        plan_id: uuid::Uuid::new_v4().to_string(),
        source: found.source.to_string_lossy().into_owned(),
        dest_root: found.dest_root.to_string_lossy().into_owned(),
        total_moves: found.moves.len(),
        total_skips: found.skips.len(),
        cross_volume_count: found.cross_volume_count(),
        moves: found
            .moves
            .iter()
            .take(SHOWN_LIMIT)
            .map(|m| MoveOut {
                src: m.src.to_string_lossy().into_owned(),
                dst: m.dst.to_string_lossy().into_owned(),
                category: m.category.clone(),
                cross_volume: m.cross_volume,
            })
            .collect(),
        skips: found
            .skips
            .iter()
            .take(SHOWN_LIMIT)
            .map(|s| SkipOut {
                path: s.path.to_string_lossy().into_owned(),
                is_dir: s.is_dir,
                code: s.code,
                message: s.message.clone(),
            })
            .collect(),
    };
    let mut plans = state.organizer.plans.lock().unwrap();
    plans.retain(|_, (_, at)| at.elapsed() < PLAN_TTL);
    plans.insert(out.plan_id.clone(), (found, Instant::now()));
    Ok(out)
}

#[tauri::command]
pub async fn org_execute(
    app: AppHandle,
    state: State<'_, AppState>,
    plan_id: String,
    allow_cross_volume: bool,
) -> Result<Report, String> {
    let (plan, at) = state
        .organizer
        .plans
        .lock()
        .unwrap()
        .remove(&plan_id)
        .ok_or("That preview has expired. Scan again.")?;
    if at.elapsed() > PLAN_TTL {
        return Err(
            "That preview is too old. Scan again so it matches what's in the folder now.".into(),
        );
    }
    let journal = state.journal.clone();
    state.organizer.stop.store(false, Ordering::SeqCst);
    let worker = app.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let st = worker.state::<AppState>();
        let _one_at_a_time = st.organizer.run_lock.lock().unwrap();
        let stop = || st.organizer.stop.load(Ordering::SeqCst);
        execute(
            &journal,
            &plan,
            "manual",
            &ExecOptions { allow_cross_volume },
            &stop,
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let _ = app.emit("organizer://changed", ());
    Ok(report)
}

#[tauri::command]
pub fn org_cancel(state: State<'_, AppState>) {
    state.organizer.stop.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn org_undo(
    app: AppHandle,
    state: State<'_, AppState>,
    batch_id: String,
    allow_cross_volume: bool,
) -> Result<UndoReport, String> {
    let journal = state.journal.clone();
    let worker = app.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let st = worker.state::<AppState>();
        let _one_at_a_time = st.organizer.run_lock.lock().unwrap();
        undo(&journal, &batch_id, &ExecOptions { allow_cross_volume })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let _ = app.emit("organizer://changed", ());
    Ok(report)
}

#[tauri::command]
pub fn org_history(state: State<'_, AppState>) -> Result<Vec<BatchRow>, String> {
    state.journal.batches(50).map_err(|e| e.to_string())
}

pub fn recover_at_startup(journal: &Journal) {
    match recover(journal) {
        Ok(notes) if !notes.is_empty() => {
            for n in notes {
                tracing::warn!(path = %n.path.display(), "organizer recovery: {}", n.message);
            }
        }
        Ok(_) => {}
        Err(e) => tracing::error!("organizer recovery failed: {e}"),
    }
}

// ---- automatic mode ---------------------------------------------------------------------

/// Starts or stops the folder watcher to match the setting. The watcher only exists while the
/// setting is on, and it watches the one folder, not its subfolders.
pub fn apply_auto(app: &AppHandle) {
    let state = app.state::<AppState>();
    let want = state.settings.get().organizer.auto_enabled;
    let mut slot = state.organizer.auto.lock().unwrap();
    if let Some(running) = slot.take() {
        running.store(false, Ordering::SeqCst);
    }
    if want {
        let running = Arc::new(AtomicBool::new(true));
        *slot = Some(running.clone());
        let app = app.clone();
        std::thread::spawn(move || auto_loop(app, running));
    }
}

fn auto_loop(app: AppHandle, running: Arc<AtomicBool>) {
    use notify::{EventKind, RecursiveMode, Watcher};
    let source = app.state::<AppState>().paths.downloads.clone();
    let (tx, rx) = mpsc::channel();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            let _ = tx.send(ev);
        }
    });
    let mut watcher = match watcher {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("couldn't start the folder watcher: {e}");
            return;
        }
    };
    if let Err(e) = watcher.watch(&source, RecursiveMode::NonRecursive) {
        tracing::error!("couldn't watch {}: {e}", source.display());
        return;
    }
    tracing::info!("automatic organizing is watching {}", source.display());

    let mut settler = Settler::new(Duration::from_secs(5));
    let mut tries: HashMap<PathBuf, u8> = HashMap::new();
    while running.load(Ordering::SeqCst) {
        let mut events = Vec::new();
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(ev) => events.push(ev),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        let now = Instant::now();
        for ev in events {
            if !matches!(ev.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                continue;
            }
            for path in ev.paths {
                let direct = path.parent().is_some_and(|p| is_same(p, &source));
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !direct || nomnom_organizer::safety::is_in_progress_name(&name) {
                    continue;
                }
                if let Ok(i) = winfs::info(&path) {
                    if !i.is_dir() {
                        settler.observe(&path, nomnom_organizer::exec::fingerprint_of(&path), now);
                    }
                }
            }
        }
        let mut ready = Vec::new();
        for path in settler.paths() {
            if settler.observe(&path, nomnom_organizer::exec::fingerprint_of(&path), now)
                == Verdict::Ready
            {
                ready.push(path);
            }
        }
        if !ready.is_empty() {
            organize_new(&app, &source, ready, &mut settler, &mut tries);
        }
    }
    tracing::info!("automatic organizing stopped");
}

/// Organizes files that have finished arriving, as one journaled (and undoable) batch.
fn organize_new(
    app: &AppHandle,
    source: &Path,
    files: Vec<PathBuf>,
    settler: &mut Settler,
    tries: &mut HashMap<PathBuf, u8>,
) {
    let state = app.state::<AppState>();
    let mut opts = options(&state, source.to_path_buf(), 0);
    opts.settle_secs = 10;
    let planned = match plan_files(&opts, &files) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("automatic organizing skipped: {e}");
            return;
        }
    };
    // Files that were only "not yet" get another look a little later (a few times, then give up).
    for skip in &planned.skips {
        if matches!(skip.code, SkipCode::RecentlyChanged | SkipCode::Locked) {
            let n = tries.entry(skip.path.clone()).or_insert(0);
            *n += 1;
            if *n <= 12 {
                settler.observe(
                    &skip.path,
                    nomnom_organizer::exec::fingerprint_of(&skip.path),
                    Instant::now(),
                );
            }
        }
    }
    for m in &planned.moves {
        tries.remove(&m.src);
    }
    if planned.moves.is_empty() {
        return;
    }
    let _one_at_a_time = state.organizer.run_lock.lock().unwrap();
    match execute(
        &state.journal,
        &planned,
        "auto",
        &ExecOptions {
            allow_cross_volume: false,
        },
        &|| false,
    ) {
        Ok(report) if report.moved > 0 => {
            tracing::info!(
                moved = report.moved,
                "automatic organizing moved new downloads"
            );
            let _ = app.emit("organizer://changed", ());
            if state.settings.get().downloads.notify {
                let body = if report.moved == 1 {
                    "Moved 1 new download to your organized folders. You can undo it in NomNom."
                        .to_string()
                } else {
                    format!("Moved {} new downloads to your organized folders. You can undo this in NomNom.", report.moved)
                };
                let _ = app
                    .notification()
                    .builder()
                    .title("Files organized")
                    .body(body)
                    .show();
            }
        }
        Ok(_) => {}
        Err(e) => tracing::error!("automatic organizing failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destinations_inside_system_folders_or_on_a_bare_drive_are_not_acceptable() {
        let zones = vec![
            PathBuf::from(r"C:\Windows"),
            PathBuf::from(r"C:\Program Files"),
        ];
        assert!(dest_is_acceptable(
            Path::new(r"C:\Users\me\Documents\Files"),
            &zones
        ));
        assert!(!dest_is_acceptable(
            Path::new(r"C:\Windows\System32\Files"),
            &zones
        ));
        assert!(!dest_is_acceptable(
            Path::new(r"C:\program files\x"),
            &zones
        ));
        assert!(!dest_is_acceptable(Path::new(r"C:\"), &zones));
        assert!(
            !dest_is_acceptable(Path::new(r"Files"), &zones),
            "a relative path"
        );
    }
}
