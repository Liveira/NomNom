//! Folder rules at work: every few seconds the watched folders are looked at, and a file that has
//! stopped changing and fits a rule goes through its steps. Nothing is ever deleted or replaced:
//! a moved file takes a free name, a converted picture is a new file, and every move can be undone
//! from the list of what happened.

use crate::i18n::{lang_of, Lang};
use crate::state::AppState;
use crate::toast::{self, Toast};
use nomnom_converter::image_conv;
use nomnom_converter::model::{Options as ConvOptions, Target};
use nomnom_core::folder_rules::{self as rules, Rule, Step};
use nomnom_core::rename::{self, FileInfo, Options as RenameOptions};
use nomnom_organizer::pathutil;
use rusqlite::params;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

/// How often the folders are looked at, and how long a file has to stay unchanged before it counts.
const POLL: Duration = Duration::from_secs(6);
const QUIET: Duration = Duration::from_secs(6);
const MAX_PER_ROUND: usize = 60;
const MAX_DEPTH: usize = 4;
const KEEP_LOG: i64 = 500;

struct Seen {
    len: u64,
    modified: SystemTime,
    since: Instant,
}
static SEEN: Mutex<Option<HashMap<PathBuf, Seen>>> = Mutex::new(None);
/// Files already dealt with, so a copy rule doesn't copy the same file every round.
static DONE: Mutex<Option<HashSet<(PathBuf, i64)>>> = Mutex::new(None);
/// Per rule: files older than this are left alone (moves forward as the folder is looked at).
static SCANNED: Mutex<Option<HashMap<String, i64>>> = Mutex::new(None);

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn secs(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---- what is allowed ------------------------------------------------------------------------------

/// Is this a folder rules may watch, or put things in? Drive roots, the system's own folders and
/// NomNom's data are not.
pub(crate) fn allowed(state: &AppState, folder: &Path) -> Result<(), String> {
    if !folder.is_absolute() || pathutil::has_traversal(folder) {
        return Err("Use a full folder path.".into());
    }
    if pathutil::is_drive_root(folder) {
        return Err("A whole drive is too much. Choose a folder on it.".into());
    }
    let (zones, exact) = state.paths.protected_folders();
    if exact.iter().any(|e| pathutil::is_same(folder, e)) {
        return Err("Choose a folder inside it, not that one.".into());
    }
    if zones
        .iter()
        .any(|z| pathutil::is_inside(folder, z) || pathutil::is_same(folder, z))
    {
        return Err(
            "That folder belongs to the system or to NomNom itself, so rules stay out of it."
                .into(),
        );
    }
    Ok(())
}

fn check_rule(state: &AppState, r: &Rule) -> Result<(), String> {
    rules::validate(r)?;
    let watched = Path::new(r.folder.trim());
    allowed(state, watched)?;
    if !watched.is_dir() {
        return Err("The folder to watch isn't there.".into());
    }
    for s in &r.steps {
        match s {
            Step::Move { folder } | Step::Copy { folder } => {
                let d = Path::new(folder.trim());
                allowed(state, d)?;
                if pathutil::is_same(d, watched) {
                    return Err("Move it somewhere else, not into the folder it is in.".into());
                }
            }
            Step::Convert { originals, .. } if !originals.trim().is_empty() => {
                allowed(state, Path::new(originals.trim()))?;
            }
            Step::Run { action }
                if !state.settings.get().actions.iter().any(|a| a.id == *action) =>
            {
                return Err("The action it should run is gone.".into());
            }
            _ => {}
        }
    }
    Ok(())
}

// ---- doing the steps ------------------------------------------------------------------------------

/// A name for `name` in `dir` that isn't taken: "photo.png", then "photo (2).png", and so on.
fn move_unique(src: &Path, dir: &Path, name: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Couldn't make the folder: {e}"))?;
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    for n in 1..10_000u32 {
        let candidate = if n == 1 {
            dir.join(name)
        } else {
            dir.join(format!("{stem} ({n}){ext}"))
        };
        if candidate.exists() {
            continue;
        }
        match nomnom_core::fs::move_no_replace(src, &candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Couldn't move it: {e}")),
        }
    }
    Err("Every name in that folder is taken.".into())
}

fn copy_unique(src: &Path, dir: &Path, name: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Couldn't make the folder: {e}"))?;
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    for n in 1..10_000u32 {
        let candidate = if n == 1 {
            dir.join(name)
        } else {
            dir.join(format!("{stem} ({n}){ext}"))
        };
        if candidate.exists() {
            continue;
        }
        // `create_new` so a file that appears meanwhile is never overwritten.
        let mut out = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Couldn't copy it: {e}")),
        };
        let mut input = std::fs::File::open(src).map_err(|e| format!("Couldn't read it: {e}"))?;
        std::io::copy(&mut input, &mut out).map_err(|e| format!("Couldn't copy it: {e}"))?;
        return Ok(candidate);
    }
    Err("Every name in that folder is taken.".into())
}

