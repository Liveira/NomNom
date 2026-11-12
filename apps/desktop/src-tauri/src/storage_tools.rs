//! Disk Analyzer tools for the window: advanced search, age totals, duplicates, batch renaming and
//! reports. Anything that changes files goes through the same plan, confirm, run steps as
//! `storage_ops`; nothing here deletes or moves anything by itself.

use crate::state::AppState;
use crate::storage_ops::{protected, store_plan, PlanView};
use nomnom_storage::analyze::{self, FolderMode, Group};
use nomnom_storage::batch::{self, RenameRule};
use nomnom_storage::dupes::{self, DupGroup, JobProgress};
use nomnom_storage::index::NodeId;
use nomnom_storage::ops::{self, Kind};
use nomnom_storage::protect::Verdict;
use nomnom_storage::report::{self, Format};
use nomnom_storage::search::{self, DateField, Found, Query};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn gate(state: &AppState, id: u64) -> Result<std::sync::Arc<nomnom_storage::Session>, String> {
    let s = state.storage.get(id)?;
    if !s.is_finished() {
        return Err("Wait for the scan to finish first.".into());
    }
    Ok(s)
}

// ---- search and age -------------------------------------------------------------------------

#[tauri::command]
pub async fn st_search(
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
    query: Query,
    limit: usize,
) -> Result<Found, String> {
    let s = state.storage.get(id)?;
    let compiled = search::compile(&query, now())?;
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let c2 = cancel.clone();
    let handle = tauri::async_runtime::spawn_blocking(move || {
        let idx = s.index.read().unwrap();
        if idx.node(node).is_none() {
            return Err("That item isn't in the scan.".to_string());
        }
        Ok(search::run(
            &idx,
            node,
            &compiled,
            limit.clamp(1, 20_000),
            &c2,
        ))
    });
    handle.await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn st_by_age(
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
    field: DateField,
) -> Result<Vec<Group>, String> {
    let s = state.storage.get(id)?;
    let idx = s.index.read().unwrap();
    if idx.node(node).is_none() {
        return Err("That item isn't in the scan.".into());
    }
    Ok(analyze::by_age(&idx, node, field, now()))
}

// ---- duplicates -----------------------------------------------------------------------------

#[tauri::command]
pub fn st_dupes_start(
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
    min_size: u64,
) -> Result<(), String> {
    let s = gate(&state, id)?;
    let mut slot = state.storage.dupes.lock().unwrap();
    if let Some((_, old)) = slot.as_ref() {
        old.cancel();
    }
    let workers = if nomnom_storage::drives::drive_of(&s.root)
        .is_some_and(|d| d.media == nomnom_storage::drives::Media::Ssd)
    {
        4
    } else {
        2
    };
    *slot = Some((id, dupes::start(s, node, min_size.max(4096), workers)));
    Ok(())
}

#[tauri::command]
pub fn st_dupes_progress(state: State<'_, AppState>) -> Option<(u64, JobProgress)> {
    state
        .storage
        .dupes
        .lock()
        .unwrap()
        .as_ref()
        .map(|(id, j)| (*id, j.progress()))
}

#[tauri::command]
pub fn st_dupes_groups(state: State<'_, AppState>, offset: usize, limit: usize) -> Vec<DupGroup> {
    state
        .storage
        .dupes
        .lock()
        .unwrap()
        .as_ref()
        .map(|(_, j)| j.groups(offset, limit.min(500)))
        .unwrap_or_default()
}

#[tauri::command]
pub fn st_dupes_cancel(state: State<'_, AppState>) {
    if let Some((_, j)) = state.storage.dupes.lock().unwrap().as_ref() {
        j.cancel();
    }
}

#[derive(Deserialize)]
pub struct KeepGroup {
    pub keep: NodeId,
    pub remove: Vec<NodeId>,
}

/// Plans deleting the chosen copies. The window must name the copy that stays in every group.
#[tauri::command]
pub fn st_dupes_plan(
    state: State<'_, AppState>,
    id: u64,
    permanent: bool,
    groups: Vec<KeepGroup>,
) -> Result<PlanView, String> {
    let s = gate(&state, id)?;
    if groups.iter().any(|g| g.remove.is_empty()) {
        return Err("Pick which copies to remove.".into());
    }
    let pairs: Vec<(NodeId, Vec<NodeId>)> =
        groups.into_iter().map(|g| (g.keep, g.remove)).collect();
    let kind = if permanent {
        Kind::Delete
    } else {
        Kind::Recycle
    };
    let plan = {
        let idx = s.index.read().unwrap();
        ops::plan_duplicates(&idx, kind, &pairs, &protected(&state))
    };
    Ok(store_plan(&state, id, kind, plan))
}

// ---- batch renaming ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct RenameRow {
    pub old: String,
    pub new: Option<String>,
    pub error: Option<String>,
}

