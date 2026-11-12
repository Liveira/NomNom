//! Disk Analyzer inside the app: drives, scans (start, pause, resume, cancel, rescan one folder),
//! and the questions the window asks of a scan (children, rankings). The scan itself and the index
//! live in `nomnom-storage`; this file only exposes them.

use crate::state::AppState;
use nomnom_storage::analyze::{self, Filter, FolderMode, Group, Page, SortKey};
use nomnom_storage::drives::{self, Drive, DriveKind, Media};
use nomnom_storage::index::NodeId;
use nomnom_storage::scan::{ScanError, State as ScanState};
use nomnom_storage::{NodeView, ScanOptions, Session};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct StorageState {
    sessions: Mutex<HashMap<u64, Arc<Session>>>,
    next: AtomicU64,
}

impl StorageState {
    fn get(&self, id: u64) -> Result<Arc<Session>, String> {
        self.sessions
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| "That scan is no longer open. Scan again.".to_string())
    }
}

#[derive(Serialize, Clone)]
pub struct Progress {
    pub id: u64,
    pub root: String,
    pub state: ScanState,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub errors: u64,
    pub elapsed_ms: u64,
    /// Used space of the drive the scan is on, when the root is on a lettered drive.
    pub drive_used: Option<u64>,
    pub drive_total: Option<u64>,
}

fn progress_of(s: &Session, drive: &Option<Drive>) -> Progress {
    Progress {
        id: s.id,
        root: s.root.to_string_lossy().to_string(),
        state: s.state(),
        files: s.stats.files.load(Ordering::Relaxed),
        dirs: s.stats.dirs.load(Ordering::Relaxed),
        bytes: s.stats.bytes.load(Ordering::Relaxed),
        errors: s.stats.errors.load(Ordering::Relaxed),
        elapsed_ms: s.elapsed().as_millis() as u64,
        drive_used: drive.as_ref().map(|d| d.used),
        drive_total: drive.as_ref().map(|d| d.total),
    }
}

#[tauri::command]
pub async fn st_drives() -> Result<Vec<Drive>, String> {
    tauri::async_runtime::spawn_blocking(drives::list)
        .await
        .map_err(|e| e.to_string())
}

/// Scans keep the disk busy; more workers help an SSD and hurt a spinning disk or a network share.
fn workers_for(drive: &Option<Drive>) -> usize {
    match drive {
        Some(d) if d.kind == DriveKind::Fixed && d.media == Media::Ssd => 8,
        Some(d) if d.kind == DriveKind::Fixed && d.media == Media::Hdd => 2,
        Some(d) if d.kind == DriveKind::Network => 2,
        Some(_) => 4,
        None => 2,
    }
}

fn emit_progress_until_done(app: AppHandle, session: Arc<Session>, drive: Option<Drive>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = app.emit("storage://progress", progress_of(&session, &drive));
            if session.is_finished() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });
}

#[tauri::command]
pub async fn st_scan_start(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
) -> Result<u64, String> {
    let path = PathBuf::from(&root);
    if !path.is_absolute() || nomnom_organizer::pathutil::has_traversal(&path) {
        return Err("Choose a drive or a folder with a complete path.".into());
    }
    if !path.is_dir() {
        return Err("That folder couldn't be found.".into());
    }
    let drive = drives::drive_of(&path);
    let opts = ScanOptions {
        workers: workers_for(&drive),
        cluster: drive.as_ref().map(|d| d.cluster).unwrap_or(4096),
        volume_serial: drive.as_ref().map(|d| d.serial).unwrap_or(0),
        ..ScanOptions::default()
    };
    let id = state.storage.next.fetch_add(1, Ordering::SeqCst) + 1;
    let session = Session::start(id, path, opts);
    state
        .storage
        .sessions
        .lock()
        .unwrap()
        .insert(id, session.clone());
    emit_progress_until_done(app, session, drive);
    Ok(id)
}

#[tauri::command]
pub fn st_scan_pause(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    state.storage.get(id)?.pause();
    Ok(())
}

#[tauri::command]
pub fn st_scan_resume(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    state.storage.get(id)?.resume();
    Ok(())
}

#[tauri::command]
pub fn st_scan_cancel(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    state.storage.get(id)?.cancel();
    Ok(())
}

/// Scan one folder again, keeping the rest of the result.
#[tauri::command]
pub fn st_rescan(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
) -> Result<(), String> {
    let s = state.storage.get(id)?;
    if !s.rescan(node) {
        return Err("A scan is still running, or that item isn't a folder.".into());
    }
    emit_progress_until_done(app, s.clone(), drives::drive_of(&s.root));
    Ok(())
}

