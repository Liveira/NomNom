//! Following channels and playlists: NomNom looks now and then, and downloads the videos that came
//! out since the last look, with the person's usual settings. Videos that were already there when
//! the subscription was made are not downloaded (unless asked for).

use crate::modules::downloads::{default_choice, enqueue_with_defaults};
use crate::state::AppState;
use crate::toast::{self, Toast};
use nomnom_core::process;
use nomnom_downloads::subscriptions::{self as rules, Entry};
use nomnom_downloads::url_check::validate_url;
use nomnom_tools::Tool;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::collections::HashSet;
use std::ffi::OsString;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncReadExt;

type CmdResult<T> = Result<T, String>;

const MAX_SUBSCRIPTIONS: usize = 30;
const LIST_TIMEOUT: Duration = Duration::from_secs(120);

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize)]
pub struct SubView {
    pub id: String,
    pub url: String,
    pub title: String,
    /// "video" or "audio".
    pub kind: String,
    pub every_hours: u32,
    pub enabled: bool,
    pub last_checked: Option<i64>,
    pub started: i64,
}

/// What yt-dlp lists for a channel or playlist: its name and its newest videos.
async fn look(state: &AppState, url: &str) -> CmdResult<(Option<String>, Vec<Entry>)> {
    let exe = state
        .tools
        .locate(Tool::YtDlp)
        .ok_or("yt-dlp isn't installed yet. Open the Downloader and let it set up its tools.")?;
    let end = rules::LOOK_AT.to_string();
    let args: Vec<OsString> = [
        "-J",
        "--flat-playlist",
        "--playlist-end",
        end.as_str(),
        "--no-warnings",
        "--ignore-errors",
        "--",
        url,
    ]
    .iter()
    .map(OsString::from)
    .collect();
    let mut child = process::spawn(&exe, args, true).map_err(|e| e.to_string())?;
    let mut stdout = child.child.stdout.take().ok_or("yt-dlp gave no answer.")?;
    let run = async {
        let mut out = Vec::new();
        let mut limited = (&mut stdout).take(32 * 1024 * 1024);
        let (_, status) = tokio::join!(limited.read_to_end(&mut out), child.child.wait());
        (out, status)
    };
    let (out, status) = tokio::time::timeout(LIST_TIMEOUT, run)
        .await
        .map_err(|_| "Looking at this link took too long.".to_string())?;
    let text = String::from_utf8_lossy(&out).into_owned();
    let entries = rules::parse_entries(&text);
    if entries.is_empty() {
        let _ = status;
        return Err("That link is not a channel or a playlist with videos in it.".into());
    }
    Ok((rules::parse_title(&text), entries))
}

