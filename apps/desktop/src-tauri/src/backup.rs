//! Backup module: plans kept in the database, runs started by hand or by a schedule, versions, and
//! restoring. The folders being backed up are only read; the work itself is in `nomnom-backup`.

use crate::organizer::dest_is_acceptable;
use crate::state::AppState;
use nomnom_backup::{Plan, RestoreReport, VersionInfo};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

type CmdResult<T> = Result<T, String>;

#[derive(Default)]
pub struct BackupState {
    /// The plan that is running now, and the flag that stops it.
    running: Mutex<Option<(String, Arc<AtomicBool>)>>,
}

#[derive(Serialize, Clone)]
pub struct LastRun {
    pub started_at: i64,
    pub ended_at: i64,
    pub ok: bool,
    pub files: i64,
    pub bytes: i64,
    pub copied: i64,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct PlanView {
    pub plan: Plan,
    pub last: Option<LastRun>,
    pub next_at: Option<i64>,
    pub running: bool,
    pub problem: Option<String>,
}

#[derive(Serialize, Clone)]
struct ProgressEvent {
    plan_id: String,
    files: u64,
    bytes: u64,
    current: String,
}

#[derive(Serialize, Clone)]
struct DoneEvent {
    plan_id: String,
    ok: bool,
    message: String,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn load_plans(state: &AppState) -> Vec<Plan> {
    state
        .db
        .with(|c| {
            let mut st = c.prepare("SELECT json FROM backup_plans ORDER BY created_at")?;
            let rows = st.query_map([], |r| r.get::<_, String>(0))?;
            Ok(rows
                .flatten()
                .filter_map(|j| serde_json::from_str::<Plan>(&j).ok())
                .collect::<Vec<_>>())
        })
        .unwrap_or_default()
}

fn load_plan(state: &AppState, id: &str) -> CmdResult<Plan> {
    load_plans(state)
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "That backup no longer exists.".to_string())
}

fn last_run(state: &AppState, id: &str) -> Option<LastRun> {
    state
        .db
        .with(|c| {
            c.query_row(
                "SELECT started_at, ended_at, ok, files, bytes, copied, message FROM backup_runs
                 WHERE plan_id = ?1 ORDER BY id DESC LIMIT 1",
                params![id],
                |r| {
                    Ok(LastRun {
                        started_at: r.get(0)?,
                        ended_at: r.get(1)?,
                        ok: r.get(2)?,
                        files: r.get(3)?,
                        bytes: r.get(4)?,
                        copied: r.get(5)?,
                        message: r.get(6)?,
                    })
                },
            )
            .optional()
        })
        .ok()
        .flatten()
}

/// When the plan's next automatic run is due, if it has a schedule.
fn next_due(plan: &Plan, last: &Option<LastRun>) -> Option<i64> {
    if !plan.enabled {
        return None;
    }
    let every = plan.interval_secs()?;
    Some(
        last.as_ref()
            .map(|l| l.started_at + every)
            .unwrap_or_else(now),
    )
}

#[tauri::command]
pub fn bk_plans(state: State<'_, AppState>, bk: State<'_, BackupState>) -> Vec<PlanView> {
    let running = bk.running.lock().unwrap().as_ref().map(|r| r.0.clone());
    load_plans(&state)
        .into_iter()
        .map(|plan| {
            let last = last_run(&state, &plan.id);
            PlanView {
                next_at: next_due(&plan, &last),
                running: running.as_deref() == Some(plan.id.as_str()),
                problem: plan.problem(),
                last,
                plan,
            }
        })
        .collect()
}

#[tauri::command]
pub fn bk_save(state: State<'_, AppState>, mut plan: Plan) -> CmdResult<Plan> {
    plan.name = plan.name.trim().chars().take(60).collect();
    if let Some(p) = plan.problem() {
        return Err(p);
    }
    let (zones, _) = state.paths.protected_folders();
    if !dest_is_acceptable(&plan.destination, &zones) {
        return Err("NomNom won't keep backups there. Choose another folder.".into());
    }
    if !matches!(
        plan.schedule.as_str(),
        "manual" | "hourly" | "daily" | "weekly"
    ) {
        return Err("Pick Manual, Hourly, Daily or Weekly.".into());
    }
    // One folder name per plan inside a destination.
    let exists = load_plans(&state).into_iter().any(|p| {
        p.id != plan.id
            && p.destination == plan.destination
            && p.folder_name().eq_ignore_ascii_case(&plan.folder_name())
    });
    if exists {
        return Err("Another backup already uses that name in this folder.".into());
    }
    if plan.id.is_empty() {
        plan.id = uuid::Uuid::new_v4().to_string();
    }
    let json = serde_json::to_string(&plan).map_err(|e| e.to_string())?;
    state
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO backup_plans(id, json, created_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET json = excluded.json",
                params![plan.id, json, now()],
            )
        })
        .map_err(|e| format!("It couldn't be saved: {e}"))?;
    Ok(plan)
}

