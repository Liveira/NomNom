//! File Tools inside the app: inspect dropped files, run conversions, and handle the
//! `--shell-action` command line that the Explorer menu uses.
//!
//! Conversions never overwrite or modify a source. The window sends file paths and options; every
//! request is checked again here before anything runs.

use crate::organizer::dest_is_acceptable;
use crate::state::AppState;
use nomnom_converter::{
    applicable, batch, capabilities, describe, presets, run, Backends, Conversion, Event, FileInfo,
    Options, Outcome, Preset, Request, Status, Target,
};
use nomnom_organizer::pathutil::is_inside;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tokio_util::sync::CancellationToken;

/// How long shell actions that arrive close together are gathered into one batch.
const SHELL_GATHER: Duration = Duration::from_millis(500);
const MAX_FILES: usize = 5000;

#[derive(Default)]
pub struct FileToolsState {
    running: AtomicBool,
    cancel: Mutex<Option<CancellationToken>>,
    /// Files this run wrote; only these can be opened or shown from the window.
    outputs: Mutex<HashSet<PathBuf>>,
    /// Files sent from Explorer's "More options" waiting for the window to take them.
    inbox: Mutex<Vec<PathBuf>>,
    /// Files from silent Explorer actions, gathered per action for a moment.
    gathering: Mutex<HashMap<String, Vec<PathBuf>>>,
}

#[derive(Serialize)]
pub struct Info {
    pub conversions: Vec<Conversion>,
    pub presets: Vec<Preset>,
}

#[tauri::command]
pub fn ft_info(state: State<'_, AppState>) -> Info {
    Info {
        conversions: capabilities(&Backends::detect(&state.tools)),
        presets: presets(),
    }
}

#[derive(Serialize)]
pub struct Inspection {
    pub files: Vec<FileInfo>,
    /// Conversions that work for every file that can be converted at all.
    pub conversions: Vec<Conversion>,
}

#[tauri::command]
pub fn ft_inspect(state: State<'_, AppState>, paths: Vec<String>) -> Result<Inspection, String> {
    if paths.len() > MAX_FILES {
        return Err("That's too many files at once. Try 5000 or fewer.".into());
    }
    let paths: Vec<PathBuf> = paths
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .collect();
    let files = describe(&paths);
    let ok: Vec<PathBuf> = files
        .iter()
        .filter(|f| f.problem.is_none())
        .map(|f| f.path.clone())
        .collect();
    let caps = capabilities(&Backends::detect(&state.tools));
    let conversions = applicable(&ok, &caps);
    Ok(Inspection { files, conversions })
}

#[derive(Deserialize)]
pub struct RunArgs {
    pub paths: Vec<String>,
    pub target: Target,
    #[serde(default)]
    pub options: Options,
    #[serde(default)]
    pub out_dir: Option<String>,
}

/// Results may be written next to the source or into a chosen folder, but never inside system,
/// program or NOMNOM data folders.
fn output_problem(
    state: &AppState,
    files: &[PathBuf],
    out_dir: &Option<PathBuf>,
) -> Option<String> {
    let (zones, _) = state.paths.protected_folders();
    match out_dir {
        Some(d) => (!dest_is_acceptable(d, &zones)).then(|| "NOMNOM won't save files there. Choose another folder.".to_string()),
        None => files
            .iter()
            .filter_map(|f| f.parent())
            .any(|d| zones.iter().any(|z| is_inside(d, z)))
            .then(|| {
                "These files are in a system or program folder. Choose a folder to save the results in."
                    .to_string()
            }),
    }
}

struct RunningGuard<'a>(&'a FileToolsState);
impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        *self.0.cancel.lock().unwrap() = None;
        self.0.running.store(false, Ordering::SeqCst);
    }
}

async fn run_request(
    app: &AppHandle,
    req: Request,
    emit_events: bool,
) -> Result<Vec<Outcome>, String> {
    let state = app.state::<AppState>();
    let ft = &state.filetools;
    let backends = Backends::detect(&state.tools);
    batch::check(&req, &backends).map_err(|e| e.to_string())?;
    if let Some(problem) = output_problem(&state, &req.files, &req.out_dir) {
        return Err(problem);
    }
    if ft
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("A conversion is already running. Wait for it to finish or stop it.".into());
    }
    let _guard = RunningGuard(ft);
    let token = CancellationToken::new();
    *ft.cancel.lock().unwrap() = Some(token.clone());
    let sink: Arc<dyn Fn(Event) + Send + Sync> = if emit_events {
        let app = app.clone();
        Arc::new(move |e| {
            let _ = app.emit("filetools://event", &e);
        })
    } else {
        Arc::new(|_| {})
    };
    let outcomes = run(req, backends, token, sink).await;
    let mut outs = ft.outputs.lock().unwrap();
    for o in &outcomes {
        if let Some(d) = &o.dst {
            outs.insert(d.clone());
        }
    }
    Ok(outcomes)
}

