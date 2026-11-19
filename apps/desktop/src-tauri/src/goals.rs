//! Playtime goals and the weekly report: reads the hours from the database, shows an alert when a
//! limit is near or passed or a target is reached, and writes the report of the week that ended.

use crate::state::AppState;
use crate::toast::{self, Toast};
use chrono::{Datelike, Duration as ChronoDuration, Local, NaiveDate, Timelike};
use nomnom_activity::encode_icon_data_url;
use nomnom_activity::goals::{
    self as rules, Alert, Goal, GoalResult, Kind, Period, Progress, Report, TopApp,
};
use nomnom_core::db::Db;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

const MAX_MINUTES: u32 = 24 * 60 * 7;
const MAX_GOALS: usize = 40;

#[derive(Debug, Clone, Serialize)]
pub struct GoalView {
    pub goal: Goal,
    pub name: String,
    pub icon_url: Option<String>,
    pub progress: Progress,
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn row_to_goal(r: &rusqlite::Row<'_>) -> rusqlite::Result<Goal> {
    let kind: String = r.get(2)?;
    let period: String = r.get(3)?;
    Ok(Goal {
        id: r.get(0)?,
        scope: r.get(1)?,
        kind: if kind == "target" {
            Kind::Target
        } else {
            Kind::Limit
        },
        period: if period == "week" {
            Period::Week
        } else {
            Period::Day
        },
        minutes: r.get::<_, i64>(4)?.clamp(1, i64::from(MAX_MINUTES)) as u32,
        enabled: r.get::<_, i64>(5)? != 0,
    })
}

pub fn load_goals(db: &Db) -> Vec<Goal> {
    db.with(|c| {
        let mut st = c.prepare(
            "SELECT id, scope, kind, period, minutes, enabled FROM activity_goals ORDER BY created_at, id",
        )?;
        let rows = st.query_map([], row_to_goal)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
    })
    .unwrap_or_default()
}

/// Seconds used by `scope` between two days (inclusive).
pub fn used_secs(db: &Db, scope: &str, from: &str, to: &str) -> u64 {
    let total: i64 = db
        .with(|c| {
            if let Some(app) = scope.strip_prefix("app:") {
                c.query_row(
                    "SELECT COALESCE(SUM(active_secs), 0) FROM activity_hourly
                     WHERE app_id = ?1 AND day BETWEEN ?2 AND ?3",
                    params![app, from, to],
                    |r| r.get(0),
                )
            } else if let Some(cat) = scope.strip_prefix("category:") {
                c.query_row(
                    "SELECT COALESCE(SUM(h.active_secs), 0) FROM activity_hourly h
                     JOIN activity_apps a ON a.id = h.app_id
                     WHERE a.category = ?1 AND h.day BETWEEN ?2 AND ?3",
                    params![cat, from, to],
                    |r| r.get(0),
                )
            } else {
                c.query_row(
                    "SELECT COALESCE(SUM(h.active_secs), 0) FROM activity_hourly h
                     JOIN activity_apps a ON a.id = h.app_id
                     WHERE a.hidden = 0 AND h.day BETWEEN ?1 AND ?2",
                    params![from, to],
                    |r| r.get(0),
                )
            }
        })
        .unwrap_or(0);
    total.max(0) as u64
}

fn name_and_icon(db: &Db, scope: &str) -> (String, Option<String>) {
    if let Some(app) = scope.strip_prefix("app:") {
        let row = db
            .with(|c| {
                c.query_row(
                    "SELECT COALESCE(custom_name, name), icon_png FROM activity_apps WHERE id = ?1",
                    params![app],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<Vec<u8>>>(1)?)),
                )
                .optional()
            })
            .ok()
            .flatten();
        return match row {
            Some((name, icon)) => (name, icon.as_deref().map(encode_icon_data_url)),
            None => (app.to_string(), None),
        };
    }
    if let Some(cat) = scope.strip_prefix("category:") {
        let mut name = cat.to_string();
        if let Some(first) = name.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        return (name, None);
    }
    ("All apps".to_string(), None)
}

fn view(db: &Db, goal: Goal, on: NaiveDate) -> GoalView {
    let (from, to) = rules::period_range(goal.period, on);
    let used = used_secs(db, &goal.scope, &from, &to);
    let (name, icon_url) = name_and_icon(db, &goal.scope);
    GoalView {
        progress: rules::progress(&goal, used),
        goal,
        name,
        icon_url,
    }
}

#[tauri::command]
pub fn goals_list(state: State<'_, AppState>) -> Vec<GoalView> {
    let on = today();
    load_goals(&state.db)
        .into_iter()
        .map(|g| view(&state.db, g, on))
        .collect()
}