/// Forgets the plan. Backups already made are left where they are.
#[tauri::command]
pub fn bk_delete(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state
        .db
        .with(|c| {
            c.execute("DELETE FROM backup_plans WHERE id = ?1", params![id])?;
            c.execute("DELETE FROM backup_runs WHERE plan_id = ?1", params![id])
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn spawn_run(app: &AppHandle, plan: Plan, scheduled: bool) -> CmdResult<()> {
    let bk = app.state::<BackupState>();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = bk.running.lock().unwrap();
        if slot.is_some() {
            return Err("A backup is already running.".into());
        }
        *slot = Some((plan.id.clone(), cancel.clone()));
    }
    let app = app.clone();
    let _ = app.emit("backup://changed", ());
    tauri::async_runtime::spawn_blocking(move || {
        let started = now();
        let emitter = app.clone();
        let pid = plan.id.clone();
        let result = nomnom_backup::run(&plan, started, &cancel, &move |p| {
            let _ = emitter.emit(
                "backup://progress",
                ProgressEvent {
                    plan_id: pid.clone(),
                    files: p.files,
                    bytes: p.bytes,
                    current: p.current,
                },
            );
        });
        let st = app.state::<AppState>();
        let (ok, message, files, bytes, copied) = match &result {
            Ok(r) if r.cancelled => (
                false,
                "Stopped before it finished. Nothing was kept.".to_string(),
                0,
                0,
                0,
            ),
            Ok(r) => {
                let note = if r.skipped.is_empty() {
                    String::new()
                } else {
                    format!(
                        "{} items couldn't be read and were left out.",
                        r.skipped.len()
                    )
                };
                (
                    true,
                    note,
                    r.files as i64,
                    r.bytes as i64,
                    r.copied_bytes as i64,
                )
            }
            Err(e) => (false, e.clone(), 0, 0, 0),
        };
        let _ = st.db.with(|c| {
            c.execute(
                "INSERT INTO backup_runs(plan_id, started_at, ended_at, ok, files, bytes, copied, version, message)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    plan.id, started, now(), ok, files, bytes, copied,
                    result.as_ref().ok().map(|r| r.version.clone()),
                    if message.is_empty() { None } else { Some(message.clone()) }
                ],
            )
        });
        *app.state::<BackupState>().running.lock().unwrap() = None;
        let _ = app.emit(
            "backup://done",
            DoneEvent {
                plan_id: plan.id.clone(),
                ok,
                message: message.clone(),
            },
        );
        let _ = app.emit("backup://changed", ());
        let cancelled = matches!(&result, Ok(r) if r.cancelled);
        if !cancelled && (scheduled || !ok) {
            let lang = crate::i18n::lang_of(&st.settings.get());
            let (title, body) = crate::i18n::backup_done(lang, ok, &plan.name, &message);
            let _ = app.notification().builder().title(title).body(body).show();
        }
    });
    Ok(())
}

#[tauri::command]
pub fn bk_run(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let plan = load_plan(&state, &id)?;
    spawn_run(&app, plan, false)
}

#[tauri::command]
pub fn bk_cancel(bk: State<'_, BackupState>) {
    if let Some((_, flag)) = bk.running.lock().unwrap().as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn bk_versions(state: State<'_, AppState>, id: String) -> CmdResult<Vec<VersionInfo>> {
    let plan = load_plan(&state, &id)?;
    let home = plan.home();
    tokio::task::spawn_blocking(move || nomnom_backup::list_versions(&home))
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct TreeFolder {
    pub name: String,
    pub files: u64,
    pub bytes: u64,
}
#[derive(Serialize)]
pub struct TreeFile {
    pub name: String,
    pub bytes: u64,
    pub modified: i64,
}
#[derive(Serialize)]
pub struct Tree {
    pub folders: Vec<TreeFolder>,
    pub files: Vec<TreeFile>,
}

/// One level of a version: its folders (with how much is inside) and its files.
#[tauri::command]
pub async fn bk_tree(
    state: State<'_, AppState>,
    id: String,
    version: String,
    prefix: String,
) -> CmdResult<Tree> {
    let plan = load_plan(&state, &id)?;
    let home = plan.home();
    tokio::task::spawn_blocking(move || -> CmdResult<Tree> {
        let m = nomnom_backup::read_manifest(&home, &version)?;
        let pre = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.trim_end_matches('/'))
        };
        let mut folders: std::collections::BTreeMap<String, (u64, u64)> = Default::default();
        let mut files = Vec::new();
        for e in &m.files {
            let Some(rest) = e.p.strip_prefix(&pre) else {
                continue;
            };
            match rest.split_once('/') {
                Some((dir, _)) => {
                    let f = folders.entry(dir.to_string()).or_default();
                    f.0 += 1;
                    f.1 += e.s;
                }
                None => files.push(TreeFile {
                    name: rest.to_string(),
                    bytes: e.s,
                    modified: e.m,
                }),
            }
        }
        files.sort_by_key(|f| f.name.to_lowercase());
        Ok(Tree {
            folders: folders
                .into_iter()
                .map(|(name, (files, bytes))| TreeFolder { name, files, bytes })
                .collect(),
            files,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn bk_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    version: String,
    only: Option<Vec<String>>,
    target: String,
) -> CmdResult<RestoreReport> {
    let plan = load_plan(&state, &id)?;
    let target = std::path::PathBuf::from(target);
    let (zones, _) = state.paths.protected_folders();
    if !dest_is_acceptable(&target, &zones) {
        return Err("NomNom won't restore files there. Choose another folder.".into());
    }
    let home = plan.home();
    let emitter = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        nomnom_backup::restore(
            &home,
            &version,
            only.as_deref(),
            &target,
            &AtomicBool::new(false),
            &move |p| {
                let _ = emitter.emit(
                    "backup://progress",
                    ProgressEvent {
                        plan_id: String::new(),
                        files: p.files,
                        bytes: p.bytes,
                        current: p.current,
                    },
                );
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn bk_verify(
    state: State<'_, AppState>,
    id: String,
    version: String,
) -> CmdResult<serde_json::Value> {
    let plan = load_plan(&state, &id)?;
    let home = plan.home();
    let r = tauri::async_runtime::spawn_blocking(move || nomnom_backup::verify(&home, &version))
        .await
        .map_err(|e| e.to_string())??;
    Ok(
        serde_json::json!({ "checked": r.checked, "missing": r.missing, "wrong_size": r.wrong_size }),
    )
}

/// Starts scheduled backups when they are due. Checks once a minute and does nothing with no schedules.
pub fn spawn_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(45)).await;
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            let st = app.state::<AppState>();
            let due: Option<Plan> = load_plans(&st).into_iter().find(|p| {
                p.problem().is_none()
                    && next_due(p, &last_run(&st, &p.id)).is_some_and(|t| t <= now())
            });
            if let Some(plan) = due {
                let _ = spawn_run(&app, plan, true);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(schedule: &str, enabled: bool) -> Plan {
        Plan {
            schedule: schedule.into(),
            enabled,
            ..Plan::default()
        }
    }

    #[test]
    fn manual_and_switched_off_plans_have_no_next_time() {
        assert_eq!(next_due(&plan("manual", true), &None), None);
        assert_eq!(next_due(&plan("daily", false), &None), None);
    }

    #[test]
    fn the_next_run_is_one_interval_after_the_last_start() {
        let last = Some(LastRun {
            started_at: 1000,
            ended_at: 1010,
            ok: true,
            files: 1,
            bytes: 1,
            copied: 1,
            message: None,
        });
        assert_eq!(next_due(&plan("hourly", true), &last), Some(4600));
        assert_eq!(
            next_due(&plan("weekly", true), &last),
            Some(1000 + 7 * 86_400)
        );
        // A schedule that never ran is due right away.
        let t = next_due(&plan("daily", true), &None).unwrap();
        assert!((t - now()).abs() < 5);
    }
}