#[tauri::command]
pub async fn ft_run(app: AppHandle, args: RunArgs) -> Result<Vec<Outcome>, String> {
    let req = Request {
        files: args.paths.into_iter().map(PathBuf::from).collect(),
        target: args.target,
        options: args.options,
        out_dir: args.out_dir.filter(|d| !d.is_empty()).map(PathBuf::from),
    };
    run_request(&app, req, true).await
}

#[tauri::command]
pub fn ft_cancel(state: State<'_, AppState>) {
    if let Some(t) = state.filetools.cancel.lock().unwrap().as_ref() {
        t.cancel();
    }
}

#[tauri::command]
pub fn ft_take_inbox(state: State<'_, AppState>) -> Vec<String> {
    std::mem::take(&mut *state.filetools.inbox.lock().unwrap())
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}

/// Opens a result, or shows it in its folder. Only files written by File Tools in this session.
#[tauri::command]
pub fn ft_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    reveal: bool,
) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !state.filetools.outputs.lock().unwrap().contains(&path) {
        return Err("NOMNOM only opens files it just made.".into());
    }
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

// ---- Explorer actions --------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct ShellAction {
    /// "choose", "convert:<target>" or "preset:<id>".
    pub action: String,
    pub files: Vec<PathBuf>,
}

/// Reads `--shell-action <action> --files <paths...>` or `--files-from <list file>`.
pub fn parse_shell_args(args: &[String]) -> Option<ShellAction> {
    let i = args.iter().position(|a| a == "--shell-action")?;
    let action = args.get(i + 1)?.clone();
    if !(action == "choose" || action.starts_with("convert:") || action.starts_with("preset:")) {
        return None;
    }
    let mut files: Vec<PathBuf> = Vec::new();
    let mut j = 0;
    while j < args.len() {
        match args[j].as_str() {
            "--files" => {
                j += 1;
                while j < args.len() && !args[j].starts_with("--") {
                    files.push(PathBuf::from(&args[j]));
                    j += 1;
                }
                continue;
            }
            "--files-from" => {
                if let Some(list) = args.get(j + 1) {
                    files.extend(read_list(Path::new(list)));
                }
                j += 2;
                continue;
            }
            _ => {}
        }
        j += 1;
    }
    files.retain(|f| f.is_absolute());
    files.truncate(MAX_FILES);
    Some(ShellAction { action, files })
}