fn file_info(path: &Path) -> Option<FileInfo> {
    let meta = std::fs::metadata(path).ok()?;
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    let when = chrono::DateTime::<chrono::Local>::from(meta.modified().ok()?);
    Some(FileInfo {
        path: path.to_path_buf(),
        stem,
        ext,
        parent: path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        date: when.format("%Y-%m-%d").to_string(),
        time: when.format("%H%M%S").to_string(),
        modified: when.timestamp(),
    })
}

fn rename_step(path: &Path, template: &str) -> Result<PathBuf, String> {
    let info = file_info(path).ok_or("Couldn't read the file.")?;
    let opts = RenameOptions {
        template: template.to_string(),
        today: chrono::Local::now().format("%Y-%m-%d").to_string(),
        ..Default::default()
    };
    let items = rename::preview(&[info], &opts, &|p: &Path| p.exists());
    let item = items.first().ok_or("Couldn't make the new name.")?;
    match item.status {
        rename::Status::Same => return Ok(path.to_path_buf()),
        rename::Status::Ok => {}
        _ => return Err(format!("The new name can't be used: {}", item.note)),
    }
    let dir = path.parent().ok_or("No folder.")?;
    let dest = dir.join(&item.to_name);
    nomnom_core::fs::move_no_replace(path, &dest)
        .map_err(|e| format!("Couldn't rename it: {e}"))?;
    Ok(dest)
}

fn convert_step(path: &Path, to: &str, originals: &str) -> Result<PathBuf, String> {
    if !rules::is_picture(&path.to_string_lossy()) {
        return Err("Only pictures can be converted. The file was left as it is.".into());
    }
    let target = match to {
        "webp" => Target::Webp,
        "jpg" => Target::Jpg,
        _ => Target::Png,
    };
    let ext = if to == "jpg" { "jpg" } else { to };
    let dir = path.parent().ok_or("No folder.")?;
    let stem = nomnom_converter::naming::stem_of(path);
    // A picture that is already in the format is not made again.
    if rules::ext_of(&path.to_string_lossy()) == ext
        || (to == "jpg" && rules::ext_of(&path.to_string_lossy()) == "jpeg")
    {
        return Ok(path.to_path_buf());
    }
    let tmp = nomnom_converter::naming::temp_for(dir, ext);
    image_conv::convert(path, &tmp, target, &ConvOptions::default())
        .map_err(|e| format!("Couldn't convert it: {e}"))?;
    if let Err(e) = image_conv::validate(&tmp, target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("The converted picture didn't check out: {e}"));
    }
    let made = nomnom_converter::naming::place(&tmp, dir, &stem, ext)
        .map_err(|e| format!("Couldn't save it: {e}"))?;
    if !originals.trim().is_empty() {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        move_unique(path, Path::new(originals.trim()), &name)?;
    }
    Ok(made)
}

#[derive(Debug, Default)]
struct Outcome {
    /// Each thing that happened, for the list: (step, from, to).
    done: Vec<(String, PathBuf, PathBuf)>,
    error: Option<String>,
    last: Option<PathBuf>,
    /// An action to run once the steps are over.
    run: Vec<String>,
}

/// Puts a file through the steps of a rule. Stops at the first step that fails; what was done up to
/// then stays done.
fn do_steps(state: &AppState, rule: &Rule, start: &Path) -> Outcome {
    let mut out = Outcome::default();
    let mut path = start.to_path_buf();
    for step in &rule.steps {
        let before = path.clone();
        let result: Result<Option<(&'static str, PathBuf)>, String> = match step {
            Step::Move { folder } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                allowed(state, Path::new(folder.trim()))
                    .and_then(|_| move_unique(&path, Path::new(folder.trim()), &name))
                    .map(|p| Some(("move", p)))
            }
            Step::Copy { folder } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                allowed(state, Path::new(folder.trim()))
                    .and_then(|_| copy_unique(&path, Path::new(folder.trim()), &name))
                    .map(|_| None)
            }
            Step::Rename { template } => rename_step(&path, template).map(|p| Some(("rename", p))),
            Step::Convert { to, originals } => {
                convert_step(&path, to, originals).map(|p| Some(("convert", p)))
            }
            Step::Run { action } => {
                out.run.push(action.clone());
                Ok(None)
            }
            Step::Notify => Ok(None),
        };
        match result {
            Ok(Some((kind, new))) => {
                if new != before {
                    out.done.push((kind.to_string(), before, new.clone()));
                    path = new;
                }
            }
            Ok(None) => {
                if matches!(step, Step::Copy { .. }) {
                    out.done.push(("copy".into(), before, path.clone()));
                }
            }
            Err(e) => {
                out.error = Some(e);
                break;
            }
        }
    }
    out.last = Some(path);
    out
}

