//! Playtime inside the app: runs the 2-second foreground probe, persists active non-idle usage to
//! SQLite, caches app icons, and answers overview and per-app statistical queries.

use crate::state::AppState;
use chrono::{Datelike, Duration as ChronoDuration, Local, NaiveDate};
use nomnom_activity::{
    encode_icon_data_url, extract_app_meta, ActiveTracker, AppCategory, ForegroundProbe, LiveFocus,
    SessionFlush,
};
use nomnom_core::db::Db;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

#[derive(Default)]
pub struct ActivityState {
    tracker: Mutex<ActiveTracker>,
    known_icons: Mutex<HashSet<String>>,
}

impl ActivityState {
    /// What is in front right now, if anything.
    pub fn live(&self) -> Option<LiveFocus> {
        self.tracker.lock().unwrap().live()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackedAppRow {
    pub id: String,
    pub name: String,
    pub custom_name: Option<String>,
    pub category: String,
    pub exe_path: Option<String>,
    pub icon_url: Option<String>,
    pub hidden: bool,
    pub range_secs: u64,
    pub today_secs: u64,
    pub week_secs: u64,
    pub month_secs: u64,
    pub year_secs: u64,
    pub total_secs: u64,
    pub first_seen: i64,
    pub last_seen: i64,
    pub share: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelinePoint {
    pub key: String,
    pub label: String,
    pub secs: u64,
    pub by_category: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapDay {
    pub day: String,
    pub secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryTotal {
    pub category: String,
    pub secs: u64,
    pub apps: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionEntry {
    pub id: i64,
    pub app_id: String,
    pub app_name: String,
    pub category: String,
    pub icon_url: Option<String>,
    pub started_at: i64,
    pub ended_at: i64,
    pub active_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityOverview {
    pub enabled: bool,
    pub idle_timeout_secs: u32,
    pub track_nomnom: bool,
    pub range: String,
    pub live: Option<LiveFocus>,
    pub range_secs: u64,
    pub today_secs: u64,
    pub week_secs: u64,
    pub month_secs: u64,
    pub year_secs: u64,
    pub all_time_secs: u64,
    pub daily_avg_secs: u64,
    pub sessions_count: u64,
    pub peak_hour: Option<u8>,
    pub apps: Vec<TrackedAppRow>,
    pub timeline: Vec<TimelinePoint>,
    pub clock_hours: Vec<u64>,
    pub heatmap: Vec<HeatmapDay>,
    pub categories: Vec<CategoryTotal>,
    pub recent_sessions: Vec<SessionEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppDetail {
    pub app: TrackedAppRow,
    pub longest_session_secs: u64,
    pub sessions_count: u64,
    pub active_days: u32,
    pub peak_hour: Option<u8>,
    pub daily_30d: Vec<TimelinePoint>,
    pub monthly_12m: Vec<TimelinePoint>,
    pub clock_hours: Vec<u64>,
    pub recent_sessions: Vec<SessionEntry>,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn persist_flushes(db: &Db, state: &ActivityState, flushes: Vec<SessionFlush>) {
    if flushes.is_empty() {
        return;
    }

    for flush in flushes {
        let need_icon = {
            let known = state.known_icons.lock().unwrap();
            !known.contains(&flush.app.id)
        };
        let has_icon_in_db = if need_icon {
            db.with(|c| {
                c.query_row(
                    "SELECT icon_png IS NOT NULL FROM activity_apps WHERE id = ?1",
                    params![&flush.app.id],
                    |r| r.get::<_, bool>(0),
                )
                .optional()
            })
            .ok()
            .flatten()
            .unwrap_or(false)
        } else {
            true
        };

        let extracted = if !has_icon_in_db {
            Some(extract_app_meta(
                flush.app.exe_path.as_deref(),
                flush.app.wm_class.as_deref(),
            ))
        } else {
            None
        };

        if has_icon_in_db || extracted.as_ref().is_some_and(|m| m.icon_png.is_some()) {
            state
                .known_icons
                .lock()
                .unwrap()
                .insert(flush.app.id.clone());
        }

        let resolved_name = extracted
            .as_ref()
            .and_then(|m| m.display_name.clone())
            .unwrap_or_else(|| flush.app.name.clone());
        let icon_png = extracted.and_then(|m| m.icon_png);

        let _ = db.with(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO activity_apps(
                    id, name, exe_path, wm_class, category, icon_png, first_seen, last_seen, total_secs
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    exe_path = COALESCE(excluded.exe_path, activity_apps.exe_path),
                    wm_class = COALESCE(excluded.wm_class, activity_apps.wm_class),
                    icon_png = COALESCE(activity_apps.icon_png, excluded.icon_png),
                    last_seen = MAX(activity_apps.last_seen, excluded.last_seen),
                    total_secs = activity_apps.total_secs + excluded.total_secs",
                params![
                    &flush.app.id,
                    &resolved_name,
                    &flush.app.exe_path,
                    &flush.app.wm_class,
                    flush.app.category.as_str(),
                    &icon_png,
                    flush.seen_at,
                    flush.seen_at,
                    i64::from(flush.delta_secs),
                ],
            )?;

            for b in &flush.hourly {
                tx.execute(
                    "INSERT INTO activity_hourly(app_id, day, hour, active_secs, sessions)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(app_id, day, hour) DO UPDATE SET
                        active_secs = activity_hourly.active_secs + excluded.active_secs,
                        sessions = activity_hourly.sessions + excluded.sessions",
                    params![
                        &flush.app.id,
                        &b.day,
                        i64::from(b.hour),
                        i64::from(b.secs),
                        i64::from(b.sessions),
                    ],
                )?;
            }

            if let Some((started_at, ended_at, secs)) = flush.finished_session {
                tx.execute(
                    "INSERT INTO activity_sessions(app_id, started_at, ended_at, active_secs)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![&flush.app.id, started_at, ended_at, i64::from(secs)],
                )?;
            }

            tx.commit()
        });
    }
}

pub fn flush_on_exit(app: &AppHandle) {
    let state = app.state::<AppState>();
    let flush = state
        .activity
        .tracker
        .lock()
        .unwrap()
        .flush_all()
        .into_iter()
        .collect::<Vec<_>>();
    persist_flushes(&state.db, &state.activity, flush);
}

/// Writes the app you are using right now to the record, so a summary made this moment includes it.
pub fn flush_now(app: &AppHandle) {
    flush_on_exit(app);
}

pub fn spawn(app: AppHandle) {
    let self_pid = std::process::id();
    tauri::async_runtime::spawn(async move {
        let probe = std::sync::Arc::new(Mutex::new(ForegroundProbe::new()));
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            let cfg = app.state::<AppState>().settings.get().activity;
            if !cfg.enabled {
                let pending = app
                    .state::<AppState>()
                    .activity
                    .tracker
                    .lock()
                    .unwrap()
                    .flush_all()
                    .into_iter()
                    .collect::<Vec<_>>();
                if !pending.is_empty() {
                    let st = app.state::<AppState>();
                    persist_flushes(&st.db, &st.activity, pending);
                }
                continue;
            }

            let probe_clone = probe.clone();
            let Ok(sample) =
                tokio::task::spawn_blocking(move || probe_clone.lock().unwrap().sample()).await
            else {
                continue;
            };

            let now = unix_now();
            let flushes = {
                let st = app.state::<AppState>();
                let mut tr = st.activity.tracker.lock().unwrap();
                tr.tick(
                    now,
                    sample,
                    cfg.idle_timeout_secs,
                    self_pid,
                    cfg.track_nomnom,
                )
            };

            if !flushes.is_empty() {
                let db = app.state::<AppState>().db.clone();
                let app_clone = app.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    let st = app_clone.state::<AppState>();
                    persist_flushes(&db, &st.activity, flushes);
                })
                .await;
                if app.get_webview_window("main").is_some() {
                    let _ = app.emit("activity://updated", ());
                }
            }
        }
    });
}

fn date_bounds(today: NaiveDate, range: &str) -> (String, u32) {
    match range {
        "today" => (today.format("%Y-%m-%d").to_string(), 1),
        "week" => (
            (today - ChronoDuration::days(6))
                .format("%Y-%m-%d")
                .to_string(),
            7,
        ),
        "month" => (
            (today - ChronoDuration::days(29))
                .format("%Y-%m-%d")
                .to_string(),
            30,
        ),
        "year" => (
            (today - ChronoDuration::days(364))
                .format("%Y-%m-%d")
                .to_string(),
            365,
        ),
        _ => ("0000-01-01".to_string(), 365),
    }
}

#[tauri::command]
pub fn act_live(state: State<'_, AppState>) -> Option<LiveFocus> {
    state.activity.tracker.lock().unwrap().live()
}

#[tauri::command]
pub fn act_overview(
    state: State<'_, AppState>,
    range: String,
    include_hidden: bool,
) -> CmdResult<ActivityOverview> {
    let cfg = state.settings.get().activity;
    let live = state.activity.tracker.lock().unwrap().live();

    let now_local = Local::now();
    let today = now_local.date_naive();
    let today_str = today.format("%Y-%m-%d").to_string();
    let week_start = (today - ChronoDuration::days(6))
        .format("%Y-%m-%d")
        .to_string();
    let month_start = (today - ChronoDuration::days(29))
        .format("%Y-%m-%d")
        .to_string();
    let year_start = (today - ChronoDuration::days(364))
        .format("%Y-%m-%d")
        .to_string();
    let heatmap_start = (today - ChronoDuration::days(181))
        .format("%Y-%m-%d")
        .to_string();

    let norm_range = match range.as_str() {
        "today" | "week" | "month" | "year" | "all" => range.clone(),
        _ => "week".to_string(),
    };
    let (range_start, span_days) = date_bounds(today, &norm_range);

    state
        .db
        .with(|conn| {
            // 1. Load all apps
            let mut app_map: HashMap<String, TrackedAppRow> = HashMap::new();
            {
                let mut stmt = conn.prepare(
                    "SELECT id, name, custom_name, category, exe_path, icon_png, hidden,
                            first_seen, last_seen, total_secs
                     FROM activity_apps",
                )?;
                let rows = stmt.query_map([], |r| {
                    let id: String = r.get(0)?;
                    let base_name: String = r.get(1)?;
                    let custom_name: Option<String> = r.get(2)?;
                    let category: String = r.get(3)?;
                    let exe_path: Option<String> = r.get(4)?;
                    let icon_png: Option<Vec<u8>> = r.get(5)?;
                    let hidden: bool = r.get::<_, i64>(6)? != 0;
                    let first_seen: i64 = r.get(7)?;
                    let last_seen: i64 = r.get(8)?;
                    let total_secs = r.get::<_, i64>(9)?.max(0) as u64;
                    let display = custom_name
                        .clone()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or(base_name);
                    Ok(TrackedAppRow {
                        id,
                        name: display,
                        custom_name,
                        category,
                        exe_path,
                        icon_url: icon_png.as_deref().map(encode_icon_data_url),
                        hidden,
                        range_secs: 0,
                        today_secs: 0,
                        week_secs: 0,
                        month_secs: 0,
                        year_secs: 0,
                        total_secs,
                        first_seen,
                        last_seen,
                        share: 0.0,
                    })
                })?;
                for row in rows {
                    let item = row?;
                    app_map.insert(item.id.clone(), item);
                }
            }

            // 2. Roll up hourly rows for the last year (plus all-time if `norm_range == "all"`)
            let query_floor = if norm_range == "all" {
                "0000-01-01"
            } else {
                &year_start
            };

            let mut today_secs = 0u64;
            let mut week_secs = 0u64;
            let mut month_secs = 0u64;
            let mut year_secs = 0u64;
            let mut range_secs = 0u64;
            let mut sessions_count = 0u64;
            let mut clock_hours = vec![0u64; 24];
            let mut heatmap_map: HashMap<String, u64> = HashMap::new();
            let mut timeline_map: HashMap<String, (u64, HashMap<String, u64>)> = HashMap::new();
            let mut active_days_in_range: HashSet<String> = HashSet::new();

            {
                let mut stmt = conn.prepare(
                    "SELECT app_id, day, hour, active_secs, sessions
                     FROM activity_hourly
                     WHERE day >= ?1",
                )?;
                let rows = stmt.query_map(params![query_floor], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?.clamp(0, 23) as usize,
                        r.get::<_, i64>(3)?.max(0) as u64,
                        r.get::<_, i64>(4)?.max(0) as u64,
                    ))
                })?;

                for row in rows {
                    let (app_id, day, hour, secs, sess) = row?;
                    let Some(app_entry) = app_map.get_mut(&app_id) else {
                        continue;
                    };
                    if app_entry.hidden && !include_hidden {
                        continue;
                    }
                    let cat = app_entry.category.clone();

                    if day.as_str() >= year_start.as_str() {
                        app_entry.year_secs += secs;
                        year_secs += secs;
                    }
                    if day.as_str() >= month_start.as_str() {
                        app_entry.month_secs += secs;
                        month_secs += secs;
                    }
                    if day.as_str() >= week_start.as_str() {
                        app_entry.week_secs += secs;
                        week_secs += secs;
                    }
                    if day == today_str {
                        app_entry.today_secs += secs;
                        today_secs += secs;
                    }
                    if day.as_str() >= heatmap_start.as_str() {
                        *heatmap_map.entry(day.clone()).or_insert(0) += secs;
                    }

                    if day.as_str() >= range_start.as_str() {
                        app_entry.range_secs += secs;
                        range_secs += secs;
                        sessions_count += sess;
                        clock_hours[hour] += secs;
                        if secs > 0 {
                            active_days_in_range.insert(day.clone());
                        }

                        let bucket_key = match norm_range.as_str() {
                            "today" => format!("{hour:02}"),
                            "week" | "month" => day.clone(),
                            _ => day.get(..7).unwrap_or(&day).to_string(),
                        };
                        let entry = timeline_map
                            .entry(bucket_key)
                            .or_insert_with(|| (0, HashMap::new()));
                        entry.0 += secs;
                        *entry.1.entry(cat).or_insert(0) += secs;
                    }
                }
            }

            let all_time_secs: u64 = app_map
                .values()
                .filter(|a| include_hidden || !a.hidden)
                .map(|a| a.total_secs)
                .sum();

            // Build ordered timeline points
            let timeline = build_timeline(today, &norm_range, &timeline_map);

            // Build 182-day (26-week) heatmap grid
            let mut heatmap = Vec::with_capacity(182);
            for offset in (0..182).rev() {
                let d = (today - ChronoDuration::days(offset))
                    .format("%Y-%m-%d")
                    .to_string();
                let secs = heatmap_map.get(&d).copied().unwrap_or(0);
                heatmap.push(HeatmapDay { day: d, secs });
            }

            // Peak hour
            let peak_hour = clock_hours
                .iter()
                .enumerate()
                .filter(|(_, &s)| s > 0)
                .max_by_key(|(_, &s)| s)
                .map(|(h, _)| h as u8);

            // Categories & sorted app rows
            let mut cat_map: HashMap<String, (u64, usize)> = HashMap::new();
            let mut apps: Vec<TrackedAppRow> = app_map
                .into_values()
                .filter(|a| {
                    (include_hidden || !a.hidden)
                        && (a.range_secs > 0 || norm_range == "all" || include_hidden)
                })
                .collect();

            for a in &mut apps {
                a.share = if range_secs > 0 {
                    (a.range_secs as f64) / (range_secs as f64)
                } else {
                    0.0
                };
                if a.range_secs > 0 {
                    let c = cat_map.entry(a.category.clone()).or_insert((0, 0));
                    c.0 += a.range_secs;
                    c.1 += 1;
                }
            }
            apps.sort_by(|a, b| {
                b.range_secs
                    .cmp(&a.range_secs)
                    .then_with(|| b.total_secs.cmp(&a.total_secs))
                    .then_with(|| a.name.cmp(&b.name))
            });

            let mut categories: Vec<CategoryTotal> = cat_map
                .into_iter()
                .map(|(category, (secs, apps_n))| CategoryTotal {
                    category,
                    secs,
                    apps: apps_n,
                })
                .collect();
            categories.sort_by_key(|a| std::cmp::Reverse(a.secs));

            // Recent sessions ("when you used it")
            let mut recent_sessions = Vec::new();
            {
                let mut stmt = conn.prepare(
                    "SELECT s.id, s.app_id, COALESCE(a.custom_name, a.name), a.category, a.icon_png,
                            s.started_at, s.ended_at, s.active_secs
                     FROM activity_sessions s
                     JOIN activity_apps a ON a.id = s.app_id
                     WHERE (?1 = 1 OR a.hidden = 0)
                     ORDER BY s.started_at DESC
                     LIMIT 30",
                )?;
                let rows = stmt.query_map(params![i64::from(include_hidden)], |r| {
                    let icon_png: Option<Vec<u8>> = r.get(4)?;
                    Ok(SessionEntry {
                        id: r.get(0)?,
                        app_id: r.get(1)?,
                        app_name: r.get(2)?,
                        category: r.get(3)?,
                        icon_url: icon_png.as_deref().map(encode_icon_data_url),
                        started_at: r.get(5)?,
                        ended_at: r.get(6)?,
                        active_secs: r.get::<_, i64>(7)?.max(0) as u64,
                    })
                })?;
                for s in rows {
                    recent_sessions.push(s?);
                }
            }

            let divisor = if norm_range == "today" {
                1
            } else if norm_range == "all" {
                (active_days_in_range.len() as u64).max(1)
            } else {
                u64::from(span_days).min((active_days_in_range.len() as u64).max(1))
            };
            let daily_avg_secs = range_secs / divisor.max(1);

            Ok(ActivityOverview {
                enabled: cfg.enabled,
                idle_timeout_secs: cfg.idle_timeout_secs,
                track_nomnom: cfg.track_nomnom,
                range: norm_range,
                live,
                range_secs,
                today_secs,
                week_secs,
                month_secs,
                year_secs,
                all_time_secs,
                daily_avg_secs,
                sessions_count,
                peak_hour,
                apps,
                timeline,
                clock_hours,
                heatmap,
                categories,
                recent_sessions,
            })
        })
        .map_err(|e| e.to_string())
}

fn build_timeline(
    today: NaiveDate,
    range: &str,
    map: &HashMap<String, (u64, HashMap<String, u64>)>,
) -> Vec<TimelinePoint> {
    match range {
        "today" => (0..24u8)
            .map(|h| {
                let key = format!("{h:02}");
                let (secs, by_cat) = map.get(&key).cloned().unwrap_or_default();
                TimelinePoint {
                    label: format!("{h:02}h"),
                    key,
                    secs,
                    by_category: by_cat,
                }
            })
            .collect(),
        "week" => (0..7)
            .rev()
            .map(|offset| {
                let d = today - ChronoDuration::days(offset);
                let key = d.format("%Y-%m-%d").to_string();
                let (secs, by_cat) = map.get(&key).cloned().unwrap_or_default();
                TimelinePoint {
                    label: d.format("%a").to_string(),
                    key,
                    secs,
                    by_category: by_cat,
                }
            })
            .collect(),
        "month" => (0..30)
            .rev()
            .map(|offset| {
                let d = today - ChronoDuration::days(offset);
                let key = d.format("%Y-%m-%d").to_string();
                let (secs, by_cat) = map.get(&key).cloned().unwrap_or_default();
                TimelinePoint {
                    label: d.format("%d/%m").to_string(),
                    key,
                    secs,
                    by_category: by_cat,
                }
            })
            .collect(),
        _ => {
            let mut months = Vec::with_capacity(12);
            let mut y = today.year();
            let mut m = today.month() as i32;
            let mut ym_list = Vec::with_capacity(12);
            for _ in 0..12 {
                ym_list.push((y, m as u32));
                m -= 1;
                if m == 0 {
                    m = 12;
                    y -= 1;
                }
            }
            ym_list.reverse();
            for (yr, mo) in ym_list {
                let key = format!("{yr:04}-{mo:02}");
                let d = NaiveDate::from_ymd_opt(yr, mo, 1).unwrap_or(today);
                let (secs, by_cat) = map.get(&key).cloned().unwrap_or_default();
                months.push(TimelinePoint {
                    label: d.format("%b").to_string(),
                    key,
                    secs,
                    by_category: by_cat,
                });
            }
            months
        }
    }
}

#[tauri::command]
pub fn act_app_detail(state: State<'_, AppState>, app_id: String) -> CmdResult<AppDetail> {
    let today = Local::now().date_naive();
    let today_str = today.format("%Y-%m-%d").to_string();
    let week_start = (today - ChronoDuration::days(6))
        .format("%Y-%m-%d")
        .to_string();
    let month_start = (today - ChronoDuration::days(29))
        .format("%Y-%m-%d")
        .to_string();
    let year_start = (today - ChronoDuration::days(364))
        .format("%Y-%m-%d")
        .to_string();

    state
        .db
        .with(|conn| {
            let mut app = conn
                .query_row(
                    "SELECT id, name, custom_name, category, exe_path, icon_png, hidden,
                            first_seen, last_seen, total_secs
                     FROM activity_apps WHERE id = ?1",
                    params![&app_id],
                    |r| {
                        let base_name: String = r.get(1)?;
                        let custom_name: Option<String> = r.get(2)?;
                        let icon_png: Option<Vec<u8>> = r.get(5)?;
                        Ok(TrackedAppRow {
                            id: r.get(0)?,
                            name: custom_name
                                .clone()
                                .filter(|s| !s.trim().is_empty())
                                .unwrap_or(base_name),
                            custom_name,
                            category: r.get(3)?,
                            exe_path: r.get(4)?,
                            icon_url: icon_png.as_deref().map(encode_icon_data_url),
                            hidden: r.get::<_, i64>(6)? != 0,
                            range_secs: 0,
                            today_secs: 0,
                            week_secs: 0,
                            month_secs: 0,
                            year_secs: 0,
                            total_secs: r.get::<_, i64>(9)?.max(0) as u64,
                            first_seen: r.get(7)?,
                            last_seen: r.get(8)?,
                            share: 1.0,
                        })
                    },
                )
                .optional()?
                .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;

            let mut daily_map: HashMap<String, (u64, HashMap<String, u64>)> = HashMap::new();
            let mut monthly_map: HashMap<String, (u64, HashMap<String, u64>)> = HashMap::new();
            let mut clock_hours = vec![0u64; 24];
            let mut active_days_set: HashSet<String> = HashSet::new();
            let mut sessions_count = 0u64;

            {
                let mut stmt = conn.prepare(
                    "SELECT day, hour, active_secs, sessions
                     FROM activity_hourly WHERE app_id = ?1",
                )?;
                let rows = stmt.query_map(params![&app_id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?.clamp(0, 23) as usize,
                        r.get::<_, i64>(2)?.max(0) as u64,
                        r.get::<_, i64>(3)?.max(0) as u64,
                    ))
                })?;
                for row in rows {
                    let (day, hour, secs, sess) = row?;
                    if secs > 0 {
                        active_days_set.insert(day.clone());
                    }
                    sessions_count += sess;
                    clock_hours[hour] += secs;
                    if day == today_str {
                        app.today_secs += secs;
                    }
                    if day.as_str() >= week_start.as_str() {
                        app.week_secs += secs;
                    }
                    if day.as_str() >= month_start.as_str() {
                        app.month_secs += secs;
                        let e = daily_map
                            .entry(day.clone())
                            .or_insert_with(|| (0, HashMap::new()));
                        e.0 += secs;
                    }
                    if day.as_str() >= year_start.as_str() {
                        app.year_secs += secs;
                        let ym = day.get(..7).unwrap_or(&day).to_string();
                        let e = monthly_map.entry(ym).or_insert_with(|| (0, HashMap::new()));
                        e.0 += secs;
                    }
                }
            }
            app.range_secs = app.total_secs;

            let longest_session_secs = conn
                .query_row(
                    "SELECT COALESCE(MAX(active_secs), 0) FROM activity_sessions WHERE app_id = ?1",
                    params![&app_id],
                    |r| r.get::<_, i64>(0),
                )?
                .max(0) as u64;

            let mut recent_sessions = Vec::new();
            {
                let mut stmt = conn.prepare(
                    "SELECT id, started_at, ended_at, active_secs
                     FROM activity_sessions
                     WHERE app_id = ?1
                     ORDER BY started_at DESC
                     LIMIT 40",
                )?;
                let rows = stmt.query_map(params![&app_id], |r| {
                    Ok(SessionEntry {
                        id: r.get(0)?,
                        app_id: app.id.clone(),
                        app_name: app.name.clone(),
                        category: app.category.clone(),
                        icon_url: app.icon_url.clone(),
                        started_at: r.get(1)?,
                        ended_at: r.get(2)?,
                        active_secs: r.get::<_, i64>(3)?.max(0) as u64,
                    })
                })?;
                for s in rows {
                    recent_sessions.push(s?);
                }
            }

            let peak_hour = clock_hours
                .iter()
                .enumerate()
                .filter(|(_, &s)| s > 0)
                .max_by_key(|(_, &s)| s)
                .map(|(h, _)| h as u8);

            Ok(AppDetail {
                app,
                longest_session_secs,
                sessions_count,
                active_days: active_days_set.len() as u32,
                peak_hour,
                daily_30d: build_timeline(today, "month", &daily_map),
                monthly_12m: build_timeline(today, "year", &monthly_map),
                clock_hours,
                recent_sessions,
            })
        })
        .map_err(|e| format!("Couldn't load app details: {e}"))
}

