//! Quick notes: short texts kept on this computer, pinned ones first, searchable.

use crate::state::AppState;
use rusqlite::params;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

type CmdResult<T> = Result<T, String>;

const MAX_NOTES: i64 = 2000;
const MAX_CHARS: usize = 20_000;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[derive(Debug, Serialize)]
pub struct Note {
    pub id: i64,
    pub text: String,
    pub pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Newest first, pinned on top. `query` keeps only notes that contain every word of it.
#[tauri::command]
pub fn nt_list(state: State<'_, AppState>, query: Option<String>) -> CmdResult<Vec<Note>> {
    let words: Vec<String> = query
        .unwrap_or_default()
        .split_whitespace()
        .map(nomnom_core::clipboard::like_pattern)
        .collect();
    let mut sql = String::from("SELECT id, text, pinned, created_at, updated_at FROM notes");
    if !words.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(
            &(1..=words.len())
                .map(|i| format!("text LIKE ?{i} ESCAPE '\\'"))
                .collect::<Vec<_>>()
                .join(" AND "),
        );
    }
    sql.push_str(" ORDER BY pinned DESC, updated_at DESC LIMIT 500");
    state
        .db
        .with(|c| {
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt
                .query_map(rusqlite::params_from_iter(words.iter()), |r| {
                    Ok(Note {
                        id: r.get(0)?,
                        text: r.get(1)?,
                        pinned: r.get::<_, i64>(2)? != 0,
                        created_at: r.get(3)?,
                        updated_at: r.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .map_err(|e| e.to_string())
}

/// Adds a note (`id` is `None`) or changes one.
#[tauri::command]
pub fn nt_save(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Option<i64>,
    text: String,
) -> CmdResult<i64> {
    let text = text.trim_end().to_string();
    if text.trim().is_empty() {
        return Err("The note is empty.".into());
    }
    if text.chars().count() > MAX_CHARS {
        return Err("That note is too long.".into());
    }
    let t = now();
    let saved = match id {
        Some(id) => {
            state
                .db
                .with(|c| {
                    c.execute(
                        "UPDATE notes SET text = ?2, updated_at = ?3 WHERE id = ?1",
                        params![id, text, t],
                    )
                })
                .map_err(|e| e.to_string())?;
            id
        }
        None => {
            let count: i64 = state
                .db
                .with(|c| c.query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0)))
                .map_err(|e| e.to_string())?;
            if count >= MAX_NOTES {
                return Err("There are too many notes. Delete some first.".into());
            }
            state
                .db
                .with(|c| {
                    c.execute(
                        "INSERT INTO notes(text, pinned, created_at, updated_at) VALUES (?1, 0, ?2, ?2)",
                        params![text, t],
                    )?;
                    Ok(c.last_insert_rowid())
                })
                .map_err(|e| e.to_string())?
        }
    };
    let _ = app.emit("notes://changed", ());
    Ok(saved)
}

#[tauri::command]
pub fn nt_pin(app: AppHandle, state: State<'_, AppState>, id: i64, pinned: bool) -> CmdResult<()> {
    state
        .db
        .with(|c| {
            c.execute(
                "UPDATE notes SET pinned = ?2 WHERE id = ?1",
                params![id, pinned as i64],
            )
        })
        .map_err(|e| e.to_string())?;
    let _ = app.emit("notes://changed", ());
    Ok(())
}

#[tauri::command]
pub fn nt_delete(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state
        .db
        .with(|c| c.execute("DELETE FROM notes WHERE id = ?1", params![id]))
        .map_err(|e| e.to_string())?;
    let _ = app.emit("notes://changed", ());
    Ok(())
}