fn seen_of(state: &AppState, id: &str) -> HashSet<String> {
    state
        .db
        .with(|c| {
            let mut st = c.prepare("SELECT video_id FROM subscription_seen WHERE sub_id = ?1")?;
            let rows = st.query_map(params![id], |r| r.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<HashSet<_>>>()
        })
        .unwrap_or_default()
}

fn mark_seen(state: &AppState, id: &str, ids: &[&str]) {
    let _ = state.db.with(|c| {
        let tx = c.unchecked_transaction()?;
        for v in ids {
            tx.execute(
                "INSERT OR IGNORE INTO subscription_seen(sub_id, video_id) VALUES (?1, ?2)",
                params![id, v],
            )?;
        }
        tx.commit()
    });
}

/// Starts these videos with the person's usual format (video or audio, as the subscription says).
fn start(state: &AppState, kind: &str, entries: &[Entry]) -> usize {
    let mut d = state.settings.get().downloads;
    d.default_kind = if kind == "audio" {
        nomnom_core::settings::DefaultKind::Audio
    } else {
        nomnom_core::settings::DefaultKind::Video
    };
    let mut started = 0;
    for e in entries {
        let title = (!e.title.is_empty()).then(|| e.title.clone());
        if enqueue_with_defaults(state, e.url.clone(), default_choice(&d), title, false).is_ok() {
            started += 1;
        }
    }
    started
}

fn view_of(r: &rusqlite::Row<'_>) -> rusqlite::Result<SubView> {
    Ok(SubView {
        id: r.get(0)?,
        url: r.get(1)?,
        title: r.get(2)?,
        kind: r.get(3)?,
        every_hours: r.get::<_, i64>(4)?.clamp(1, 168) as u32,
        enabled: r.get::<_, i64>(5)? != 0,
        last_checked: r.get(6)?,
        started: r.get(7)?,
    })
}

const COLUMNS: &str = "id, url, title, kind, every_hours, enabled, last_checked, started";

#[tauri::command]
pub fn sub_list(state: State<'_, AppState>) -> Vec<SubView> {
    state
        .db
        .with(|c| {
            let mut st = c.prepare(&format!(
                "SELECT {COLUMNS} FROM subscriptions ORDER BY created_at, id"
            ))?;
            let rows = st.query_map([], view_of)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap_or_default()
}

/// Follows a channel or playlist. `backfill` of its newest videos are downloaded now (0 to 10).
#[tauri::command]
pub async fn sub_add(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    kind: String,
    every_hours: u32,
    backfill: u32,
) -> CmdResult<SubView> {
    let parsed = validate_url(&url).map_err(|e| e.to_string())?;
    let url = parsed.as_str().to_string();
    if !matches!(kind.as_str(), "video" | "audio") {
        return Err("Pick video or audio.".into());
    }
    let existing: Vec<SubView> = sub_list(state.clone());
    if existing.len() >= MAX_SUBSCRIPTIONS {
        return Err(format!(
            "You can follow up to {MAX_SUBSCRIPTIONS} channels or playlists."
        ));
    }
    if existing.iter().any(|s| s.url == url) {
        return Err("You already follow that.".into());
    }
    let (title, entries) = look(&state, &url).await?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let title = title.unwrap_or_else(|| url.clone());
    let every = every_hours.clamp(1, 168);
    state
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO subscriptions(id, url, title, kind, every_hours, enabled, last_checked, started, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 0, ?6)",
                params![id, url, title, kind, every, now()],
            )
        })
        .map_err(|e| e.to_string())?;
    // What is there now counts as seen; the newest `backfill` are downloaded first.
    let take = (backfill.min(10)) as usize;
    let (wanted, rest) = entries.split_at(take.min(entries.len()));
    let ids: Vec<&str> = rest.iter().map(|e| e.id.as_str()).collect();
    mark_seen(&state, &id, &ids);
    let mut oldest_first: Vec<Entry> = wanted.to_vec();
    oldest_first.reverse();
    let started = start(&state, &kind, &oldest_first);
    let wanted_ids: Vec<&str> = wanted.iter().map(|e| e.id.as_str()).collect();
    mark_seen(&state, &id, &wanted_ids);
    let _ = state.db.with(|c| {
        c.execute(
            "UPDATE subscriptions SET started = ?2 WHERE id = ?1",
            params![id, started as i64],
        )
    });
    let _ = app.emit("subscriptions://changed", ());
    state
        .db
        .with(|c| {
            c.query_row(
                &format!("SELECT {COLUMNS} FROM subscriptions WHERE id = ?1"),
                params![id],
                view_of,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sub_remove(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state
        .db
        .with(|c| {
            c.execute(
                "DELETE FROM subscription_seen WHERE sub_id = ?1",
                params![id],
            )?;
            c.execute("DELETE FROM subscriptions WHERE id = ?1", params![id])
        })
        .map_err(|e| e.to_string())?;
    let _ = app.emit("subscriptions://changed", ());
    Ok(())
}

#[tauri::command]
pub fn sub_toggle(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> CmdResult<()> {
    state
        .db
        .with(|c| {
            c.execute(
                "UPDATE subscriptions SET enabled = ?2 WHERE id = ?1",
                params![id, enabled as i64],
            )
        })
        .map_err(|e| e.to_string())?;
    let _ = app.emit("subscriptions://changed", ());
    Ok(())
}

/// Looks at one subscription now and starts what is new. Returns how many videos were started.
async fn check(app: &AppHandle, id: &str) -> CmdResult<usize> {
    let state = app.state::<AppState>();
    let sub: SubView = state
        .db
        .with(|c| {
            c.query_row(
                &format!("SELECT {COLUMNS} FROM subscriptions WHERE id = ?1"),
                params![id],
                view_of,
            )
            .optional()
        })
        .map_err(|e| e.to_string())?
        .ok_or("That subscription is gone.")?;
    // Whatever happens, the next look is a full wait away (a broken link must not be retried at once).
    let _ = state.db.with(|c| {
        c.execute(
            "UPDATE subscriptions SET last_checked = ?2 WHERE id = ?1",
            params![id, now()],
        )
    });
    let (_, entries) = look(&state, &sub.url).await?;
    let new = rules::fresh(&entries, &seen_of(&state, id));
    if new.is_empty() {
        return Ok(0);
    }
    let started = start(&state, &sub.kind, &new);
    let ids: Vec<&str> = new.iter().map(|e| e.id.as_str()).collect();
    mark_seen(&state, id, &ids);
    let _ = state.db.with(|c| {
        c.execute(
            "UPDATE subscriptions SET started = started + ?2 WHERE id = ?1",
            params![id, started as i64],
        )
    });
    if started > 0 {
        let lang = crate::i18n::lang_of(&state.settings.get());
        let (title, body) = crate::i18n::new_videos_text(lang, &sub.title, started);
        toast::show(app, Toast::new("info", title, body).opens("downloads"));
    }
    let _ = app.emit("subscriptions://changed", ());
    Ok(started)
}

#[tauri::command]
pub async fn sub_check_now(app: AppHandle, id: String) -> CmdResult<usize> {
    check(&app, &id).await
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(90)).await;
        loop {
            let due: Vec<String> = {
                let state = app.state::<AppState>();
                sub_list_due(&state)
            };
            for id in due {
                let _ = check(&app, &id).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            tokio::time::sleep(Duration::from_secs(600)).await;
        }
    });
}

fn sub_list_due(state: &AppState) -> Vec<String> {
    let t = now();
    state
        .db
        .with(|c| {
            let mut st = c.prepare(&format!(
                "SELECT {COLUMNS} FROM subscriptions WHERE enabled = 1"
            ))?;
            let rows = st.query_map([], view_of)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|s| rules::due(s.last_checked, s.every_hours, t))
        .map(|s| s.id)
        .collect()
}
