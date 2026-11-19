//! Clipboard history. While it is switched on, text that is copied is kept on this computer, so it
//! can be found and pasted again later. Off by default. Copies a password manager marks as private
//! are left out (Windows), and an entry is forgotten after the number of days the person chose.

use crate::state::AppState;
use crate::windows::shared_webview_args;
use nomnom_core::clipboard as rules;
use nomnom_core::db::Db;
use rusqlite::params;
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;

type CmdResult<T> = Result<T, String>;

const LABEL: &str = "clipboard";
const POPUP_W: f64 = 440.0;
const POPUP_H: f64 = 560.0;
/// Newest entries kept besides the pinned ones, whatever the number of days.
const MAX_ITEMS: i64 = 3000;
const LIST_CHARS: usize = 220;

/// Until when (milliseconds since 1970) copies NomNom makes itself are not written to the history.
static SUPPRESS_UNTIL: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Keeps what NomNom copies for its own purposes (reading a selection) out of the history.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn suppress(ms: i64) {
    let until = now() * 1000 + ms;
    SUPPRESS_UNTIL.store(until, std::sync::atomic::Ordering::Relaxed);
}

fn suppressed() -> bool {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    now_ms < SUPPRESS_UNTIL.load(std::sync::atomic::Ordering::Relaxed)
}

/// The window that was in front when the popup opened, and when the popup appeared.
static BEFORE: Mutex<(isize, Option<Instant>)> = Mutex::new((0, None));

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: i64,
    /// What is shown in the list: one line.
    pub preview: String,
    /// How many characters the whole text has.
    pub chars: usize,
    pub pinned: bool,
    pub copies: i64,
    pub last_at: i64,
}

/// Keeps one copied text. The same text again only moves it to the top.
fn remember(db: &Db, text: &str) -> bool {
    let Some(text) = rules::worth_keeping(text) else {
        return false;
    };
    let print = rules::fingerprint(text);
    db.with(|c| {
        c.execute(
            "INSERT INTO clipboard_items(text, hash, pinned, copies, created_at, last_at)
             VALUES (?1, ?2, 0, 1, ?3, ?3)
             ON CONFLICT(hash) DO UPDATE SET copies = copies + 1, last_at = excluded.last_at",
            params![text, print, now()],
        )
    })
    .is_ok()
}

/// Forgets what is too old, and what is beyond the most that is kept. Pinned entries stay.
fn tidy(db: &Db, keep_days: u32) {
    let cutoff = rules::expires_before(now(), keep_days);
    let _ = db.with(|c| {
        if let Some(cutoff) = cutoff {
            c.execute(
                "DELETE FROM clipboard_items WHERE pinned = 0 AND last_at < ?1",
                params![cutoff],
            )?;
        }
        c.execute(
            "DELETE FROM clipboard_items WHERE pinned = 0 AND id NOT IN
               (SELECT id FROM clipboard_items WHERE pinned = 0 ORDER BY last_at DESC LIMIT ?1)",
            params![MAX_ITEMS],
        )
    });
}

fn current_text(app: &AppHandle) -> Option<String> {
    app.clipboard().read_text().ok()
}

#[cfg(windows)]
fn changed(last_seq: &mut u32) -> bool {
    let seq = nomnom_win::clipboard::sequence();
    let moved = seq != *last_seq;
    *last_seq = seq;
    moved && !nomnom_win::clipboard::private_copy()
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_seq = 0u32;
        let mut last_print = String::new();
        let mut tidied = Instant::now() - Duration::from_secs(7200);
        loop {
            let settings = app.state::<AppState>().settings.get().clipboard;
            if !settings.enabled {
                tokio::time::sleep(Duration::from_secs(2)).await;
                last_seq = 0;
                continue;
            }
            #[cfg(windows)]
            let fresh = changed(&mut last_seq);
            // Elsewhere there is no change counter, so the text is read and compared.
            #[cfg(not(windows))]
            let fresh = {
                let _ = &mut last_seq;
                true
            };
            if fresh {
                if let Some(text) = current_text(&app) {
                    let print = rules::fingerprint(&text);
                    // The first look after switching on only notes what is there.
                    if print != last_print {
                        let first = last_print.is_empty();
                        last_print = print;
                        if !first && !suppressed() && remember(&app.state::<AppState>().db, &text) {
                            let _ = app.emit("clipboard://changed", ());
                        }
                    }
                }
            }
            if tidied.elapsed() > Duration::from_secs(3600) {
                tidied = Instant::now();
                tidy(&app.state::<AppState>().db, settings.keep_days);
            }
            tokio::time::sleep(Duration::from_millis(if cfg!(windows) {
                500
            } else {
                1200
            }))
            .await;
        }
    });
}