// ---- looking at the folders -----------------------------------------------------------------------

fn files_in(
    folder: &Path,
    subfolders: bool,
    skip: &[PathBuf],
) -> Vec<(PathBuf, std::fs::Metadata)> {
    let mut out = Vec::new();
    let mut stack = vec![(folder.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            // Links are never followed, so a rule can't be led out of its folder.
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                if subfolders
                    && depth < MAX_DEPTH
                    && !skip
                        .iter()
                        .any(|s| pathutil::is_inside(&p, s) || pathutil::is_same(&p, s))
                {
                    stack.push((p, depth + 1));
                }
            } else if ft.is_file() {
                if let Ok(m) = e.metadata() {
                    out.push((p, m));
                }
            }
        }
    }
    out
}

fn destinations(r: &Rule) -> Vec<PathBuf> {
    r.steps
        .iter()
        .filter_map(|s| match s {
            Step::Move { folder } | Step::Copy { folder } => Some(PathBuf::from(folder.trim())),
            Step::Convert { originals, .. } if !originals.trim().is_empty() => {
                Some(PathBuf::from(originals.trim()))
            }
            _ => None,
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct LogItem {
    pub id: i64,
    pub at: i64,
    pub rule: String,
    pub step: String,
    pub from: String,
    pub to: String,
    pub ok: bool,
    pub note: String,
    pub undone: bool,
}

fn log(state: &AppState, rule: &str, step: &str, from: &Path, to: &Path, ok: bool, note: &str) {
    let _ = state.db.with(|c| {
        c.execute(
            "INSERT INTO rule_log(at, rule, step, src, dst, ok, note, undone) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![now(), rule, step, from.to_string_lossy(), to.to_string_lossy(), ok as i64, note],
        )?;
        c.execute(
            "DELETE FROM rule_log WHERE id NOT IN (SELECT id FROM rule_log ORDER BY id DESC LIMIT ?1)",
            params![KEEP_LOG],
        )
    });
}

fn count_text(lang: Lang, n: usize) -> String {
    match (lang, n) {
        (Lang::En, 1) => "1 file handled".into(),
        (Lang::En, n) => format!("{n} files handled"),
        (Lang::Pt, 1) => "1 arquivo tratado".into(),
        (Lang::Pt, n) => format!("{n} arquivos tratados"),
        (Lang::Es, 1) => "1 archivo tratado".into(),
        (Lang::Es, n) => format!("{n} archivos tratados"),
    }
}

/// Runs one rule over the files that are ready. `existing` ignores the age and settling checks (for
/// "apply to what is already there"). Returns how many files went through.
async fn round(app: &AppHandle, rule: &Rule, existing: bool) -> usize {
    let state = app.state::<AppState>();
    let watched = PathBuf::from(rule.folder.trim());
    if check_rule(&state, rule).is_err() {
        return 0;
    }
    let started = now();
    let since = if existing {
        0
    } else {
        SCANNED
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .get(&rule.id)
            .copied()
            .unwrap_or(rule.since)
    };
    let skip = destinations(rule);
    let candidates = {
        let w = watched.clone();
        let s = skip.clone();
        let sub = rule.subfolders;
        tokio::task::spawn_blocking(move || files_in(&w, sub, &s))
            .await
            .unwrap_or_default()
    };
    let mut handled = 0usize;
    let mut notes: Vec<String> = Vec::new();
    for (path, meta) in candidates {
        if handled >= MAX_PER_ROUND {
            break;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !rules::matches(rule, &name, meta.len()) {
            continue;
        }
        let modified = meta.modified().unwrap_or(UNIX_EPOCH);
        let stamp = meta.created().map(secs).unwrap_or(0).max(secs(modified));
        if !existing && stamp + 5 < since {
            continue;
        }
        let key = (path.clone(), secs(modified));
        if DONE
            .lock()
            .unwrap()
            .get_or_insert_with(HashSet::new)
            .contains(&key)
        {
            continue;
        }
        if !existing {
            // Wait until the file has stopped changing.
            let ready = {
                let mut seen = SEEN.lock().unwrap();
                let map = seen.get_or_insert_with(HashMap::new);
                match map.get(&path) {
                    Some(s) if s.len == meta.len() && s.modified == modified => {
                        s.since.elapsed() >= QUIET
                    }
                    _ => {
                        if map.len() > 2000 {
                            map.clear();
                        }
                        map.insert(
                            path.clone(),
                            Seen {
                                len: meta.len(),
                                modified,
                                since: Instant::now(),
                            },
                        );
                        false
                    }
                }
            };
            if !ready {
                continue;
            }
        }
        let locked = {
            let p = path.clone();
            tokio::task::spawn_blocking(move || nomnom_organizer::winfs::is_locked(&p))
                .await
                .unwrap_or(true)
        };
        if locked {
            continue;
        }
        let (r, p) = (rule.clone(), path.clone());
        let a = app.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let st = a.state::<AppState>();
            do_steps(&st, &r, &p)
        })
        .await
        .unwrap_or_default();
        for (kind, from, to) in &outcome.done {
            log(&state, &rule.name, kind, from, to, true, "");
        }
        if let Some(e) = &outcome.error {
            log(
                &state,
                &rule.name,
                "error",
                &path,
                outcome.last.as_deref().unwrap_or(&path),
                false,
                e,
            );
            notes.push(e.clone());
        }
        for id in &outcome.run {
            let mut values = HashMap::new();
            values.insert(
                "file".to_string(),
                outcome
                    .last
                    .clone()
                    .unwrap_or(path.clone())
                    .to_string_lossy()
                    .into_owned(),
            );
            if let Err(e) = crate::actions::run_by_id(app, id, &values).await {
                log(&state, &rule.name, "run", &path, &path, false, &e);
                notes.push(e);
            } else {
                log(
                    &state,
                    &rule.name,
                    "run",
                    &path,
                    outcome.last.as_deref().unwrap_or(&path),
                    true,
                    "",
                );
            }
        }
        // The file as it was, and wherever it ended up, are not looked at again.
        {
            let mut done = DONE.lock().unwrap();
            let set = done.get_or_insert_with(HashSet::new);
            if set.len() > 5000 {
                set.clear();
            }
            set.insert(key);
            if let Some(last) = &outcome.last {
                if let Ok(m) = std::fs::metadata(last) {
                    set.insert((last.clone(), secs(m.modified().unwrap_or(UNIX_EPOCH))));
                }
            }
        }
        handled += 1;
    }
    if !existing {
        SCANNED
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(rule.id.clone(), started - 10);
        let id = rule.id.clone();
        let _ = state.db.with(|c| {
            c.execute(
                "INSERT INTO rule_state(rule_id, scanned_at) VALUES (?1, ?2) ON CONFLICT(rule_id) DO UPDATE SET scanned_at = ?2",
                params![id, started - 10],
            )
        });
    }
    if handled > 0 || !notes.is_empty() {
        let _ = app.emit("rules://changed", ());
        if rule.steps.iter().any(|s| matches!(s, Step::Notify)) || !notes.is_empty() {
            let lang = lang_of(&state.settings.get());
            let (kind, body) = match notes.first() {
                Some(e) => ("warning", e.clone()),
                None => ("success", count_text(lang, handled)),
            };
            toast::show(
                app,
                Toast::new(kind, rule.name.clone(), body).opens("rules"),
            );
        }
    }
    handled
}

/// The loop: every few seconds, each rule that is on.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Where each rule had got to when NomNom last ran.
        {
            let state = app.state::<AppState>();
            let rows: Vec<(String, i64)> = state
                .db
                .with(|c| {
                    let mut stmt = c.prepare("SELECT rule_id, scanned_at FROM rule_state")?;
                    let r = stmt
                        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    Ok(r)
                })
                .unwrap_or_default();
            SCANNED
                .lock()
                .unwrap()
                .get_or_insert_with(HashMap::new)
                .extend(rows);
        }
        loop {
            tokio::time::sleep(POLL).await;
            let list: Vec<Rule> = app
                .state::<AppState>()
                .settings
                .get()
                .rules
                .into_iter()
                .filter(|r| r.enabled)
                .collect();
            for rule in list {
                round(&app, &rule, false).await;
            }
        }
    });
}

