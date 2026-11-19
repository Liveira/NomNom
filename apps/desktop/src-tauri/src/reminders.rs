//! Reminders: said in plain words, kept on this computer, and shown as a card when the time comes.

use crate::i18n::{lang_of, tr};
use crate::state::AppState;
use crate::toast::{self, Toast};
use chrono::{Local, NaiveDateTime, TimeZone};
use nomnom_core::reminders::{self as rules, Repeat};
use rusqlite::params;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

const MAX_OPEN: i64 = 500;
const KEEP_DONE: i64 = 20;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn local_now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// A local wall-clock time as a moment. A time that doesn't exist (the clocks jumped) moves on.
fn to_unix(local: NaiveDateTime) -> i64 {
    Local
        .from_local_datetime(&local)
        .earliest()
        .or_else(|| {
            Local
                .from_local_datetime(&(local + chrono::Duration::hours(1)))
                .earliest()
        })
        .map(|d| d.timestamp())
        .unwrap_or_else(|| local.and_utc().timestamp())
}

fn to_local(unix: i64) -> NaiveDateTime {
    Local
        .timestamp_opt(unix, 0)
        .single()
        .map(|d| d.naive_local())
        .unwrap_or_default()
}

#[derive(Debug, Serialize)]
pub struct Reminder {
    pub id: i64,
    pub text: String,
    pub due_at: i64,
    pub repeat: Option<String>,
    pub done: bool,
}

#[derive(Debug, Serialize)]
pub struct Preview {
    pub text: String,
    pub due_at: i64,
    pub repeat: Option<String>,
}

/// What a sentence would become, without saving it (the search box shows this as you type).
#[tauri::command]
pub fn rm_preview(text: String) -> Option<Preview> {
    rules::parse(&text, local_now()).map(|p| Preview {
        text: p.text,
        due_at: to_unix(p.when),
        repeat: p.repeat.map(|r| r.as_str().to_string()),
    })
}

fn insert(state: &AppState, text: &str, due_at: i64, repeat: Option<&str>) -> CmdResult<()> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Write what to be reminded about.".into());
    }
    if text.chars().count() > 300 {
        return Err("That reminder is too long (300 characters at most).".into());
    }
    let open: i64 = state
        .db
        .with(|c| {
            c.query_row("SELECT COUNT(*) FROM reminders WHERE done = 0", [], |r| {
                r.get(0)
            })
        })
        .map_err(|e| e.to_string())?;
    if open >= MAX_OPEN {
        return Err("There are too many reminders waiting. Delete some first.".into());
    }
    state
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO reminders(text, due_at, repeat, done, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
                params![text, due_at, repeat, now()],
            )
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Saves a reminder said in words: "in 20 minutes stretch".
#[tauri::command]
pub fn rm_add_text(app: AppHandle, state: State<'_, AppState>, text: String) -> CmdResult<()> {
    let p = rules::parse(&text, local_now())
        .ok_or("Say when, for example: in 20 minutes, at 18:30, or tomorrow at 9.")?;
    insert(
        &state,
        &p.text,
        to_unix(p.when),
        p.repeat.map(Repeat::as_str),
    )?;
    let _ = app.emit("reminders://changed", ());
    Ok(())
}

/// Saves a reminder with a chosen moment (from the page's own date and time boxes).
#[tauri::command]
pub fn rm_add_at(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    due_at: i64,
    repeat: Option<String>,
) -> CmdResult<()> {
    if due_at <= now() {
        return Err("That time has already passed.".into());
    }
    let repeat = repeat.as_deref().and_then(Repeat::parse);
    insert(&state, &text, due_at, repeat.map(Repeat::as_str))?;
    let _ = app.emit("reminders://changed", ());
    Ok(())
}

