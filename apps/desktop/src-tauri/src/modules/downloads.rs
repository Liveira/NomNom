//! Tauri commands for the Downloads module. All logic lives in `nomnom-downloads`.

use crate::state::AppState;
use nomnom_downloads::analyze::MediaInfo;
use nomnom_downloads::url_check::{is_probably_supported, site_label, validate_url};
use nomnom_downloads::{Choice, EnqueueOutcome, EnqueueRequest, JobRow};
use nomnom_tools::{Tool, ToolStatus};
use serde::Serialize;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

const CACHE_TTL: Duration = Duration::from_secs(600);
const CACHE_MAX: usize = 32;

#[derive(Serialize)]
pub struct Analysis {
    info: MediaInfo,
    site: String,
    known_site: bool,
}

#[tauri::command]
pub async fn dl_analyze(state: State<'_, AppState>, url: String) -> CmdResult<serde_json::Value> {
    let parsed = validate_url(&url).map_err(|e| e.to_string())?;
    let key = parsed.as_str().to_string();
    if let Some((at, v)) = state.analysis_cache.lock().unwrap().get(&key) {
        if at.elapsed() < CACHE_TTL {
            return Ok(v.clone());
        }
    }
    let cookies = state.settings.get().downloads.use_firefox_cookies;
    let backend = state.downloads.analyze_backend();
    let info = backend
        .analyze(&key, cookies)
        .await
        .map_err(|e| e.to_string())?;
    let value = serde_json::to_value(Analysis {
        info,
        site: site_label(&parsed),
        known_site: is_probably_supported(&parsed),
    })
    .map_err(|e| e.to_string())?;

    let mut cache = state.analysis_cache.lock().unwrap();
    if cache.len() >= CACHE_MAX {
        if let Some(oldest) = cache
            .iter()
            .min_by_key(|(_, (t, _))| *t)
            .map(|(k, _)| k.clone())
        {
            cache.remove(&oldest);
        }
    }
    cache.insert(key, (Instant::now(), value.clone()));
    Ok(value)
}

/// Shared by the app UI and, later, the popup and browser host.
pub fn enqueue_with_defaults(
    state: &AppState,
    url: String,
    choice: Choice,
    title: Option<String>,
    allow_duplicate_once: bool,
) -> CmdResult<EnqueueOutcome> {
    let s = state.settings.get().downloads;
    let dest_dir = s
        .output_dir
        .clone()
        .unwrap_or_else(|| state.paths.downloads.clone());
    state
        .downloads
        .enqueue(EnqueueRequest {
            url,
            choice,
            dest_dir,
            title,
            firefox_cookies: s.use_firefox_cookies,
            allow_duplicates: s.allow_duplicates || allow_duplicate_once,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dl_enqueue(
    state: State<'_, AppState>,
    url: String,
    choice: Choice,
    title: Option<String>,
    again: Option<bool>,
) -> CmdResult<EnqueueOutcome> {
    enqueue_with_defaults(&state, url, choice, title, again.unwrap_or(false))
}

#[tauri::command]
pub fn dl_list(state: State<'_, AppState>) -> Vec<JobRow> {
    state.downloads.list(200)
}

#[tauri::command]
pub async fn dl_cancel(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.downloads.cancel(&id).await;
    Ok(())
}

#[tauri::command]
pub async fn dl_pause(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.downloads.pause(&id).await;
    Ok(())
}

#[tauri::command]
pub fn dl_resume(state: State<'_, AppState>, id: String) {
    state.downloads.resume(&id);
}

#[tauri::command]
pub fn dl_retry(state: State<'_, AppState>, id: String) {
    state.downloads.retry(&id);
}

#[tauri::command]
pub fn dl_remove(state: State<'_, AppState>, id: String) {
    state.downloads.remove(&id);
}

#[tauri::command]
pub fn dl_clear_history(state: State<'_, AppState>) {
    state.downloads.clear_history();
}

/// Opens or reveals a finished download. The path comes from our own history, never from the UI.
#[tauri::command]
pub fn dl_open(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    reveal: bool,
) -> CmdResult<()> {
    let row = state
        .downloads
        .list(500)
        .into_iter()
        .find(|r| r.id == id)
        .ok_or("That download isn't in the history.")?;
    let path = PathBuf::from(row.dest_path.ok_or("This download has no file yet.")?);
    if !path.is_file() {
        return Err("The file has been moved or deleted.".into());
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

#[derive(Serialize)]
pub struct ToolsReport {
    ytdlp: ToolStatus,
    ffmpeg: ToolStatus,
}

#[tauri::command]
pub async fn tools_status(state: State<'_, AppState>) -> CmdResult<ToolsReport> {
    Ok(ToolsReport {
        ytdlp: state.tools.status(Tool::YtDlp).await,
        ffmpeg: state.tools.status(Tool::Ffmpeg).await,
    })
}

#[derive(Serialize, Clone)]
struct InstallEvent {
    tool: String,
    done: u64,
    total: Option<u64>,
}

/// Installs a tool after the user asked for it. Progress arrives as `tools://progress`.
#[tauri::command]
pub async fn tools_install(
    app: AppHandle,
    state: State<'_, AppState>,
    tool: String,
) -> CmdResult<()> {
    let name = tool.clone();
    let emitter = app.clone();
    let last = std::sync::Mutex::new(Instant::now() - Duration::from_secs(1));
    let progress = move |done: u64, total: Option<u64>| {
        let mut at = last.lock().unwrap();
        if at.elapsed() >= Duration::from_millis(150) {
            *at = Instant::now();
            let _ = emitter.emit(
                "tools://progress",
                InstallEvent {
                    tool: name.clone(),
                    done,
                    total,
                },
            );
        }
    };
    let result = match tool.as_str() {
        "ytdlp" => state.tools.install_ytdlp(&progress).await,
        "ffmpeg" => state.tools.install_ffmpeg(&progress).await,
        _ => return Err("Unknown tool.".into()),
    };
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tools_update_ytdlp(state: State<'_, AppState>) -> CmdResult<String> {
    state.tools.update_ytdlp().await.map_err(|e| e.to_string())
}