// ---- commands -------------------------------------------------------------------------------------

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("r{nanos:x}")
}

#[tauri::command]
pub fn fr_list(state: State<'_, AppState>) -> Vec<Rule> {
    state.settings.get().rules
}

#[tauri::command]
pub fn fr_save(app: AppHandle, state: State<'_, AppState>, mut rule: Rule) -> CmdResult<Vec<Rule>> {
    rule.name = rule.name.trim().to_string();
    rule.folder = rule.folder.trim().to_string();
    rule.extensions = rules::parse_extensions(&rule.extensions.join(" "));
    if rule.id.trim().is_empty() {
        rule.id = new_id();
    }
    check_rule(&state, &rule)?;
    let mut list = state.settings.get().rules;
    match list.iter_mut().find(|r| r.id == rule.id) {
        Some(slot) => {
            // Turning it on (or changing it) starts from now: older files are left alone.
            if rule.enabled && (!slot.enabled || slot.folder != rule.folder) {
                rule.since = now();
            } else {
                rule.since = slot.since;
            }
            *slot = rule;
        }
        None => {
            rule.since = now();
            list.push(rule);
        }
    }
    rules::check_list(&list)?;
    let saved = list.clone();
    state
        .settings
        .update("rules", |s| s.rules = list)
        .map_err(|e| e.to_string())?;
    SCANNED
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .clear();
    let _ = app.emit("rules://changed", ());
    Ok(saved)
}

