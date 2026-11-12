//! NomNom's own updates: look at the latest published release, and on request download its
//! installer, check it against the published SHA-256, run it and quit so it can replace the files.
//!
//! The check only reads the release page. A download or install happens only when the user presses
//! the button, and never while a download is running.

use crate::state::AppState;
use nomnom_tools::update::{self, Available, Check};
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

const DAY: i64 = 24 * 60 * 60;

#[derive(Default)]
pub struct UpdateState {
    /// The newest version found by the last check, if it is newer than this one.
    available: Mutex<Option<Available>>,
    busy: Mutex<bool>,
}

#[derive(Serialize, Clone)]
struct Progress {
    done: u64,
    total: Option<u64>,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The version of this copy of NomNom.
fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

async fn run_check(app: &AppHandle) -> Result<Check, String> {
    let result = update::check(current()).await.map_err(|e| e.to_string())?;
    let state = app.state::<AppState>();
    let _ = state
        .settings
        .update("updates", |s| s.updates.last_checked = now());
    let found = match &result {
        Check::Available(a) => Some(a.clone()),
        Check::UpToDate { .. } => None,
    };
    *app.state::<UpdateState>().available.lock().unwrap() = found.clone();
    if let Some(a) = found {
        let _ = app.emit("update://available", a);
    }
    Ok(result)
}

#[tauri::command]
pub fn update_version() -> &'static str {
    current()
}

/// What the last check found, without going to the network.
#[tauri::command]
pub fn update_known(updates: State<'_, UpdateState>) -> Option<Available> {
    updates.available.lock().unwrap().clone()
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> CmdResult<Check> {
    run_check(&app).await
}

/// Downloads the installer found by the last check, checks it, starts it and quits NomNom.
#[tauri::command]
pub async fn update_install(app: AppHandle) -> CmdResult<()> {
    let state = app.state::<AppState>();
    let updates = app.state::<UpdateState>();
    let Some(wanted) = updates.available.lock().unwrap().clone() else {
        return Err("Check for updates first.".into());
    };
    let running = state
        .downloads
        .list(500)
        .iter()
        .filter(|j| matches!(j.status.as_str(), "queued" | "running" | "paused"))
        .count();
    if running > 0 {
        return Err(format!(
            "{running} download{} still in the list. Finish or cancel {}, then update.",
            if running == 1 { " is" } else { "s are" },
            if running == 1 { "it" } else { "them" }
        ));
    }
    {
        let mut busy = updates.busy.lock().unwrap();
        if *busy {
            return Err("An update is already being downloaded.".into());
        }
        *busy = true;
    }
    let result = download_and_start(&app, &wanted).await;
    *updates.busy.lock().unwrap() = false;
    result?;
    // The installer needs NomNom closed to replace its files.
    app.exit(0);
    Ok(())
}

async fn download_and_start(app: &AppHandle, wanted: &Available) -> CmdResult<()> {
    let dir = std::env::temp_dir().join("NomNom-update");
    let emitter = app.clone();
    let last = Mutex::new(Instant::now() - Duration::from_secs(1));
    let progress = move |done: u64, total: Option<u64>| {
        let mut at = last.lock().unwrap();
        if at.elapsed() >= Duration::from_millis(150) {
            *at = Instant::now();
            let _ = emitter.emit("update://progress", Progress { done, total });
        }
    };
    let file = update::download(wanted, &dir, &progress)
        .await
        .map_err(|e| e.to_string())?;
    launch_installer(&file).map_err(|e| format!("The installer couldn't be started: {e}"))
}

/// Starts the installer on its own, so it keeps running after NomNom quits. This is not
/// `nomnom_core::process::spawn` on purpose: that ties the child to NomNom's process group.
/// `/P` shows progress without questions, `/UPDATE` marks it as an update, `/R` starts NomNom again.
fn launch_installer(file: &std::path::Path) -> std::io::Result<()> {
    let mut cmd = std::process::Command::new(file);
    cmd.args(["/P", "/UPDATE", "/R"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    cmd.spawn().map(|_| ())
}

/// Checks on start and then about once a day, if the user left automatic checks on.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Let NomNom finish starting before touching the network.
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            let cfg = app.state::<AppState>().settings.get().updates;
            if cfg.auto_check && now() - cfg.last_checked >= DAY {
                if let Err(e) = run_check(&app).await {
                    tracing::info!("update check failed: {e}");
                }
            }
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}
