//! The launcher: a small search window (Alt+Space by default) that starts programs, opens folders
//! and recent files, runs the person's own actions and jumps to a NomNom tool. The page does the
//! searching; this file lists what there is and opens what was chosen.

use crate::state::AppState;
use crate::windows::shared_webview_args;
use nomnom_core::launcher as rules;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

const LABEL: &str = "launcher";
const W: f64 = 640.0;
const H: f64 = 440.0;
/// Start-menu entries are re-read at most this often.
const APPS_FRESH: Duration = Duration::from_secs(120);
#[cfg_attr(not(windows), allow(dead_code))]
const MAX_RECENT: usize = 120;

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub name: String,
    /// What gets opened: a shortcut, a `.desktop` file, a folder or a file.
    pub path: String,
    /// "app", "folder" or "recent".
    pub kind: &'static str,
    #[serde(skip)]
    exec: Option<String>,
}

static APPS: Mutex<Option<(Instant, Vec<Entry>)>> = Mutex::new(None);
static OPENED: Mutex<Option<Instant>> = Mutex::new(None);

fn read_dir_tree(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth > 0 {
                read_dir_tree(&p, depth - 1, out);
            }
        } else {
            out.push(p);
        }
    }
}

#[cfg(windows)]
fn programs() -> Vec<Entry> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(pd) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(pd).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(ad) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(ad).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    let mut files = Vec::new();
    for r in &roots {
        read_dir_tree(r, 4, &mut files);
    }
    files
        .into_iter()
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("lnk") || e.eq_ignore_ascii_case("url"))
        })
        .filter_map(|p| {
            let name = p.file_stem()?.to_string_lossy().into_owned();
            (!rules::is_noise(&name)).then(|| Entry {
                name,
                path: p.to_string_lossy().into_owned(),
                kind: "app",
                exec: None,
            })
        })
        .collect()
}

#[cfg(not(windows))]
fn programs() -> Vec<Entry> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut roots: Vec<PathBuf> = vec![
        "/usr/share/applications".into(),
        "/usr/local/share/applications".into(),
        "/var/lib/flatpak/exports/share/applications".into(),
    ];
    if let Some(h) = &home {
        roots.push(h.join(".local/share/applications"));
        roots.push(h.join(".local/share/flatpak/exports/share/applications"));
    }
    let mut files = Vec::new();
    for r in &roots {
        read_dir_tree(r, 2, &mut files);
    }
    files
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "desktop"))
        .filter_map(|p| {
            let text = std::fs::read_to_string(&p).ok()?;
            let e = rules::parse_desktop_entry(&text)?;
            (!rules::is_noise(&e.name)).then(|| Entry {
                name: e.name,
                path: p.to_string_lossy().into_owned(),
                kind: "app",
                exec: Some(e.exec),
            })
        })
        .collect()
}

fn home() -> Option<PathBuf> {
    #[cfg(windows)]
    let var = "USERPROFILE";
    #[cfg(not(windows))]
    let var = "HOME";
    std::env::var_os(var).map(PathBuf::from)
}

fn folders() -> Vec<Entry> {
    let Some(home) = home() else {
        return Vec::new();
    };
    [
        "Desktop",
        "Documents",
        "Downloads",
        "Pictures",
        "Videos",
        "Music",
    ]
    .into_iter()
    .map(|n| (n, home.join(n)))
    .filter(|(_, p)| p.is_dir())
    .map(|(n, p)| Entry {
        name: n.into(),
        path: p.to_string_lossy().into_owned(),
        kind: "folder",
        exec: None,
    })
    .collect()
}

#[cfg(windows)]
fn recent() -> Vec<Entry> {
    let Some(ad) = std::env::var_os("APPDATA") else {
        return Vec::new();
    };
    let dir = PathBuf::from(ad).join(r"Microsoft\Windows\Recent");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(std::time::SystemTime, Entry)> = rd
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("lnk"))
        })
        .filter_map(|e| {
            let when = e.metadata().ok()?.modified().ok()?;
            let name = e.path().file_stem()?.to_string_lossy().into_owned();
            Some((
                when,
                Entry {
                    name,
                    path: e.path().to_string_lossy().into_owned(),
                    kind: "recent",
                    exec: None,
                },
            ))
        })
        .collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.0));
    found.into_iter().take(MAX_RECENT).map(|(_, e)| e).collect()
}

#[cfg(not(windows))]
fn recent() -> Vec<Entry> {
    Vec::new()
}

