//! Text snippets kept on this computer. Using one fills in its values and puts the text on the
//! clipboard; from the launcher it is also pasted where the person was typing.

use crate::state::AppState;
use chrono::Local;
use nomnom_core::snippets::{self as rules, Context};
use rusqlite::params;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

type CmdResult<T> = Result<T, String>;

const MAX_SNIPPETS: i64 = 500;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[derive(Debug, Serialize)]
pub struct Snippet {
    pub id: i64,
    pub name: String,
    pub body: String,
}

#[tauri::command]
pub fn sn_list(state: State<'_, AppState>) -> CmdResult<Vec<Snippet>> {
    state
        .db
        .with(|c| {
            let mut stmt = c.prepare("SELECT id, name, body FROM snippets ORDER BY used_at DESC, name COLLATE NOCASE LIMIT 500")?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(Snippet {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        body: r.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sn_save(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Option<i64>,
    name: String,
    body: String,
) -> CmdResult<i64> {
    let name = name.trim().to_string();
    rules::validate(&name, &body)?;
    let saved = match id {
        Some(id) => {
            state
                .db
                .with(|c| {
                    c.execute(
                        "UPDATE snippets SET name = ?2, body = ?3 WHERE id = ?1",
                        params![id, name, body],
                    )
                })
                .map_err(|e| e.to_string())?;
            id
        }
        None => {
            let count: i64 = state
                .db
                .with(|c| c.query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0)))
                .map_err(|e| e.to_string())?;
            if count >= MAX_SNIPPETS {
                return Err("There are too many snippets. Delete some first.".into());
            }
            state
                .db
                .with(|c| {
                    c.execute(
                        "INSERT INTO snippets(name, body, created_at) VALUES (?1, ?2, ?3)",
                        params![name, body, now()],
                    )?;
                    Ok(c.last_insert_rowid())
                })
                .map_err(|e| e.to_string())?
        }
    };
    let _ = app.emit("snippets://changed", ());
    Ok(saved)
}

#[tauri::command]
pub fn sn_delete(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state
        .db
        .with(|c| c.execute("DELETE FROM snippets WHERE id = ?1", params![id]))
        .map_err(|e| e.to_string())?;
    let _ = app.emit("snippets://changed", ());
    Ok(())
}

fn context(app: &AppHandle) -> Context {
    let t = Local::now();
    Context {
        date: t.format("%Y-%m-%d").to_string(),
        time: t.format("%H:%M").to_string(),
        weekday: t.format("%A").to_string(),
        year: t.format("%Y").to_string(),
        month: t.format("%m").to_string(),
        day: t.format("%d").to_string(),
        clipboard: app.clipboard().read_text().unwrap_or_default(),
    }
}

/// The text of a snippet with its values filled in, without using it (for the preview).
#[tauri::command]
pub fn sn_preview(app: AppHandle, body: String) -> String {
    rules::expand(&body, &context(&app))
}

/// Puts a snippet on the clipboard, and with `paste` also into the window that has the keyboard.
#[tauri::command]
pub fn sn_use(app: AppHandle, state: State<'_, AppState>, id: i64, paste: bool) -> CmdResult<()> {
    let body: String = state
        .db
        .with(|c| {
            c.query_row(
                "SELECT body FROM snippets WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
        })
        .map_err(|_| "That snippet is gone.".to_string())?;
    let text = rules::expand(&body, &context(&app));
    let _ = state.db.with(|c| {
        c.execute(
            "UPDATE snippets SET used_at = ?2 WHERE id = ?1",
            params![id, now()],
        )
    });
    crate::clipboard::suppress(2500);
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    if paste {
        std::thread::spawn(|| {
            // The launcher window is closing and the person's window is getting the keyboard back.
            std::thread::sleep(std::time::Duration::from_millis(280));
            nomnom_win::clipboard::wait_modifiers_released(1500);
            nomnom_win::clipboard::press_paste();
        });
    }
    #[cfg(not(windows))]
    let _ = paste;
    Ok(())
}