#[tauri::command]
pub fn act_update_app(
    state: State<'_, AppState>,
    app_id: String,
    custom_name: Option<String>,
    category: Option<String>,
    hidden: Option<bool>,
) -> CmdResult<()> {
    let cleaned_name = custom_name.map(|n| {
        let t = n.trim().to_string();
        t
    });
    let cleaned_cat = category.map(|c| AppCategory::parse(&c).as_str().to_string());

    state
        .db
        .with(|conn| {
            if let Some(ref name) = cleaned_name {
                let val: Option<&str> = (!name.is_empty()).then_some(name.as_str());
                conn.execute(
                    "UPDATE activity_apps SET custom_name = ?1 WHERE id = ?2",
                    params![val, &app_id],
                )?;
            }
            if let Some(ref cat) = cleaned_cat {
                conn.execute(
                    "UPDATE activity_apps SET category = ?1 WHERE id = ?2",
                    params![cat, &app_id],
                )?;
            }
            if let Some(h) = hidden {
                conn.execute(
                    "UPDATE activity_apps SET hidden = ?1 WHERE id = ?2",
                    params![i64::from(h), &app_id],
                )?;
            }
            Ok(())
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn act_delete_app(state: State<'_, AppState>, app_id: String) -> CmdResult<()> {
    state.activity.known_icons.lock().unwrap().remove(&app_id);
    state
        .db
        .with(|conn| {
            conn.execute("DELETE FROM activity_apps WHERE id = ?1", params![&app_id])?;
            Ok(())
        })
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nomnom_activity::{AppIdentity, HourBucket};

    #[test]
    fn persists_flushes_and_reads_totals_correctly() {
        let db = Db::open_in_memory().unwrap();
        let st = ActivityState::default();
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let flush = SessionFlush {
            app: AppIdentity {
                id: "app:clip-studio-paint".into(),
                name: "Clip Studio Paint".into(),
                exe_path: None,
                wm_class: None,
                category: AppCategory::Creative,
            },
            finished_session: Some((1_700_000_000, 1_700_003_600, 3600)),
            hourly: vec![HourBucket {
                day: today.clone(),
                hour: 21,
                secs: 3600,
                sessions: 1,
            }],
            delta_secs: 3600,
            seen_at: 1_700_003_600,
        };
        persist_flushes(&db, &st, vec![flush]);

        let total: i64 = db
            .with(|c| {
                c.query_row(
                    "SELECT total_secs FROM activity_apps WHERE id = 'app:clip-studio-paint'",
                    [],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(total, 3600);
    }
}