// ---- the list ------------------------------------------------------------------------------------------

#[tauri::command]
pub fn cb_list(state: State<'_, AppState>, query: String, limit: u32) -> CmdResult<Vec<Entry>> {
    let limit = i64::from(limit.clamp(1, 500));
    let pattern = rules::like_pattern(&query);
    state
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT id, text, pinned, copies, last_at FROM clipboard_items
                 WHERE text LIKE ?1 ESCAPE '\\' ORDER BY pinned DESC, last_at DESC LIMIT ?2",
            )?;
            let rows = st.query_map(params![pattern, limit], |r| {
                let text: String = r.get(1)?;
                Ok(Entry {
                    id: r.get(0)?,
                    preview: rules::one_line(&text, LIST_CHARS),
                    chars: text.chars().count(),
                    pinned: r.get::<_, i64>(2)? != 0,
                    copies: r.get(3)?,
                    last_at: r.get(4)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cb_count(state: State<'_, AppState>) -> i64 {
    state
        .db
        .with(|c| c.query_row("SELECT COUNT(*) FROM clipboard_items", [], |r| r.get(0)))
        .unwrap_or(0)
}

#[tauri::command]
pub fn cb_pin(app: AppHandle, state: State<'_, AppState>, id: i64, pinned: bool) -> CmdResult<()> {
    state
        .db
        .with(|c| {
            c.execute(
                "UPDATE clipboard_items SET pinned = ?2 WHERE id = ?1",
                params![id, pinned as i64],
            )
        })
        .map_err(|e| e.to_string())?;
    let _ = app.emit("clipboard://changed", ());
    Ok(())
}

#[tauri::command]
pub fn cb_delete(app: AppHandle, state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    state
        .db
        .with(|c| c.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id]))
        .map_err(|e| e.to_string())?;
    let _ = app.emit("clipboard://changed", ());
    Ok(())
}

/// Forgets everything, or everything but the pinned entries.
#[tauri::command]
pub fn cb_clear(app: AppHandle, state: State<'_, AppState>, keep_pinned: bool) -> CmdResult<()> {
    state
        .db
        .with(|c| {
            if keep_pinned {
                c.execute("DELETE FROM clipboard_items WHERE pinned = 0", [])
            } else {
                c.execute("DELETE FROM clipboard_items", [])
            }
        })
        .map_err(|e| e.to_string())?;
    let _ = app.emit("clipboard://changed", ());
    Ok(())
}

/// The whole text of one entry (the list only carries a preview).
#[tauri::command]
pub fn cb_text(state: State<'_, AppState>, id: i64) -> CmdResult<String> {
    state
        .db
        .with(|c| {
            c.query_row(
                "SELECT text FROM clipboard_items WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
        })
        .map_err(|_| "That entry is gone.".to_string())
}

/// A link without its tracking, or nothing when there was none.
#[tauri::command]
pub fn links_clean(text: String) -> Option<String> {
    nomnom_core::links::clean(&text)
}

/// Puts an entry on the clipboard. From the popup (`paste`) it also goes into the window that was in
/// front, where the person was typing.
#[tauri::command]
pub fn cb_use(app: AppHandle, state: State<'_, AppState>, id: i64, paste: bool) -> CmdResult<()> {
    let text: String = state
        .db
        .with(|c| {
            c.query_row(
                "SELECT text FROM clipboard_items WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
        })
        .map_err(|_| "That entry is gone.".to_string())?;
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;
    if paste {
        let before = BEFORE.lock().unwrap().0;
        close_popup(&app);
        #[cfg(windows)]
        std::thread::spawn(move || nomnom_win::clipboard::paste_into(before));
        #[cfg(not(windows))]
        let _ = before;
    }
    Ok(())
}

// ---- the popup -------------------------------------------------------------------------------------------

pub fn close_popup(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.destroy();
    }
}

/// Opens the history next to the pointer, or closes it if it is open.
pub fn open_popup(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        close_popup(app);
        return;
    }
    if !app.state::<AppState>().settings.get().clipboard.enabled {
        return;
    }
    #[cfg(windows)]
    {
        BEFORE.lock().unwrap().0 = nomnom_win::clipboard::foreground();
    }
    BEFORE.lock().unwrap().1 = Some(Instant::now());
    let app = app.clone();
    // Built off the main thread (see toast.rs).
    tauri::async_runtime::spawn(async move {
        let built = WebviewWindowBuilder::new(
            &app,
            LABEL,
            WebviewUrl::App("panel.html?kind=clipboard".into()),
        )
        .title("NomNom clipboard")
        .inner_size(POPUP_W, POPUP_H)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false);
        match shared_webview_args(built).build() {
            Ok(win) => {
                place(&app, &win);
                let watched = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let opened = BEFORE.lock().unwrap().1.map(|t| t.elapsed());
                        if opened.is_some_and(|e| e > Duration::from_millis(700)) {
                            let _ = watched.destroy();
                        }
                    }
                });
            }
            Err(e) => tracing::warn!("couldn't open the clipboard history: {e}"),
        }
    });
}

