//! File Tools: renaming many files at once. The window sends only the files and the options; the
//! new names are worked out here, every time, so what is applied is always what the preview showed.

use crate::filetools::output_problem;
use crate::state::AppState;
use chrono::{DateTime, Local};
use nomnom_core::rename::{self, FileInfo, Item, Moved, Options, Status};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

type CmdResult<T> = Result<T, String>;

const MAX_FILES: usize = 5000;

#[derive(Serialize)]
pub struct RnResult {
    pub renamed: usize,
    pub skipped: usize,
    pub failed: Vec<(String, String)>,
}

fn undo_file(state: &AppState) -> PathBuf {
    state.paths.data_dir.join("rename-undo.json")
}

fn info_of(path: &Path) -> Option<FileInfo> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    // Only plain files: never a folder, and never a link (which would rename the link's target).
    if !meta.is_file() {
        return None;
    }
    let modified = meta.modified().ok()?;
    let local: DateTime<Local> = modified.into();
    Some(FileInfo {
        path: path.to_path_buf(),
        stem: path.file_stem()?.to_string_lossy().into_owned(),
        ext: path
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default(),
        parent: path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        date: local.format("%Y-%m-%d").to_string(),
        time: local.format("%H%M%S").to_string(),
        modified: local.timestamp(),
    })
}

fn files_from(state: &AppState, paths: &[String]) -> CmdResult<Vec<FileInfo>> {
    if paths.is_empty() {
        return Err("Pick some files first.".into());
    }
    if paths.len() > MAX_FILES {
        return Err(format!("Rename up to {MAX_FILES} files at a time."));
    }
    let list: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    if list.iter().any(|p| !p.is_absolute()) {
        return Err("Only files on this computer can be renamed.".into());
    }
    if let Some(problem) = output_problem(state, &list, &None) {
        return Err(problem.replace(
            "Choose a folder to save the results in.",
            "NomNom won't rename files there.",
        ));
    }
    let infos: Vec<FileInfo> = list.iter().filter_map(|p| info_of(p)).collect();
    if infos.is_empty() {
        return Err("Pick files, not folders.".into());
    }
    Ok(infos)
}

fn with_today(mut o: Options) -> Options {
    o.today = Local::now().format("%Y-%m-%d").to_string();
    o
}

/// What the files are, for the list (folders and links are left out).
#[tauri::command]
pub fn rn_inspect(state: State<'_, AppState>, paths: Vec<String>) -> CmdResult<Vec<String>> {
    Ok(files_from(&state, &paths)?
        .into_iter()
        .map(|f| f.path.to_string_lossy().into_owned())
        .collect())
}

#[tauri::command]
pub async fn rn_preview(
    state: State<'_, AppState>,
    paths: Vec<String>,
    options: Options,
) -> CmdResult<Vec<Item>> {
    let files = files_from(&state, &paths)?;
    Ok(rename::preview(&files, &with_today(options), &|p| {
        p.exists()
    }))
}

#[tauri::command]
pub async fn rn_apply(
    state: State<'_, AppState>,
    paths: Vec<String>,
    options: Options,
) -> CmdResult<RnResult> {
    let files = files_from(&state, &paths)?;
    let items = rename::preview(&files, &with_today(options), &|p| p.exists());
    let skipped = items.iter().filter(|i| i.status != Status::Ok).count();
    let out = rename::apply(&items);
    if !out.done.is_empty() {
        if let Ok(json) = serde_json::to_string(&out.done) {
            let _ = std::fs::write(undo_file(&state), json);
        }
    }
    Ok(RnResult {
        renamed: out.done.len(),
        skipped,
        failed: out
            .failed
            .into_iter()
            .map(|(p, e)| (p.to_string_lossy().into_owned(), e))
            .collect(),
    })
}

fn last_batch(state: &AppState) -> Vec<Moved> {
    std::fs::read_to_string(undo_file(state))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// How many files the last rename can put back.
#[tauri::command]
pub fn rn_can_undo(state: State<'_, AppState>) -> usize {
    last_batch(&state).iter().filter(|m| m.to.is_file()).count()
}

#[tauri::command]
pub async fn rn_undo(state: State<'_, AppState>) -> CmdResult<RnResult> {
    // Only what is still where the rename left it.
    let batch: Vec<Moved> = last_batch(&state)
        .into_iter()
        .filter(|m| m.to.is_file())
        .collect();
    if batch.is_empty() {
        return Err("There is nothing to undo.".into());
    }
    let out = rename::undo(&batch);
    let _ = std::fs::remove_file(undo_file(&state));
    Ok(RnResult {
        renamed: out.done.len(),
        skipped: 0,
        failed: out
            .failed
            .into_iter()
            .map(|(p, e)| (p.to_string_lossy().into_owned(), e))
            .collect(),
    })
}
