//! The notices NomNom showed, kept for a few days so one that went away unread can be found again.
//! They stay on this computer, and the list can be cleared at any time.

use crate::state::AppState;
use crate::toast::Toast;
use rusqlite::params;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

const KEEP_DAYS: i64 = 7;
const KEEP_MAX: i64 = 200;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[derive(Debug, Serialize)]
pub struct Item {
    pub id: i64,
    pub at: i64,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub action: Option<String>,
}

/// Writes one notice down. Called for every card, whatever way it is shown.
pub fn record(app: &AppHandle, toast: &Toast) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let t = now();
    let _ = state.db.with(|c| {
        c.execute(
            "INSERT INTO notifications(at, kind, title, body, action) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![t, toast.kind, toast.title, toast.body, toast.action],
        )?;
        // Old ones, and anything past the limit, are forgotten.
        c.execute(
            "DELETE FROM notifications WHERE at < ?1 OR id NOT IN (SELECT id FROM notifications ORDER BY id DESC LIMIT ?2)",
            params![t - KEEP_DAYS * 86_400, KEEP_MAX],
        )
    });
    let _ = app.emit("notifications://changed", ());
}

#[tauri::command]
pub fn nf_list(state: State<'_, AppState>) -> CmdResult<Vec<Item>> {
    state
        .db
        .with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, at, kind, title, body, action FROM notifications ORDER BY at DESC, id DESC LIMIT ?1",
            )?;
            let rows = stmt
                .query_map(params![KEEP_MAX], |r| {
                    Ok(Item {
                        id: r.get(0)?,
                        at: r.get(1)?,
                        kind: r.get(2)?,
                        title: r.get(3)?,
                        body: r.get(4)?,
                        action: r.get(5)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn nf_clear(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    state
        .db
        .with(|c| c.execute("DELETE FROM notifications", []))
        .map_err(|e| e.to_string())?;
    let _ = app.emit("notifications://changed", ());
    Ok(())
}

#[tauri::command]
pub fn nf_delete(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state
        .db
        .with(|c| c.execute("DELETE FROM notifications WHERE id = ?1", params![id]))
        .map_err(|e| e.to_string())?;
    let _ = app.emit("notifications://changed", ());
    Ok(())
}
