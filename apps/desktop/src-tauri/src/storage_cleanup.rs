//! Disk Analyzer cleanup: finds the known places where programs keep files they can make again
//! and says how much each holds. It only measures. Clearing a place is done by the person, through
//! the Disk Analyzer's usual preview and confirmation.

use nomnom_storage::cleanup::{self, Env};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Serialize)]
pub struct SpotInfo {
    pub id: String,
    pub title: String,
    pub why: String,
    pub path: String,
    pub size: u64,
    pub files: u64,
    /// The count stopped early, so the real size is at least this.
    pub partial: bool,
}

fn env() -> Env {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_default();
    Env {
        home,
        local_app_data: if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        } else {
            None
        },
        temp: std::env::temp_dir(),
    }
}

#[tauri::command]
pub async fn st_cleanup_spots() -> Result<Vec<SpotInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut out: Vec<SpotInfo> = cleanup::spots(&env())
            .into_iter()
            .map(|s| {
                let m = cleanup::measure(&s.path, &cancel, 400_000);
                SpotInfo {
                    id: s.id,
                    title: s.title,
                    why: s.why,
                    path: s.path.to_string_lossy().into_owned(),
                    size: m.size,
                    files: m.files,
                    partial: m.partial,
                }
            })
            .filter(|s| s.files > 0)
            .collect();
        out.sort_by_key(|s| std::cmp::Reverse(s.size));
        out
    })
    .await
    .map_err(|e| e.to_string())
}
