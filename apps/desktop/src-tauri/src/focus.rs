//! Focus mode: a timer for a block of work (or a break) that sits next to Playtime. When a block ends
//! NomNom says so, and the summary shows which apps you really used during it. Nothing about the
//! window titles or contents is ever recorded, only the same app names Playtime already keeps.

use crate::state::AppState;
use nomnom_core::db::Db;
use rusqlite::params;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

type CmdResult<T> = Result<T, String>;

#[derive(Default)]
pub struct FocusState {
    active: Mutex<Option<Active>>,
}

#[derive(Debug, Clone)]
struct Active {
    id: i64,
    kind: String,
    label: String,
    started_at: i64,
    planned_secs: i64,
}

#[derive(Serialize, Clone)]
pub struct ActiveBlock {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub started_at: i64,
    pub planned_secs: i64,
    pub remaining_secs: i64,
}

#[derive(Serialize, Clone)]
pub struct AppShare {
    pub name: String,
    pub secs: i64,
}

#[derive(Serialize, Clone)]
pub struct Block {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub planned_secs: i64,
    pub completed: bool,
    pub apps: Vec<AppShare>,
}

#[derive(Serialize)]
pub struct FocusStatus {
    pub active: Option<ActiveBlock>,
    /// Finished focus blocks today, and the seconds they added up to.
    pub today_blocks: i64,
    pub today_secs: i64,
    /// Days in a row, ending today, with at least one finished focus block.
    pub streak_days: i64,
    pub recent: Vec<Block>,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The three apps you used most between `start` and `end`.
fn apps_between(db: &Db, start: i64, end: i64) -> Vec<AppShare> {
    db.with(|c| {
        let mut st = c.prepare(
            "SELECT COALESCE(a.custom_name, a.name),
                    SUM(MAX(0, MIN(s.ended_at, ?2) - MAX(s.started_at, ?1))) AS secs
             FROM activity_sessions s JOIN activity_apps a ON a.id = s.app_id
             WHERE s.ended_at > ?1 AND s.started_at < ?2
             GROUP BY a.id HAVING secs > 0 ORDER BY secs DESC LIMIT 3",
        )?;
        let rows = st.query_map(params![start, end], |r| {
            Ok(AppShare {
                name: r.get(0)?,
                secs: r.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    })
    .unwrap_or_default()
}

fn load_block(db: &Db, id: i64) -> Option<Block> {
    let row = db
        .with(|c| {
            c.query_row(
                "SELECT id, kind, label, started_at, COALESCE(ended_at, started_at), planned_secs, completed
                 FROM focus_blocks WHERE id = ?1",
                params![id],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?,
                        r.get::<_, bool>(6)?,
                    ))
                },
            )
        })
        .ok()?;
    let apps = if row.1 == "focus" {
        apps_between(db, row.3, row.4)
    } else {
        Vec::new()
    };
    Some(Block {
        id: row.0,
        kind: row.1,
        label: row.2,
        started_at: row.3,
        ended_at: row.4,
        planned_secs: row.5,
        completed: row.6,
        apps,
    })
}

/// Days (as day numbers) that have a finished focus block, newest first.
pub fn streak(days: &[i64], today: i64) -> i64 {
    let mut n = 0;
    let mut want = today;
    for d in days {
        if *d == want {
            n += 1;
            want -= 1;
        } else if *d < want {
            break;
        }
    }
    // A streak may end yesterday: today has no block yet, but it is not lost.
    if n == 0 && days.first() == Some(&(today - 1)) {
        return streak(days, today - 1);
    }
    n
}

fn finish(app: &AppHandle, completed: bool) -> Option<Block> {
    let st = app.state::<AppState>();
    let focus = app.state::<FocusState>();
    let active = focus.active.lock().unwrap().take()?;
    let ended = if completed {
        active.started_at + active.planned_secs
    } else {
        now()
    };
    // Put what you were using in the record before reading it back.
    crate::activity::flush_now(app);
    let _ = st.db.with(|c| {
        c.execute(
            "UPDATE focus_blocks SET ended_at = ?2, completed = ?3 WHERE id = ?1",
            params![active.id, ended, completed],
        )
    });
    let block = load_block(&st.db, active.id);
    let _ = app.emit("focus://changed", ());
    block
}

#[tauri::command]
pub fn focus_start(
    app: AppHandle,
    state: State<'_, AppState>,
    focus: State<'_, FocusState>,
    kind: String,
    minutes: u32,
    label: String,
) -> CmdResult<ActiveBlock> {
    if kind != "focus" && kind != "break" {
        return Err("Pick focus or break.".into());
    }
    if !(1..=240).contains(&minutes) {
        return Err("Pick between 1 and 240 minutes.".into());
    }
    let label: String = label.trim().chars().take(60).collect();
    let mut slot = focus.active.lock().unwrap();
    if slot.is_some() {
        return Err("A timer is already running. Stop it first.".into());
    }
    let started = now();
    let planned = i64::from(minutes) * 60;
    let id = state
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO focus_blocks(started_at, planned_secs, kind, label) VALUES (?1, ?2, ?3, ?4)",
                params![started, planned, kind, label],
            )?;
            Ok(c.last_insert_rowid())
        })
        .map_err(|e| format!("The timer couldn't be saved: {e}"))?;
    *slot = Some(Active {
        id,
        kind: kind.clone(),
        label: label.clone(),
        started_at: started,
        planned_secs: planned,
    });
    drop(slot);
    let _ = app.emit("focus://changed", ());
    Ok(ActiveBlock {
        id,
        kind,
        label,
        started_at: started,
        planned_secs: planned,
        remaining_secs: planned,
    })
}