/// Everything the launcher can open without searching the disk: programs, the usual folders and
/// recent files. The same name appears once.
fn all() -> Vec<Entry> {
    let mut cache = APPS.lock().unwrap();
    let apps = match cache.as_ref() {
        Some((at, list)) if at.elapsed() < APPS_FRESH => list.clone(),
        _ => {
            let mut seen = HashSet::new();
            let mut list: Vec<Entry> = programs()
                .into_iter()
                .filter(|e| seen.insert(e.name.to_lowercase()))
                .collect();
            list.sort_by_key(|e| e.name.to_lowercase());
            *cache = Some((Instant::now(), list.clone()));
            list
        }
    };
    drop(cache);
    let mut out = apps;
    out.extend(folders());
    out.extend(recent());
    out
}

#[tauri::command]
pub async fn lc_entries() -> CmdResult<Vec<Entry>> {
    tokio::task::spawn_blocking(all)
        .await
        .map_err(|e| e.to_string())
}

/// Opens one of the entries that `lc_entries` gave. Anything else is refused.
#[tauri::command]
pub async fn lc_open(app: AppHandle, path: String) -> CmdResult<()> {
    let list = tokio::task::spawn_blocking(all)
        .await
        .map_err(|e| e.to_string())?;
    let entry = list
        .into_iter()
        .find(|e| e.path == path)
        .ok_or("That isn't in the list any more.")?;
    close(&app);
    if let Some(exec) = &entry.exec {
        let parts = nomnom_core::actions::split_args(exec)?;
        let (program, args) = parts.split_first().ok_or("That entry has no command.")?;
        return std::process::Command::new(program)
            .args(args)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Couldn't start it: {e}"));
    }
    app.opener()
        .open_path(&entry.path, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Opens a folder or file found by the file search, or reveals it. The path must exist.
#[tauri::command]
pub fn lc_open_path(app: AppHandle, path: String, reveal: bool) -> CmdResult<()> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err("That isn't there any more.".into());
    }
    close(&app);
    let opener = app.opener();
    if reveal {
        opener.reveal_item_in_dir(p).map_err(|e| e.to_string())
    } else {
        opener
            .open_path(&path, None::<&str>)
            .map_err(|e| e.to_string())
    }
}

/// Brings the main window to the front on one of its pages.
#[tauri::command]
pub fn lc_go(app: AppHandle, page: String) {
    close(&app);
    let was_open = app.get_webview_window("main").is_some();
    crate::windows::open_main(&app);
    // A window that was just made isn't listening yet.
    tauri::async_runtime::spawn(async move {
        if !was_open {
            tokio::time::sleep(Duration::from_millis(900)).await;
        }
        let _ = app.emit("nav://go", page);
    });
}

pub fn close(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.destroy();
    }
}

#[tauri::command]
pub fn lc_close(app: AppHandle) {
    close(&app);
}

/// The page calls this once it has drawn, so the window never shows empty.
#[tauri::command]
pub fn lc_ready(app: AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.show();
        let _ = win.set_focus();
        *OPENED.lock().unwrap() = Some(Instant::now());
    }
}

/// Opens the launcher on the screen the pointer is on, or closes it if it is open.
pub fn toggle(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        close(app);
        return;
    }
    if !app.state::<AppState>().settings.get().shortcuts.launcher_on {
        return;
    }
    *OPENED.lock().unwrap() = Some(Instant::now());
    let app = app.clone();
    // Built off the main thread (see toast.rs).
    tauri::async_runtime::spawn(async move {
        let built = WebviewWindowBuilder::new(
            &app,
            LABEL,
            WebviewUrl::App("panel.html?kind=launcher".into()),
        )
        .title("NomNom launcher")
        .inner_size(W, H)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false);
        match shared_webview_args(built).build() {
            Ok(win) => {
                place(&app, &win);
                let watched = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let opened = OPENED.lock().unwrap().map(|t| t.elapsed());
                        if opened.is_some_and(|e| e > Duration::from_millis(700)) {
                            let _ = watched.destroy();
                        }
                    }
                });
            }
            Err(e) => tracing::warn!("couldn't open the launcher: {e}"),
        }
    });
}

fn place(app: &AppHandle, win: &tauri::WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|c| app.monitor_from_point(c.x, c.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    if let (Some(m), Ok(size)) = (monitor, win.outer_size()) {
        let area = m.work_area();
        let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
        let y = area.position.y + (area.size.height as i32 - size.height as i32) / 4;
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }
}
