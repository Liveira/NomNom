//! The person's own actions: list, save, delete and run. What counts as an action and how it is
//! turned into a command is in `nomnom_core::actions`; this file keeps the list, gives the
//! shortcuts to the system, and starts things.

use crate::state::AppState;
use crate::toast::{self, Toast};
use nomnom_core::actions::{self as rules, Action, Plan};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::io::AsyncReadExt;

type CmdResult<T> = Result<T, String>;

/// How long a command may run before it is stopped, and how much of what it prints is kept.
const TIMEOUT: Duration = Duration::from_secs(120);
const KEEP_BYTES: u64 = 64 * 1024;

#[derive(Debug, Serialize)]
pub struct ActionView {
    #[serde(flatten)]
    action: Action,
    /// Why its shortcut isn't active, when it isn't.
    shortcut_error: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct RunResult {
    pub ok: bool,
    /// The exit code of a command that was waited for.
    pub code: Option<i32>,
    pub output: String,
    pub truncated: bool,
    pub timed_out: bool,
    /// A short sentence for what happened ("Opened the folder.").
    pub message: String,
}

fn views(app: &AppHandle) -> Vec<ActionView> {
    let state = app.state::<AppState>();
    let errors = state.custom_errors.lock().unwrap();
    state
        .settings
        .get()
        .actions
        .into_iter()
        .map(|a| ActionView {
            shortcut_error: errors.get(&a.id).cloned(),
            action: a,
        })
        .collect()
}

fn new_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("a{nanos:x}")
}

#[tauri::command]
pub fn ca_list(app: AppHandle) -> Vec<ActionView> {
    views(&app)
}

/// Adds an action or replaces the one with the same id. The shortcut is checked here, so a key
/// another action or NomNom already uses is refused with a reason.
#[tauri::command]
pub fn ca_save(app: AppHandle, mut action: Action) -> CmdResult<Vec<ActionView>> {
    action.name = action.name.trim().to_string();
    action.target = action.target.trim().to_string();
    if action.id.trim().is_empty() {
        action.id = new_id();
    }
    action.shortcut = action.shortcut.take().filter(|k| !k.trim().is_empty());
    rules::validate(&action)?;
    if let Some(keys) = action.shortcut.as_deref() {
        let parsed = crate::shortcuts::parse_accelerator(keys)?;
        if crate::shortcuts::builtin_uses(&app, &parsed) {
            return Err("That's already used by a NomNom shortcut.".into());
        }
        if crate::shortcuts::custom_uses(&app, &parsed, Some(&action.id)) {
            return Err("Another one of your actions already uses that.".into());
        }
    }
    let mut list = app.state::<AppState>().settings.get().actions;
    match list.iter_mut().find(|a| a.id == action.id) {
        Some(slot) => *slot = action,
        None => list.push(action),
    }
    rules::check_list(&list)?;
    app.state::<AppState>()
        .settings
        .update("actions", |s| s.actions = list)
        .map_err(|e| e.to_string())?;
    crate::shortcuts::sync_custom(&app);
    Ok(views(&app))
}

#[tauri::command]
pub fn ca_delete(app: AppHandle, id: String) -> CmdResult<Vec<ActionView>> {
    app.state::<AppState>()
        .settings
        .update("actions", |s| s.actions.retain(|a| a.id != id))
        .map_err(|e| e.to_string())?;
    crate::shortcuts::sync_custom(&app);
    Ok(views(&app))
}

/// Runs one of the saved actions. Only an action that is in the list can run: the window cannot
/// hand over a command of its own.
#[tauri::command]
pub async fn ca_run(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    values: HashMap<String, String>,
) -> CmdResult<RunResult> {
    let action = state
        .settings
        .get()
        .actions
        .into_iter()
        .find(|a| a.id == id)
        .ok_or("That action is gone.")?;
    run(&app, &action, &values).await
}

/// Runs a saved action with the values given (used by folder rules, which pass the file).
pub async fn run_by_id(
    app: &AppHandle,
    id: &str,
    values: &HashMap<String, String>,
) -> Result<RunResult, String> {
    let action = app
        .state::<AppState>()
        .settings
        .get()
        .actions
        .into_iter()
        .find(|a| a.id == id)
        .ok_or("The action it should run is gone.")?;
    let r = run(app, &action, values).await?;
    if r.ok {
        Ok(r)
    } else {
        Err(format!("{}: {}", action.name, r.message))
    }
}

/// Runs an action from its shortcut: no window to show the answer in, so a card says what happened.
pub fn run_in_background(app: &AppHandle, id: &str) {
    let app = app.clone();
    let id = id.to_string();
    tauri::async_runtime::spawn(async move {
        let Some(action) = app
            .state::<AppState>()
            .settings
            .get()
            .actions
            .into_iter()
            .find(|a| a.id == id)
        else {
            return;
        };
        let name = action.name.clone();
        match run(&app, &action, &HashMap::new()).await {
            Ok(r) if r.ok => {
                // Something that only opens a program or a folder needs no announcement.
                if action.kind == rules::Kind::Command {
                    toast::show(&app, Toast::new("success", name, first_line(&r)));
                }
            }
            Ok(r) => toast::show(&app, Toast::new("error", name, first_line(&r))),
            Err(e) => toast::show(&app, Toast::new("error", name, e)),
        }
    });
}

fn first_line(r: &RunResult) -> String {
    let line = r
        .output
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or(&r.message);
    line.chars().take(160).collect()
}