/// A list file written by the Explorer menu, one path per line. Only read from NOMNOM's own
/// folder in the temp directory, and removed after reading.
fn read_list(list: &Path) -> Vec<PathBuf> {
    let allowed = std::env::temp_dir().join("nomnom");
    if !is_inside(list, &allowed) {
        tracing::warn!("ignored a file list outside {}", allowed.display());
        return vec![];
    }
    let text = std::fs::read_to_string(list).unwrap_or_default();
    let _ = std::fs::remove_file(list);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

pub fn handle_shell_action(app: &AppHandle, action: ShellAction) {
    let state = app.state::<AppState>();
    if action.action == "choose" {
        state.filetools.inbox.lock().unwrap().extend(action.files);
        crate::windows::open_main(app);
        let _ = app.emit("filetools://inbox", ());
        return;
    }
    {
        let mut g = state.filetools.gathering.lock().unwrap();
        g.entry(action.action.clone())
            .or_default()
            .extend(action.files);
    }
    let app = app.clone();
    let key = action.action;
    tauri::async_runtime::spawn(async move {
        // Several files selected in Explorer can arrive as several launches; wait for the rest.
        tokio::time::sleep(SHELL_GATHER).await;
        let files = {
            let state = app.state::<AppState>();
            let mut g = state.filetools.gathering.lock().unwrap();
            g.remove(&key).unwrap_or_default()
        };
        if files.is_empty() {
            return;
        }
        run_silent(&app, &key, files).await;
    });
}

fn resolve(action: &str) -> Option<(Target, Options)> {
    if let Some(t) = action.strip_prefix("convert:") {
        let target: Target = serde_json::from_value(serde_json::Value::String(t.into())).ok()?;
        return Some((target, Options::default()));
    }
    let id = action.strip_prefix("preset:")?;
    presets()
        .into_iter()
        .find(|p| p.id == id)
        .map(|p| (p.target, p.options))
}

async fn run_silent(app: &AppHandle, action: &str, mut files: Vec<PathBuf>) {
    let notify = |title: &str, body: String| {
        let _ = app.notification().builder().title(title).body(body).show();
    };
    let Some((target, mut options)) = resolve(action) else {
        notify("File Tools", "That action isn't one NOMNOM knows.".into());
        return;
    };
    files.sort();
    files.dedup();
    let total = files.len();
    // Combining several pictures is a preset choice; everything else converts file by file.
    let usable: Vec<PathBuf> = files
        .into_iter()
        .filter(|f| nomnom_converter::model::valid_for(f, target))
        .collect();
    if usable.is_empty() {
        notify(
            "File Tools",
            format!(
                "None of the {total} selected files can be turned into {}.",
                target.label()
            ),
        );
        return;
    }
    if options.combine && usable.len() < 2 {
        options.combine = false;
    }
    let skipped = total - usable.len();
    let req = Request {
        files: usable,
        target,
        options,
        out_dir: None,
    };
    match run_request(app, req, false).await {
        Ok(out) => {
            let done = out.iter().filter(|o| o.status == Status::Done).count();
            let failed = out.iter().filter(|o| o.status == Status::Failed).count();
            let mut body = if done == 1 && out.len() == 1 {
                format!(
                    "Saved as {}.",
                    out[0]
                        .dst
                        .as_ref()
                        .and_then(|d| d.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("a new file")
                )
            } else {
                format!("{done} of {} converted to {}.", out.len(), target.label())
            };
            if failed > 0 {
                let first = out.iter().find_map(|o| {
                    (o.status == Status::Failed)
                        .then(|| o.message.clone())
                        .flatten()
                });
                body.push_str(&format!(" {failed} failed"));
                if let Some(m) = first {
                    body.push_str(&format!(": {m}"));
                }
            }
            if skipped > 0 {
                body.push_str(&format!(
                    " {skipped} skipped because they can't be turned into {}.",
                    target.label()
                ));
            }
            notify("File Tools", body);
            let _ = app.emit("filetools://changed", ());
        }
        Err(msg) => notify("File Tools couldn't convert", msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn shell_args_with_several_files() {
        let a = parse_shell_args(&args(&[
            "nomnom.exe",
            "--shell-action",
            "convert:webp",
            "--files",
            "C:\\a.png",
            "C:\\b c.png",
        ]))
        .unwrap();
        assert_eq!(a.action, "convert:webp");
        assert_eq!(
            a.files,
            [PathBuf::from("C:\\a.png"), PathBuf::from("C:\\b c.png")]
        );
    }

    #[test]
    fn relative_paths_and_unknown_actions_are_ignored() {
        let a = parse_shell_args(&args(&[
            "x",
            "--shell-action",
            "convert:webp",
            "--files",
            "rel.png",
            "C:\\ok.png",
        ]))
        .unwrap();
        assert_eq!(a.files, [PathBuf::from("C:\\ok.png")]);
        assert!(parse_shell_args(&args(&[
            "x",
            "--shell-action",
            "delete:all",
            "--files",
            "C:\\a"
        ]))
        .is_none());
        assert!(parse_shell_args(&args(&["x", "--files", "C:\\a"])).is_none());
    }

    #[test]
    fn a_list_file_outside_the_nomnom_temp_folder_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let list = dir.path().join("list.txt");
        std::fs::write(&list, "C:\\a.png\n").unwrap();
        assert!(read_list(&list).is_empty());
        assert!(list.exists(), "an unrelated file must not be deleted");
    }

    #[test]
    fn a_list_file_in_the_nomnom_temp_folder_is_read_then_removed() {
        let dir = std::env::temp_dir().join("nomnom");
        std::fs::create_dir_all(&dir).unwrap();
        let list = dir.join(format!("test-{}.txt", std::process::id()));
        std::fs::write(&list, "C:\\a.png\r\n\r\nC:\\b.png\n").unwrap();
        assert_eq!(
            read_list(&list),
            [PathBuf::from("C:\\a.png"), PathBuf::from("C:\\b.png")]
        );
        assert!(!list.exists());
    }

    #[test]
    fn actions_resolve_to_targets_and_presets() {
        assert_eq!(resolve("convert:webp").unwrap().0, Target::Webp);
        assert_eq!(resolve("preset:podcast-mp3").unwrap().0, Target::Mp3);
        assert!(resolve("convert:exe").is_none());
        assert!(resolve("preset:nope").is_none());
    }
}