fn valid_scope(scope: &str) -> bool {
    scope == "all"
        || scope
            .strip_prefix("app:")
            .or_else(|| scope.strip_prefix("category:"))
            .is_some_and(|rest| !rest.is_empty() && rest.len() <= 300)
}

/// Creates a goal (empty id) or changes one.
#[tauri::command]
pub fn goals_save(app: AppHandle, state: State<'_, AppState>, goal: Goal) -> CmdResult<Goal> {
    if !valid_scope(&goal.scope) {
        return Err("Pick an app, a kind of app, or all apps.".into());
    }
    if goal.minutes == 0 || goal.minutes > MAX_MINUTES {
        return Err("Pick a time between 1 minute and 7 days.".into());
    }
    let mut goal = goal;
    let creating = goal.id.trim().is_empty();
    if creating {
        if load_goals(&state.db).len() >= MAX_GOALS {
            return Err(format!("You can have up to {MAX_GOALS} goals."));
        }
        goal.id = uuid::Uuid::new_v4().simple().to_string();
    }
    let kind = if goal.kind == Kind::Target {
        "target"
    } else {
        "limit"
    };
    let period = if goal.period == Period::Week {
        "week"
    } else {
        "day"
    };
    state
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO activity_goals(id, scope, kind, period, minutes, enabled, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%s','now'))
                 ON CONFLICT(id) DO UPDATE SET scope = ?2, kind = ?3, period = ?4, minutes = ?5, enabled = ?6",
                params![goal.id, goal.scope, kind, period, goal.minutes, goal.enabled as i64],
            )
        })
        .map_err(|e| e.to_string())?;
    let _ = app.emit("activity://goals", ());
    Ok(goal)
}

#[tauri::command]
pub fn goals_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state
        .db
        .with(|c| c.execute("DELETE FROM activity_goals WHERE id = ?1", params![id]))
        .map_err(|e| e.to_string())?;
    let _ = app.emit("activity://goals", ());
    Ok(())
}

// ---- the weekly report ----------------------------------------------------------------------------

fn day_secs(db: &Db, day: &str) -> u64 {
    used_secs(db, "all", day, day)
}