#[tauri::command]
pub fn fr_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<Vec<Rule>> {
    let list: Vec<Rule> = state
        .settings
        .get()
        .rules
        .into_iter()
        .filter(|r| r.id != id)
        .collect();
    let saved = list.clone();
    state
        .settings
        .update("rules", |s| s.rules = list)
        .map_err(|e| e.to_string())?;
    let _ = state
        .db
        .with(|c| c.execute("DELETE FROM rule_state WHERE rule_id = ?1", params![id]));
    let _ = app.emit("rules://changed", ());
    Ok(saved)
}

#[derive(Debug, Serialize)]
pub struct PreviewItem {
    pub name: String,
    pub size: u64,
}

/// The files in the folder right now that this rule covers: what "apply to what is already there"
/// would touch.
#[tauri::command]
pub async fn fr_preview(state: State<'_, AppState>, id: String) -> CmdResult<Vec<PreviewItem>> {
    let rule = state
        .settings
        .get()
        .rules
        .into_iter()
        .find(|r| r.id == id)
        .ok_or("That rule is gone.")?;
    check_rule(&state, &rule)?;
    let (folder, sub, skip) = (
        PathBuf::from(rule.folder.trim()),
        rule.subfolders,
        destinations(&rule),
    );
    let files = tokio::task::spawn_blocking(move || files_in(&folder, sub, &skip))
        .await
        .map_err(|e| e.to_string())?;
    let mut out: Vec<PreviewItem> = files
        .into_iter()
        .filter_map(|(p, m)| {
            let name = p.file_name()?.to_string_lossy().into_owned();
            rules::matches(&rule, &name, m.len()).then_some(PreviewItem {
                name,
                size: m.len(),
            })
        })
        .collect();
    out.sort_by_key(|f| f.name.to_lowercase());
    out.truncate(500);
    Ok(out)
}

/// Runs a rule on the files already in the folder (up to 60 at a time).
#[tauri::command]
pub async fn fr_apply_existing(app: AppHandle, id: String) -> CmdResult<usize> {
    let rule = app
        .state::<AppState>()
        .settings
        .get()
        .rules
        .into_iter()
        .find(|r| r.id == id)
        .ok_or("That rule is gone.")?;
    check_rule(&app.state::<AppState>(), &rule)?;
    Ok(round(&app, &rule, true).await)
}

