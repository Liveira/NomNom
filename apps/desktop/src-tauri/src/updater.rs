//! NomNom's own updates: look at the latest published release, and on request download its
//! package, check it against the published SHA-256, and apply it directly in-app.
//!
//! - On Windows: starts the NSIS updater silently (`/S /UPDATE /R`), replaces the files in place,
//!   and relaunches NomNom automatically without an installer wizard.
//! - On Linux: replaces writable `.AppImage` files atomically in place and relaunches; on Arch
//!   Linux (`.pkg.tar.zst`) and Debian/Ubuntu (`.deb`), upgrades directly via Polkit (`pkexec`) and
//!   relaunches `/usr/bin/nomnom` (falling back to saving in Downloads if Polkit is unavailable).

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

/// Downloads the package found by the last check, verifies its SHA-256, and applies it (or reveals
/// the verified package in Downloads when managed by a Linux system package manager).
#[tauri::command]
pub async fn update_install(app: AppHandle) -> CmdResult<Option<String>> {
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
    let result = download_and_apply(&app, &wanted).await;
    *updates.busy.lock().unwrap() = false;
    let outcome = result?;
    if outcome.should_exit {
        app.exit(0);
    }
    Ok(outcome.message)
}

struct ApplyOutcome {
    should_exit: bool,
    message: Option<String>,
}

async fn download_and_apply(app: &AppHandle, wanted: &Available) -> CmdResult<ApplyOutcome> {
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
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || apply_downloaded_package(&app_clone, &file))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(windows)]
fn apply_downloaded_package(_app: &AppHandle, file: &std::path::Path) -> CmdResult<ApplyOutcome> {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    let mut cmd = std::process::Command::new(file);
    cmd.args(["/S", "/UPDATE", "/R"])
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    cmd.spawn()
        .map_err(|e| format!("The update couldn't be started: {e}"))?;
    Ok(ApplyOutcome {
        should_exit: true,
        message: None,
    })
}

#[cfg(not(windows))]
fn has_cmd(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()))
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn relaunch_linux(bin: &std::path::Path) {
    use std::os::unix::process::CommandExt;
    let _ = std::process::Command::new(bin)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn();
}

#[cfg(not(windows))]
fn apply_downloaded_package(app: &AppHandle, file: &std::path::Path) -> CmdResult<ApplyOutcome> {
    use std::os::unix::fs::PermissionsExt;
    use tauri_plugin_opener::OpenerExt;

    let name_lower = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    // 1. In-place AppImage self-update when launched from a writable AppImage path.
    if name_lower.ends_with(".appimage") {
        let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755));
        if let Some(current_appimage) = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from) {
            let sibling = current_appimage.with_extension("AppImage.new");
            if std::fs::copy(file, &sibling).is_ok() {
                let _ = std::fs::set_permissions(&sibling, std::fs::Permissions::from_mode(0o755));
                if std::fs::rename(&sibling, &current_appimage).is_ok() {
                    let _ = std::fs::remove_file(file);
                    relaunch_linux(&current_appimage);
                    return Ok(ApplyOutcome {
                        should_exit: true,
                        message: None,
                    });
                }
                let _ = std::fs::remove_file(&sibling);
            }
        }
    }

    // 2. Direct in-app upgrade via Polkit (`pkexec`) for Arch Linux (.pkg.tar.zst) and Debian/Ubuntu (.deb).
    if has_cmd("pkexec") {
        let upgraded = if name_lower.ends_with(".pkg.tar.zst") && has_cmd("pacman") {
            std::process::Command::new("pkexec")
                .args(["pacman", "-U", "--noconfirm"])
                .arg(file)
                .status()
                .is_ok_and(|s| s.success())
        } else if name_lower.ends_with(".deb") && has_cmd("apt-get") {
            std::process::Command::new("pkexec")
                .args(["apt-get", "install", "-y", "--allow-downgrades"])
                .arg(file)
                .status()
                .is_ok_and(|s| s.success())
        } else {
            false
        };

        if upgraded {
            let _ = std::fs::remove_file(file);
            let target_bin = std::path::Path::new("/usr/bin/nomnom");
            if target_bin.is_file() {
                relaunch_linux(target_bin);
            }
            return Ok(ApplyOutcome {
                should_exit: true,
                message: None,
            });
        }
    }

    // 3. Fallback when Polkit is unavailable or dismissed: save verified package in Downloads.
    let downloads = app.state::<AppState>().paths.downloads.clone();
    let file_name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("nomnom-update");
    if name_lower.ends_with(".appimage") {
        let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755));
    }
    let saved = nomnom_downloads::files::finalize(file, &downloads, file_name)
        .map_err(|e| format!("Couldn't save the verified package in Downloads: {e}"))?;
    let _ = app.opener().reveal_item_in_dir(&saved);

    let hint = if name_lower.ends_with(".pkg.tar.zst") {
        format!(
            "Verified and saved to {}. Install with: sudo pacman -U \"{}\"",
            saved.display(),
            saved.display()
        )
    } else if name_lower.ends_with(".deb") {
        format!(
            "Verified and saved to {}. Install with: sudo apt install \"{}\"",
            saved.display(),
            saved.display()
        )
    } else {
        format!("Verified and saved to {}.", saved.display())
    };
    Ok(ApplyOutcome {
        should_exit: false,
        message: Some(hint),
    })
}

/// Checks on start and then about once a day, if the user left automatic checks on.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
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