fn top_apps(db: &Db, from: &str, to: &str, limit: usize) -> Vec<TopApp> {
    db.with(|c| {
        let mut st = c.prepare(
            "SELECT COALESCE(a.custom_name, a.name), SUM(h.active_secs) AS s FROM activity_hourly h
             JOIN activity_apps a ON a.id = h.app_id
             WHERE a.hidden = 0 AND h.day BETWEEN ?1 AND ?2
             GROUP BY h.app_id ORDER BY s DESC LIMIT ?3",
        )?;
        let rows = st.query_map(params![from, to, limit as i64], |r| {
            Ok(TopApp {
                name: r.get(0)?,
                secs: r.get::<_, i64>(1)?.max(0) as u64,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
    })
    .unwrap_or_default()
}

/// The report for the week that began on `week_start` (a Monday).
pub fn build_report(db: &Db, week_start: NaiveDate) -> Report {
    let fmt = |d: NaiveDate| d.format("%Y-%m-%d").to_string();
    let end = week_start + ChronoDuration::days(6);
    let mut days = [0u64; 7];
    for (i, slot) in days.iter_mut().enumerate() {
        *slot = day_secs(db, &fmt(week_start + ChronoDuration::days(i as i64)));
    }
    let previous_start = week_start - ChronoDuration::days(7);
    let previous_total = used_secs(
        db,
        "all",
        &fmt(previous_start),
        &fmt(previous_start + ChronoDuration::days(6)),
    );
    let mut goals = Vec::new();
    for g in load_goals(db).into_iter().filter(|g| g.enabled) {
        let (name, _) = name_and_icon(db, &g.scope);
        let (kept, out_of) = match g.period {
            Period::Day => {
                let kept = (0..7)
                    .filter(|i| {
                        let d = fmt(week_start + ChronoDuration::days(*i));
                        rules::kept(g.kind, g.minutes, used_secs(db, &g.scope, &d, &d))
                    })
                    .count() as u32;
                (kept, 7)
            }
            Period::Week => {
                let used = used_secs(db, &g.scope, &fmt(week_start), &fmt(end));
                (u32::from(rules::kept(g.kind, g.minutes, used)), 1)
            }
        };
        goals.push(GoalResult {
            scope: g.scope,
            name,
            kind: g.kind,
            period: g.period,
            minutes: g.minutes,
            kept,
            out_of,
        });
    }
    Report {
        week_start: fmt(week_start),
        total_secs: days.iter().sum(),
        previous_total_secs: previous_total,
        days,
        top_apps: top_apps(db, &fmt(week_start), &fmt(end), 5),
        goals,
    }
}

fn stored_report(db: &Db, week_start: &str) -> Option<Report> {
    db.with(|c| {
        c.query_row(
            "SELECT json FROM activity_reports WHERE week_start = ?1",
            params![week_start],
            |r| r.get::<_, String>(0),
        )
        .optional()
    })
    .ok()
    .flatten()
    .and_then(|j| serde_json::from_str(&j).ok())
}

fn store_report(db: &Db, report: &Report) {
    if let Ok(json) = serde_json::to_string(report) {
        let _ = db.with(|c| {
            c.execute(
                "INSERT OR REPLACE INTO activity_reports(week_start, json, created_at)
                 VALUES (?1, ?2, strftime('%s','now'))",
                params![report.week_start, json],
            )
        });
    }
}

/// The report of a week (a Monday, `YYYY-MM-DD`), or of the last full week. Made when asked for.
#[tauri::command]
pub fn goals_report(state: State<'_, AppState>, week_start: Option<String>) -> Option<Report> {
    let monday = match week_start {
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()?,
        None => rules::last_full_week(today()),
    };
    let monday = rules::monday_of(monday);
    let key = monday.format("%Y-%m-%d").to_string();
    if monday < rules::monday_of(today()) {
        if let Some(r) = stored_report(&state.db, &key) {
            return Some(r);
        }
    }
    let report = build_report(&state.db, monday);
    if monday < rules::monday_of(today()) && report.total_secs > 0 {
        store_report(&state.db, &report);
    }
    Some(report)
}

/// This week so far, for the card on the page.
#[tauri::command]
pub fn goals_this_week(state: State<'_, AppState>) -> Report {
    build_report(&state.db, rules::monday_of(today()))
}

// ---- the watcher ----------------------------------------------------------------------------------

fn alert_done(db: &Db, key: &str) -> bool {
    db.with(|c| {
        c.query_row(
            "SELECT 1 FROM activity_goal_alerts WHERE key = ?1",
            params![key],
            |_| Ok(()),
        )
        .optional()
    })
    .ok()
    .flatten()
    .is_some()
}

fn alert_remember(db: &Db, key: &str) {
    let _ = db.with(|c| {
        c.execute(
            "INSERT OR IGNORE INTO activity_goal_alerts(key, at) VALUES (?1, strftime('%s','now'))",
            params![key],
        )?;
        // Old keys are of no use.
        c.execute(
            "DELETE FROM activity_goal_alerts WHERE at < strftime('%s','now') - 5184000",
            [],
        )
    });
}

fn evaluate(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings.get();
    let lang = crate::i18n::lang_of(&settings);
    let now = Local::now();
    let day = now.date_naive();
    let reminder = settings.activity.goal_reminder_hour;

    let mut changed = false;
    for goal in load_goals(&state.db) {
        if !goal.enabled {
            continue;
        }
        let (from, to) = rules::period_range(goal.period, day);
        let used = used_secs(&state.db, &goal.scope, &from, &to);
        for alert in rules::due_alerts(&goal, used, now.hour(), now.weekday(), reminder) {
            let key = format!(
                "{}:{}:{}",
                goal.id,
                rules::period_key(goal.period, day),
                alert.code()
            );
            if alert_done(&state.db, &key) {
                continue;
            }
            alert_remember(&state.db, &key);
            changed = true;
            let (name, _) = name_and_icon(&state.db, &goal.scope);
            let (title, body) = crate::i18n::goal_text(
                lang,
                alert,
                &name,
                (used / 60) as u32,
                goal.minutes,
                goal.period,
            );
            let kind = match alert {
                Alert::LimitOver => "warning",
                Alert::TargetMet => "success",
                _ => "info",
            };
            toast::show(app, Toast::new(kind, title, body).opens("activity"));
        }
    }

    // The report of the week that ended, once, from Monday morning on.
    if state.settings.get().activity.weekly_report && now.hour() >= 8 {
        let monday = rules::last_full_week(day);
        let key = monday.format("%Y-%m-%d").to_string();
        if stored_report(&state.db, &key).is_none() {
            let report = build_report(&state.db, monday);
            // A week with nothing in it (the app was not used) gets no report.
            if report.total_secs > 0 {
                store_report(&state.db, &report);
                let (title, body) = crate::i18n::report_text(lang, report.total_secs / 60);
                toast::show(app, Toast::new("info", title, body).opens("activity"));
                changed = true;
            }
        }
    }
    if changed {
        let _ = app.emit("activity://goals", ());
    }
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(40)).await;
        loop {
            let a = app.clone();
            let _ = tokio::task::spawn_blocking(move || evaluate(&a)).await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}