/// What a rule would do to the names of the chosen items. Nothing is changed.
#[tauri::command]
pub fn st_rename_preview(
    state: State<'_, AppState>,
    id: u64,
    nodes: Vec<NodeId>,
    rule: RenameRule,
) -> Result<Vec<RenameRow>, String> {
    let s = state.storage.get(id)?;
    let idx = s.index.read().unwrap();
    let mut ids: Vec<NodeId> = nodes
        .into_iter()
        .filter(|n| idx.node(*n).is_some())
        .collect();
    ids.sort_by_key(|n| idx.path_of(*n).to_string_lossy().to_lowercase());
    let names: Vec<String> = ids.iter().map(|n| idx.name(*n).to_string()).collect();
    let out = batch::rename_preview(&names, &rule)?;
    Ok(names
        .into_iter()
        .zip(out)
        .map(|(old, r)| match r {
            Ok(new) if new == old => RenameRow {
                old,
                new: None,
                error: Some("The name wouldn't change.".into()),
            },
            Ok(new) => RenameRow {
                old,
                new: Some(new),
                error: None,
            },
            Err(e) => RenameRow {
                old,
                new: None,
                error: Some(e),
            },
        })
        .collect())
}

#[tauri::command]
pub fn st_rename_plan(
    state: State<'_, AppState>,
    id: u64,
    nodes: Vec<NodeId>,
    rule: RenameRule,
) -> Result<PlanView, String> {
    let s = gate(&state, id)?;
    let plan = {
        let idx = s.index.read().unwrap();
        batch::plan_batch_rename(&idx, &nodes, &rule, &protected(&state))?
    };
    Ok(store_plan(&state, id, Kind::BatchRename, plan))
}

// ---- reports and path lists -----------------------------------------------------------------

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReportKind {
    Tree { depth: u32 },
    TopFiles { n: usize },
    TopFolders { n: usize, mode: FolderMode },
    Extensions,
    Categories,
    Age { field: DateField },
    Duplicates,
}

#[derive(Deserialize)]
pub struct ReportSpec {
    #[serde(flatten)]
    pub what: ReportKind,
    pub format: Format,
    /// Column keys to keep; empty keeps all.
    #[serde(default)]
    pub columns: Vec<String>,
}

/// Where a report may be written: a new file, outside the places NomNom protects.
fn checked_target(state: &AppState, path: &str, ext: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    if !p.is_absolute() {
        return Err("Choose where to save the file.".into());
    }
    if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
        return Err(format!("The file name must end in .{ext}."));
    }
    if let Verdict::Refuse(why) = protected(state).check(&p) {
        return Err(format!("NomNom won't save there. {why}"));
    }
    if !p.parent().is_some_and(|d| d.is_dir()) {
        return Err("That folder doesn't exist.".into());
    }
    Ok(p)
}

fn write_new(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                "A file with that name already exists. Choose another name.".to_string()
            } else {
                e.to_string()
            }
        })?;
    f.write_all(bytes).map_err(|e| e.to_string())
}

/// Builds a report from the scan and writes it to a new file. Returns the size written.
#[tauri::command]
pub async fn st_export(
    state: State<'_, AppState>,
    id: u64,
    node: NodeId,
    spec: ReportSpec,
    path: String,
) -> Result<u64, String> {
    let s = state.storage.get(id)?;
    let target = checked_target(&state, &path, spec.format.ext())?;
    let duplicates = state
        .storage
        .dupes
        .lock()
        .unwrap()
        .as_ref()
        .map(|(_, j)| j.groups(0, 100_000))
        .unwrap_or_default();
    let format = spec.format;
    let columns = spec.columns.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let idx = s.index.read().unwrap();
        if idx.node(node).is_none() {
            return Err("That item isn't in the scan.".into());
        }
        let table = match spec.what {
            ReportKind::Tree { depth } => report::tree_table(&idx, node, depth.clamp(1, 12)),
            ReportKind::TopFiles { n } => report::top_files_table(&idx, node, n.clamp(1, 100_000)),
            ReportKind::TopFolders { n, mode } => {
                report::top_folders_table(&idx, node, n.clamp(1, 100_000), mode)
            }
            ReportKind::Extensions => report::groups_table(
                "Space by file type",
                "Extension",
                &analyze::by_extension(&idx, node, 5000),
            ),
            ReportKind::Categories => report::groups_table(
                "Space by kind of file",
                "Kind",
                &analyze::by_category(&idx, node),
            ),
            ReportKind::Age { field } => report::groups_table(
                "Files by age",
                "Age",
                &analyze::by_age(&idx, node, field, now()),
            ),
            ReportKind::Duplicates => {
                if duplicates.is_empty() {
                    return Err("Search for duplicates first.".into());
                }
                report::duplicates_table(&duplicates)
            }
        };
        let table = report::select_columns(&table, &columns);
        report::render(&table, format).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    write_new(&target, &bytes)?;
    Ok(bytes.len() as u64)
}

/// Writes the full paths of the chosen items, one per line, to a new text file.
#[tauri::command]
pub fn st_export_paths(
    state: State<'_, AppState>,
    id: u64,
    nodes: Vec<NodeId>,
    path: String,
) -> Result<usize, String> {
    let s = state.storage.get(id)?;
    let target = checked_target(&state, &path, "txt")?;
    let idx = s.index.read().unwrap();
    let lines: Vec<String> = nodes
        .iter()
        .filter(|n| idx.node(**n).is_some())
        .map(|n| idx.path_of(*n).to_string_lossy().to_string())
        .collect();
    if lines.is_empty() {
        return Err("Select something first.".into());
    }
    write_new(&target, (lines.join("\r\n") + "\r\n").as_bytes())?;
    Ok(lines.len())
}
