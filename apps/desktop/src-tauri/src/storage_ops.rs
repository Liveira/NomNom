//! Disk Analyzer file operations for the window: plan, confirm, run. The window can only run a plan
//! this app made and kept; it never sends paths to delete. Every item is checked against the disk
//! again at run time (see `nomnom_storage::ops`).

use crate::state::AppState;
use nomnom_storage::index::NodeId;
use nomnom_storage::ops::{self, Kind, Plan, Report, RunOptions, Status};
use nomnom_storage::protect::Protected;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

const PLAN_TTL: Duration = Duration::from_secs(15 * 60);

pub struct StoredPlan {
    session: u64,
    plan: Plan,
    made: Instant,
}

pub(crate) fn protected(state: &AppState) -> Protected {
    let (zones, exact) = state.paths.protected_folders();
    Protected::from_zones(zones, exact)
}

#[derive(Serialize)]
pub struct ItemView {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub files: u64,
    pub warn: bool,
    pub new_name: Option<String>,
    /// For a duplicate: the copy that stays.
    pub keep: Option<String>,
}

#[derive(Serialize)]
pub struct PlanView {
    pub plan_id: String,
    pub kind: Kind,
    pub item_count: usize,
    /// The first items, for the confirmation window.
    pub items: Vec<ItemView>,
    pub total_size: u64,
    pub total_files: u64,
    pub refused: Vec<ops::Refusal>,
    pub warnings: Vec<String>,
    pub needs_typed: bool,
    pub dest: Option<String>,
    pub new_name: Option<String>,
}

#[tauri::command]
pub fn st_op_plan(
    state: State<'_, AppState>,
    id: u64,
    kind: Kind,
    nodes: Vec<NodeId>,
    dest: Option<String>,
    new_name: Option<String>,
) -> Result<PlanView, String> {
    let s = state.storage.get(id)?;
    if !s.is_finished() {
        return Err("Wait for the scan to finish before changing files.".into());
    }
    if nodes.is_empty() {
        return Err("Select something first.".into());
    }
    let dest_path = dest.filter(|d| !d.is_empty()).map(PathBuf::from);
    let plan = {
        let idx = s.index.read().unwrap();
        ops::plan(
            &idx,
            kind,
            &nodes,
            dest_path.as_deref(),
            new_name.as_deref(),
            &protected(&state),
        )
    };
    Ok(store_plan(&state, id, kind, plan))
}

/// Keeps a plan so it can be run later, and describes it for the window.
pub(crate) fn store_plan(state: &AppState, session: u64, kind: Kind, plan: Plan) -> PlanView {
    let plan_id = uuid::Uuid::new_v4().to_string();
    let view = PlanView {
        plan_id: plan_id.clone(),
        kind,
        item_count: plan.items.len(),
        items: plan
            .items
            .iter()
            .take(200)
            .map(|i| ItemView {
                path: i.path.to_string_lossy().to_string(),
                is_dir: i.is_dir,
                size: i.size,
                files: i.files,
                warn: i.warn,
                new_name: i.new_name.clone(),
                keep: i
                    .keep
                    .as_ref()
                    .map(|k| k.path.to_string_lossy().to_string()),
            })
            .collect(),
        total_size: plan.total_size,
        total_files: plan.total_files,
        refused: plan.refused.clone(),
        warnings: plan.warnings.clone(),
        needs_typed: plan.needs_typed,
        dest: plan.dest.as_ref().map(|d| d.to_string_lossy().to_string()),
        new_name: plan.new_name.clone(),
    };
    let mut plans = state.storage.plans.lock().unwrap();
    plans.retain(|_, p| p.made.elapsed() < PLAN_TTL);
    plans.insert(
        plan_id,
        StoredPlan {
            session,
            plan,
            made: Instant::now(),
        },
    );
    view
}

#[derive(Serialize, Clone)]
struct OpEvent {
    done: usize,
    total: usize,
    current: String,
}