fn place(app: &AppHandle, win: &tauri::WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|c| app.monitor_from_point(c.x, c.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    if let (Some(m), Ok(size)) = (monitor, win.outer_size()) {
        let area = m.work_area();
        let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
        let y = area.position.y + (area.size.height as i32 - size.height as i32) / 3;
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }
}

/// The page calls this once it has drawn, so the window never shows empty.
#[tauri::command]
pub fn cb_ready(app: AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.show();
        let _ = win.set_focus();
        BEFORE.lock().unwrap().1 = Some(Instant::now());
    }
}

/// The same as the shortcut: opens the history (or closes it).
#[tauri::command]
pub fn cb_open(app: AppHandle) {
    open_popup(&app);
}

#[tauri::command]
pub fn cb_close(app: AppHandle) {
    close_popup(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Db {
        Db::open_in_memory().unwrap()
    }

    #[test]
    fn the_same_text_copied_twice_is_one_entry_that_moves_up() {
        let db = db();
        assert!(remember(&db, "first"));
        assert!(remember(&db, "second"));
        assert!(remember(&db, "first"));
        let (n, copies): (i64, i64) = db
            .with(|c| {
                c.query_row(
                    "SELECT COUNT(*), (SELECT copies FROM clipboard_items WHERE text = 'first') FROM clipboard_items",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
            })
            .unwrap();
        assert_eq!((n, copies), (2, 2));
    }

    #[test]
    fn empty_and_huge_texts_are_not_remembered() {
        let db = db();
        assert!(!remember(&db, "   "));
        assert!(!remember(&db, &"x".repeat(rules::MAX_CHARS + 1)));
        let n: i64 = db
            .with(|c| c.query_row("SELECT COUNT(*) FROM clipboard_items", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn old_entries_are_forgotten_but_pinned_ones_stay() {
        let db = db();
        let long_ago = now() - 40 * 86_400;
        db.with(|c| {
            c.execute_batch(&format!(
                "INSERT INTO clipboard_items(text, hash, pinned, copies, created_at, last_at) VALUES
                   ('old', 'a', 0, 1, {long_ago}, {long_ago}),
                   ('old pinned', 'b', 1, 1, {long_ago}, {long_ago}),
                   ('fresh', 'c', 0, 1, {n}, {n});",
                n = now()
            ))
        })
        .unwrap();
        tidy(&db, 30);
        let left: Vec<String> = db
            .with(|c| {
                let mut st = c.prepare("SELECT text FROM clipboard_items ORDER BY text")?;
                let rows = st.query_map([], |r| r.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()
            })
            .unwrap();
        assert_eq!(left, vec!["fresh", "old pinned"]);
        // "Forever" keeps even the old one.
        db.with(|c| {
            c.execute(
                "INSERT INTO clipboard_items(text, hash, pinned, copies, created_at, last_at) VALUES ('older', 'd', 0, 1, 1, 1)",
                [],
            )
        })
        .unwrap();
        tidy(&db, 0);
        let n: i64 = db
            .with(|c| c.query_row("SELECT COUNT(*) FROM clipboard_items", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(n, 3);
    }
}