#[tauri::command]
pub fn focus_stop(app: AppHandle) -> Option<Block> {
    finish(&app, false)
}

#[tauri::command]
pub fn focus_status(state: State<'_, AppState>, focus: State<'_, FocusState>) -> FocusStatus {
    let t = now();
    let active = focus.active.lock().unwrap().as_ref().map(|a| ActiveBlock {
        id: a.id,
        kind: a.kind.clone(),
        label: a.label.clone(),
        started_at: a.started_at,
        planned_secs: a.planned_secs,
        remaining_secs: (a.started_at + a.planned_secs - t).max(0),
    });
    // Local midnight, so "today" matches the person's day.
    let offset = i64::from(chrono_offset_secs());
    let day = |ts: i64| (ts + offset).div_euclid(86_400);
    let today = day(t);
    let (today_blocks, today_secs, days) = state
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT started_at, COALESCE(ended_at, started_at) FROM focus_blocks
                 WHERE kind = 'focus' AND completed = 1 ORDER BY started_at DESC LIMIT 400",
            )?;
            let rows: Vec<(i64, i64)> = st
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?;
            let mut blocks = 0;
            let mut secs = 0;
            let mut days: Vec<i64> = Vec::new();
            for (s, e) in rows {
                let d = day(s);
                if d == today {
                    blocks += 1;
                    secs += (e - s).max(0);
                }
                if days.last() != Some(&d) {
                    days.push(d);
                }
            }
            Ok((blocks, secs, days))
        })
        .unwrap_or((0, 0, Vec::new()));
    let ids: Vec<i64> = state
        .db
        .with(|c| {
            let mut st = c.prepare("SELECT id FROM focus_blocks WHERE ended_at IS NOT NULL ORDER BY started_at DESC LIMIT 12")?;
            let v: Vec<i64> = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
            Ok(v)
        })
        .unwrap_or_default();
    FocusStatus {
        active,
        today_blocks,
        today_secs,
        streak_days: streak(&days, today),
        recent: ids
            .into_iter()
            .filter_map(|id| load_block(&state.db, id))
            .collect(),
    }
}

/// Seconds the local time zone is ahead of UTC.
fn chrono_offset_secs() -> i32 {
    #[cfg(windows)]
    {
        use windows::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};
        let mut tz = TIME_ZONE_INFORMATION::default();
        // SAFETY: tz is a valid, writable TIME_ZONE_INFORMATION.
        let id = unsafe { GetTimeZoneInformation(&mut tz) };
        let extra = if id == 2 {
            tz.DaylightBias
        } else {
            tz.StandardBias
        };
        -(tz.Bias + extra) * 60
    }
    #[cfg(not(windows))]
    {
        chrono::Local::now().offset().local_minus_utc()
    }
}

/// Watches the running timer and ends it when its time is up.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tick.tick().await;
            let due = {
                let f = app.state::<FocusState>();
                let g = f.active.lock().unwrap();
                g.as_ref()
                    .is_some_and(|a| now() >= a.started_at + a.planned_secs)
            };
            if !due {
                continue;
            }
            let kind = app
                .state::<FocusState>()
                .active
                .lock()
                .unwrap()
                .as_ref()
                .map(|a| a.kind.clone())
                .unwrap_or_default();
            let block = finish(&app, true);
            let lang = crate::i18n::lang_of(&app.state::<AppState>().settings.get());
            let (title, body) = crate::i18n::focus_done(lang, kind == "focus", block.as_ref());
            let _ = app.notification().builder().title(title).body(body).show();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::streak;

    #[test]
    fn counts_days_in_a_row_ending_today() {
        assert_eq!(streak(&[10, 9, 8, 6], 10), 3);
        assert_eq!(streak(&[10], 10), 1);
    }

    #[test]
    fn a_streak_that_ended_yesterday_is_still_alive() {
        assert_eq!(streak(&[9, 8, 7], 10), 3);
    }

    #[test]
    fn a_gap_ends_the_streak() {
        assert_eq!(streak(&[10, 8, 7], 10), 1);
        assert_eq!(streak(&[8, 7], 10), 0);
        assert_eq!(streak(&[], 10), 0);
    }
}