/// Runs a plan. `confirmed` must be true for a permanent delete, and `typed` must match the item
/// count when the plan asked for it.
#[tauri::command]
pub async fn st_op_run(
    app: AppHandle,
    state: State<'_, AppState>,
    plan_id: String,
    confirmed: bool,
    typed: Option<String>,
    include_readonly: bool,
) -> Result<Report, String> {
    // Validate plan and session BEFORE consuming the plan or setting `op_running`, so a mistyped
    // confirmation or expired session never destroys the preview or deadlocks future operations.
    let (session_id, expired, needs_check) = {
        let plans = state.storage.plans.lock().unwrap();
        let stored = plans
            .get(&plan_id)
            .ok_or("That preview has expired. Start again.")?;
        let expired = stored.made.elapsed() > PLAN_TTL;
        let check = if stored.plan.kind == Kind::Delete {
            if !confirmed {
                Err("Permanent deletes need your confirmation.".to_string())
            } else if stored.plan.needs_typed
                && typed.as_deref().map(str::trim)
                    != Some(stored.plan.items.len().to_string().as_str())
            {
                Err(format!("Type {} to confirm.", stored.plan.items.len()))
            } else {
                Ok(())
            }
        } else {
            Ok(())
        };
        (stored.session, expired, check)
    };
    if expired {
        state.storage.plans.lock().unwrap().remove(&plan_id);
        return Err("That preview is too old. Start again so it matches what is on disk.".into());
    }
    needs_check?;
    let session = state.storage.get(session_id)?;
    let stored = state
        .storage
        .plans
        .lock()
        .unwrap()
        .remove(&plan_id)
        .ok_or("That preview has expired. Start again.")?;
    let plan = stored.plan;
    if plan.items.is_empty() {
        return Err("There is nothing to do.".into());
    }
    if state.storage.op_running.swap(true, Ordering::SeqCst) {
        return Err("Another file operation is still running.".into());
    }
    state.storage.op_cancel.store(false, Ordering::SeqCst);
    let worker = app.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let st = tauri::Manager::state::<AppState>(&worker);
        let emit = worker.clone();
        let report = ops::run(
            &plan,
            RunOptions { include_readonly },
            &st.storage.op_cancel,
            &move |p| {
                let _ = emit.emit(
                    "storage://op",
                    OpEvent {
                        done: p.done,
                        total: p.total,
                        current: p.current,
                    },
                );
            },
        );
        update_index(&session, &plan, &report);
        st.storage.op_running.store(false, Ordering::SeqCst);
        report
    })
    .await
    .map_err(|e| {
        state.storage.op_running.store(false, Ordering::SeqCst);
        e.to_string()
    })?;
    let _ = app.emit("storage://changed", stored.session);
    Ok(report)
}

/// Brings the scan in line with what really happened, without scanning everything again.
fn update_index(session: &std::sync::Arc<nomnom_storage::Session>, plan: &Plan, report: &Report) {
    let mut rescan: Vec<NodeId> = Vec::new();
    {
        let mut idx = session.index.write().unwrap();
        for (item, res) in plan.items.iter().zip(&report.results) {
            match (plan.kind, res.status) {
                (Kind::Recycle | Kind::Delete | Kind::Move, Status::Done) => {
                    idx.remove_node(item.node)
                }
                (Kind::Rename, Status::Done) => {
                    if let Some(n) = &plan.new_name {
                        idx.rename_node(item.node, n);
                    }
                }
                (Kind::BatchRename, Status::Done) => {
                    if let Some(n) = &item.new_name {
                        idx.rename_node(item.node, n);
                    }
                }
                (Kind::Compress, Status::Done) => {
                    if item.is_dir {
                        rescan.push(item.node);
                    } else {
                        let new_alloc =
                            nomnom_storage::scan::compressed_size(&item.path).unwrap_or(item.size);
                        idx.update_file_alloc(
                            item.node,
                            new_alloc,
                            nomnom_storage::index::F_COMPRESSED,
                            0,
                            0x800,
                            0,
                        );
                    }
                }
                (Kind::Decompress, Status::Done) => {
                    if item.is_dir {
                        rescan.push(item.node);
                    } else {
                        let cluster = nomnom_storage::drives::drive_of(&item.path)
                            .map(|d| d.cluster)
                            .unwrap_or(4096);
                        let new_alloc = nomnom_storage::scan::compressed_size(&item.path)
                            .unwrap_or_else(|| nomnom_storage::scan::round_up(item.size, cluster));
                        idx.update_file_alloc(
                            item.node,
                            new_alloc,
                            0,
                            nomnom_storage::index::F_COMPRESSED,
                            0,
                            0x800,
                        );
                    }
                }
                (Kind::Hardlink, Status::Done) => {
                    idx.update_file_alloc(
                        item.node,
                        0,
                        nomnom_storage::index::F_HARDLINK_DUP,
                        0,
                        0,
                        0,
                    );
                }
                // Part of a folder may be gone; look again at that folder only.
                (_, Status::Failed) if item.is_dir => rescan.push(item.node),
                _ => {}
            }
        }
        if matches!(plan.kind, Kind::Copy | Kind::Move | Kind::Zip)
            && report.count(Status::Done) > 0
        {
            if let Some(d) = plan.dest.as_ref().and_then(|d| idx.resolve_path(d)) {
                rescan.push(d);
            }
        }
    }
    for node in rescan {
        if session.rescan(node) {
            let t = Instant::now();
            while !session.is_finished() && t.elapsed() < Duration::from_secs(600) {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

#[tauri::command]
pub fn st_op_cancel(state: State<'_, AppState>) {
    state.storage.op_cancel.store(true, Ordering::SeqCst);
}

/// Opens an item with its program, or shows it in Explorer. Only items of an open scan, and only
/// if they are still there.
#[tauri::command]
pub fn st_open(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
    reveal: bool,
) -> Result<(), String> {
    let s = state.storage.get(id)?;
    let path = {
        let idx = s.index.read().unwrap();
        idx.node(node).ok_or("That item isn't in the scan.")?;
        idx.path_of(node)
    };
    if !path.exists() {
        return Err("It has been moved or deleted. Refresh the folder.".into());
    }
    let opener = app.opener();
    if reveal {
        opener.reveal_item_in_dir(&path).map_err(|e| e.to_string())
    } else {
        opener
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|e| e.to_string())
    }
}