#[tauri::command]
pub fn rm_list(state: State<'_, AppState>) -> CmdResult<Vec<Reminder>> {
    let rows = |sql: &str, limit: i64| -> CmdResult<Vec<Reminder>> {
        state
            .db
            .with(|c| {
                let mut stmt = c.prepare(sql)?;
                let list = stmt
                    .query_map(params![limit], |r| {
                        Ok(Reminder {
                            id: r.get(0)?,
                            text: r.get(1)?,
                            due_at: r.get(2)?,
                            repeat: r.get(3)?,
                            done: r.get::<_, i64>(4)? != 0,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(list)
            })
            .map_err(|e| e.to_string())
    };
    let mut out = rows(
        "SELECT id, text, due_at, repeat, done FROM reminders WHERE done = 0 ORDER BY due_at LIMIT ?1",
        MAX_OPEN,
    )?;
    out.extend(rows(
        "SELECT id, text, due_at, repeat, done FROM reminders WHERE done = 1 ORDER BY due_at DESC LIMIT ?1",
        KEEP_DONE,
    )?);
    Ok(out)
}

#[tauri::command]
pub fn rm_delete(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state
        .db
        .with(|c| c.execute("DELETE FROM reminders WHERE id = ?1", params![id]))
        .map_err(|e| e.to_string())?;
    let _ = app.emit("reminders://changed", ());
    Ok(())
}

/// Moves a reminder to later: `minutes` from now. Also brings back one that already went off.
#[tauri::command]
pub fn rm_snooze(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    minutes: i64,
) -> CmdResult<()> {
    let minutes = minutes.clamp(1, 60 * 24 * 30);
    state
        .db
        .with(|c| {
            c.execute(
                "UPDATE reminders SET due_at = ?2, done = 0 WHERE id = ?1",
                params![id, now() + minutes * 60],
            )
        })
        .map_err(|e| e.to_string())?;
    let _ = app.emit("reminders://changed", ());
    Ok(())
}

/// What is due: shows each as a card, then finishes it or sets the next time of a repeating one.
fn fire_due(app: &AppHandle) {
    let state = app.state::<AppState>();
    let t = now();
    let due: Vec<(i64, String, i64, Option<String>)> = state
        .db
        .with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, text, due_at, repeat FROM reminders WHERE done = 0 AND due_at <= ?1 ORDER BY due_at LIMIT 20",
            )?;
            let rows = stmt
                .query_map(params![t], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .unwrap_or_default();
    if due.is_empty() {
        return;
    }
    let lang = lang_of(&state.settings.get());
    for (id, text, due_at, repeat) in due {
        toast::show(
            app,
            Toast::new("info", tr(lang, "Reminder"), text).opens("reminders"),
        );
        match repeat.as_deref().and_then(Repeat::parse) {
            Some(r) => {
                // Skips the times missed while the computer was off: one card, then the next future one.
                let mut next = r.next(to_local(due_at));
                while to_unix(next) <= t {
                    next = r.next(next);
                }
                let _ = state.db.with(|c| {
                    c.execute(
                        "UPDATE reminders SET due_at = ?2 WHERE id = ?1",
                        params![id, to_unix(next)],
                    )
                });
            }
            None => {
                let _ = state.db.with(|c| {
                    c.execute("UPDATE reminders SET done = 1 WHERE id = ?1", params![id])
                });
            }
        }
    }
    // Old finished ones are tidied away.
    let _ = state.db.with(|c| {
        c.execute(
            "DELETE FROM reminders WHERE done = 1 AND id NOT IN (SELECT id FROM reminders WHERE done = 1 ORDER BY due_at DESC LIMIT ?1)",
            params![KEEP_DONE],
        )
    });
    let _ = app.emit("reminders://changed", ());
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            fire_due(&app);
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_time_makes_the_same_moment_back() {
        let t = chrono::NaiveDate::from_ymd_opt(2026, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(to_local(to_unix(t)), t);
    }

    #[test]
    fn the_preview_is_in_the_future_and_says_what_it_is_for() {
        let p = rm_preview("in 20 minutes stretch".into()).unwrap();
        assert_eq!(p.text, "stretch");
        let wait = p.due_at - now();
        assert!((19 * 60..=21 * 60).contains(&wait), "{wait}");
        assert!(rm_preview("buy milk".into()).is_none());
    }
}