/// Forgets a scan and frees its memory.
#[tauri::command]
pub fn st_close(state: State<'_, AppState>, id: u64) {
    if let Some(s) = state.storage.sessions.lock().unwrap().remove(&id) {
        s.cancel();
    }
}

#[tauri::command]
pub fn st_sessions(state: State<'_, AppState>) -> Vec<Progress> {
    let mut v: Vec<Progress> = state
        .storage
        .sessions
        .lock()
        .unwrap()
        .values()
        .map(|s| progress_of(s, &drives::drive_of(&s.root)))
        .collect();
    v.sort_by_key(|p| p.id);
    v
}

#[tauri::command]
pub fn st_progress(state: State<'_, AppState>, id: u64) -> Result<Progress, String> {
    let s = state.storage.get(id)?;
    Ok(progress_of(&s, &drives::drive_of(&s.root)))
}

#[tauri::command]
pub fn st_errors(state: State<'_, AppState>, id: u64) -> Result<Vec<ScanError>, String> {
    Ok(state.storage.get(id)?.errors())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn st_children(
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
    sort: SortKey,
    desc: bool,
    offset: usize,
    limit: usize,
    text: Option<String>,
) -> Result<Page, String> {
    let s = state.storage.get(id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let idx = s.index.read().unwrap();
        if idx.node(node).is_none() {
            return Err("That item isn't in the scan.".to_string());
        }
        Ok(analyze::children_page(
            &idx,
            node,
            sort,
            desc,
            offset,
            limit.min(5000),
            text.as_deref(),
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
pub struct NodeDetail {
    pub node: NodeView,
    pub path: String,
    /// From the scan's root down to the item.
    pub trail: Vec<NodeView>,
}

#[tauri::command]
pub fn st_node(state: State<'_, AppState>, id: u64, node: NodeId) -> Result<NodeDetail, String> {
    let s = state.storage.get(id)?;
    let idx = s.index.read().unwrap();
    if idx.node(node).is_none() {
        return Err("That item isn't in the scan.".into());
    }
    Ok(NodeDetail {
        node: idx.view(node),
        path: idx.path_of(node).to_string_lossy().to_string(),
        trail: idx
            .ancestors(node)
            .into_iter()
            .map(|a| idx.view(a))
            .collect(),
    })
}

#[tauri::command]
pub async fn st_top_files(
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
    n: usize,
    filter: Filter,
) -> Result<Vec<NodeView>, String> {
    let s = state.storage.get(id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let idx = s.index.read().unwrap();
        if idx.node(node).is_none() {
            return Err("That item isn't in the scan.".to_string());
        }
        Ok(analyze::top_files(&idx, node, n.clamp(1, 100_000), &filter))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn st_top_folders(
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
    n: usize,
    mode: FolderMode,
    min_size: Option<u64>,
) -> Result<Vec<NodeView>, String> {
    let s = state.storage.get(id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let idx = s.index.read().unwrap();
        if idx.node(node).is_none() {
            return Err("That item isn't in the scan.".to_string());
        }
        Ok(analyze::top_folders(
            &idx,
            node,
            n.clamp(1, 100_000),
            mode,
            min_size.unwrap_or(0),
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
pub struct Stats {
    pub extensions: Vec<Group>,
    pub categories: Vec<Group>,
}

#[tauri::command]
pub async fn st_stats(state: State<'_, AppState>, id: u64, node: NodeId) -> Result<Stats, String> {
    let s = state.storage.get(id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let idx = s.index.read().unwrap();
        if idx.node(node).is_none() {
            return Err("That item isn't in the scan.".to_string());
        }
        Ok(Stats {
            extensions: analyze::by_extension(&idx, node, 200),
            categories: analyze::by_category(&idx, node),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Full paths for a list of items (the rankings show where each one is).
#[tauri::command]
pub fn st_paths(
    state: State<'_, AppState>,
    id: u64,
    nodes: Vec<NodeId>,
) -> Result<Vec<String>, String> {
    let s = state.storage.get(id)?;
    let idx = s.index.read().unwrap();
    Ok(nodes
        .into_iter()
        .map(|n| {
            if idx.node(n).is_some() {
                idx.path_of(n).to_string_lossy().to_string()
            } else {
                String::new()
            }
        })
        .collect())
}