async fn run(
    app: &AppHandle,
    action: &Action,
    values: &HashMap<String, String>,
) -> CmdResult<RunResult> {
    match rules::plan(action, values)? {
        Plan::Open(target) => {
            let opener = app.opener();
            let res = if action.kind == rules::Kind::Link {
                opener.open_url(&target, None::<&str>)
            } else {
                if !Path::new(&target).exists() {
                    return Err("That folder or file isn't there any more.".into());
                }
                opener.open_path(&target, None::<&str>)
            };
            res.map_err(|e| e.to_string())?;
            Ok(RunResult {
                ok: true,
                message: "Opened.".into(),
                ..Default::default()
            })
        }
        Plan::Run {
            program,
            args,
            cwd,
            wait: false,
        } => {
            start_program(app, &program, &args, cwd.as_deref())?;
            Ok(RunResult {
                ok: true,
                message: "Started.".into(),
                ..Default::default()
            })
        }
        Plan::Run {
            program,
            args,
            cwd,
            wait: true,
        } => run_and_collect(&program, &args, cwd.as_deref()).await,
    }
}

/// Windows programs such as cmd.exe refuse a path written with forward slashes.
fn native(program: &str) -> String {
    if cfg!(windows) && program.contains('/') && !program.contains("://") {
        program.replace('/', "\\")
    } else {
        program.to_string()
    }
}

/// Starts a program and lets it go: it keeps running after NomNom closes.
fn start_program(
    app: &AppHandle,
    program: &str,
    args: &[String],
    cwd: Option<&str>,
) -> CmdResult<()> {
    let program = &native(program);
    // A shortcut file or a document with no arguments goes through the system, as a double click.
    #[cfg(windows)]
    if args.is_empty() && Path::new(program).is_file() {
        return app
            .opener()
            .open_path(program, None::<&str>)
            .map_err(|e| e.to_string());
    }
    let _ = app;
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        if !Path::new(dir).is_dir() {
            return Err("The folder to start in isn't there.".into());
        }
        cmd.current_dir(dir);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't start it: {e}"))
}

async fn run_and_collect(
    program: &str,
    args: &[String],
    cwd: Option<&str>,
) -> CmdResult<RunResult> {
    let program = &native(program);
    let dir = cwd.map(Path::new);
    if let Some(d) = dir {
        if !d.is_dir() {
            return Err("The folder to run in isn't there.".into());
        }
    }
    let mut child = nomnom_core::process::spawn_in(Path::new(program), args, true, dir)
        .map_err(|e| format!("Couldn't start it: {e}"))?;
    let mut stdout = child.child.stdout.take();
    let mut stderr = child.child.stderr.take();
    let work = async {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let read_out = async {
            if let Some(s) = stdout.as_mut() {
                let _ = s.take(KEEP_BYTES + 1).read_to_end(&mut out).await;
                let _ = tokio::io::copy(s, &mut tokio::io::sink()).await;
            }
        };
        let read_err = async {
            if let Some(s) = stderr.as_mut() {
                let _ = s.take(KEEP_BYTES + 1).read_to_end(&mut err).await;
                let _ = tokio::io::copy(s, &mut tokio::io::sink()).await;
            }
        };
        tokio::join!(read_out, read_err);
        let status = child.child.wait().await;
        (out, err, status)
    };
    match tokio::time::timeout(TIMEOUT, work).await {
        Ok((out, err, status)) => {
            let truncated = out.len() as u64 > KEEP_BYTES || err.len() as u64 > KEEP_BYTES;
            let mut text =
                String::from_utf8_lossy(&out[..out.len().min(KEEP_BYTES as usize)]).into_owned();
            let err_text =
                String::from_utf8_lossy(&err[..err.len().min(KEEP_BYTES as usize)]).into_owned();
            if !err_text.trim().is_empty() {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&err_text);
            }
            let code = status.ok().and_then(|s| s.code());
            Ok(RunResult {
                ok: code == Some(0),
                code,
                output: text,
                truncated,
                timed_out: false,
                message: match code {
                    Some(0) => "Finished.".into(),
                    Some(c) => format!("It stopped with code {c}."),
                    None => "It was stopped.".into(),
                },
            })
        }
        Err(_) => {
            child.kill_tree().await;
            Ok(RunResult {
                ok: false,
                timed_out: true,
                message: "It took longer than two minutes, so it was stopped.".into(),
                ..Default::default()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(windows)]
    async fn a_command_gives_back_what_it_printed_and_its_code() {
        let r = run_and_collect(
            r"C:\Windows\System32\cmd.exe",
            &["/c".into(), "echo hello & exit 3".into()],
            None,
        )
        .await
        .unwrap();
        assert_eq!(r.code, Some(3));
        assert!(!r.ok);
        assert!(r.output.contains("hello"));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn a_command_gives_back_what_it_printed_and_its_code() {
        let r = run_and_collect("/bin/sh", &["-c".into(), "echo hello; exit 3".into()], None)
            .await
            .unwrap();
        assert_eq!(r.code, Some(3));
        assert!(!r.ok);
        assert!(r.output.contains("hello"));
    }

    #[tokio::test]
    async fn a_program_that_isnt_there_is_an_error_not_a_crash() {
        assert!(
            run_and_collect("definitely-not-a-program-nomnom", &[], None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_missing_start_folder_is_refused() {
        assert!(run_and_collect("x", &[], Some("Z:\\no\\such\\dir"))
            .await
            .is_err());
    }
}