#[tauri::command]
pub fn fr_log(state: State<'_, AppState>) -> CmdResult<Vec<LogItem>> {
    state
        .db
        .with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, at, rule, step, src, dst, ok, note, undone FROM rule_log ORDER BY id DESC LIMIT 200",
            )?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(LogItem {
                        id: r.get(0)?,
                        at: r.get(1)?,
                        rule: r.get(2)?,
                        step: r.get(3)?,
                        from: r.get(4)?,
                        to: r.get(5)?,
                        ok: r.get::<_, i64>(6)? != 0,
                        note: r.get(7)?,
                        undone: r.get::<_, i64>(8)? != 0,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .map_err(|e| e.to_string())
}

/// Puts a moved or renamed file back where it was. Refused when something is in the way.
#[tauri::command]
pub fn fr_undo(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    let (step, from, to, undone): (String, String, String, i64) = state
        .db
        .with(|c| {
            c.query_row(
                "SELECT step, src, dst, undone FROM rule_log WHERE id = ?1 AND ok = 1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
        })
        .map_err(|_| "That entry is gone.".to_string())?;
    if undone != 0 {
        return Err("That was already put back.".into());
    }
    if step != "move" && step != "rename" {
        return Err("Only a move or a rename can be put back.".into());
    }
    let (from, to) = (PathBuf::from(from), PathBuf::from(to));
    if !to.exists() {
        return Err("The file isn't there any more.".into());
    }
    if from.exists() {
        return Err("Something with the old name is already there.".into());
    }
    allowed(&state, from.parent().unwrap_or(&from))?;
    nomnom_core::fs::move_no_replace(&to, &from)
        .map_err(|e| format!("Couldn't put it back: {e}"))?;
    let _ = state
        .db
        .with(|c| c.execute("UPDATE rule_log SET undone = 1 WHERE id = ?1", params![id]));
    // The file is in its old place again and must not be picked up a second time.
    if let Ok(m) = std::fs::metadata(&from) {
        DONE.lock()
            .unwrap()
            .get_or_insert_with(HashSet::new)
            .insert((from, secs(m.modified().unwrap_or(UNIX_EPOCH))));
    }
    let _ = app.emit("rules://changed", ());
    Ok(())
}

#[tauri::command]
pub fn fr_clear_log(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    state
        .db
        .with(|c| c.execute("DELETE FROM rule_log", []))
        .map_err(|e| e.to_string())?;
    let _ = app.emit("rules://changed", ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn a_moved_file_takes_a_free_name_and_nothing_is_replaced() {
        let d = tmp();
        let src_dir = d.path().join("in");
        let dst_dir = d.path().join("out");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        std::fs::write(dst_dir.join("a.txt"), "old").unwrap();
        let f = src_dir.join("a.txt");
        std::fs::write(&f, "new").unwrap();
        let moved = move_unique(&f, &dst_dir, "a.txt").unwrap();
        assert_eq!(moved.file_name().unwrap(), "a (2).txt");
        assert_eq!(
            std::fs::read_to_string(dst_dir.join("a.txt")).unwrap(),
            "old"
        );
        assert_eq!(std::fs::read_to_string(&moved).unwrap(), "new");
        assert!(!f.exists());
    }

    #[test]
    fn a_copy_leaves_the_original_and_never_overwrites() {
        let d = tmp();
        let dst = d.path().join("copies");
        let f = d.path().join("b.bin");
        std::fs::write(&f, [1u8, 2, 3]).unwrap();
        let one = copy_unique(&f, &dst, "b.bin").unwrap();
        let two = copy_unique(&f, &dst, "b.bin").unwrap();
        assert_ne!(one, two);
        assert!(f.exists());
        assert_eq!(std::fs::read(&two).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn a_rename_keeps_the_extension_and_refuses_a_taken_name() {
        let d = tmp();
        let f = d.path().join("photo.png");
        std::fs::write(&f, "x").unwrap();
        let out = rename_step(&f, "holiday_{name}").unwrap();
        assert_eq!(out.file_name().unwrap(), "holiday_photo.png");
        assert!(!f.exists());
        let g = d.path().join("other.png");
        std::fs::write(&g, "y").unwrap();
        // The pattern would give the name that is already taken.
        std::fs::write(d.path().join("same.png"), "z").unwrap();
        assert!(rename_step(&g, "same").is_err());
        assert!(g.exists());
    }

    #[test]
    fn links_are_not_followed_when_listing() {
        let d = tmp();
        let real = d.path().join("real");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("a.txt"), "x").unwrap();
        std::fs::write(d.path().join("b.txt"), "x").unwrap();
        let names: Vec<String> = files_in(d.path(), true, &[])
            .into_iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"a.txt".to_string()) && names.contains(&"b.txt".to_string()));
        // A destination inside the watched folder is skipped, so a rule can't feed itself.
        let skipped: Vec<String> = files_in(d.path(), true, std::slice::from_ref(&real))
            .into_iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(skipped, vec!["b.txt".to_string()]);
    }
}
