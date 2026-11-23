//! SQLite persistence with embedded, versioned migrations.

use crate::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE downloads(
        id TEXT PRIMARY KEY, url TEXT NOT NULL, canonical_key TEXT, title TEXT, source TEXT,
        kind TEXT, format TEXT, quality TEXT, status TEXT NOT NULL, dest_path TEXT,
        bytes INTEGER, error TEXT, created_at INTEGER NOT NULL, finished_at INTEGER);
     CREATE INDEX downloads_key ON downloads(canonical_key);
     CREATE TABLE conversions(
        id TEXT PRIMARY KEY, batch_id TEXT, src TEXT NOT NULL, dst TEXT, preset TEXT,
        status TEXT NOT NULL, error TEXT, created_at INTEGER NOT NULL, finished_at INTEGER);
     CREATE TABLE org_batches(
        id TEXT PRIMARY KEY, mode TEXT NOT NULL, source_root TEXT NOT NULL,
        dest_root TEXT NOT NULL, created_at INTEGER NOT NULL, state TEXT NOT NULL);
     CREATE TABLE org_ops(
        id TEXT PRIMARY KEY, batch_id TEXT NOT NULL REFERENCES org_batches(id),
        seq INTEGER NOT NULL, kind TEXT NOT NULL, src TEXT NOT NULL, dst TEXT NOT NULL,
        fp_size INTEGER, fp_mtime INTEGER, fp_file_id TEXT, fp_volume INTEGER,
        state TEXT NOT NULL, reason TEXT, updated_at INTEGER NOT NULL);
     CREATE INDEX org_ops_batch ON org_ops(batch_id, seq);",
    // v2: what a download needs to be retried or resumed after a restart
    "ALTER TABLE downloads ADD COLUMN dest_dir TEXT; ALTER TABLE downloads ADD COLUMN cookies INTEGER NOT NULL DEFAULT 0;",
    // v3: optional folder (inside the save folder) that a finished download is sorted into
    "ALTER TABLE downloads ADD COLUMN subdir TEXT;",
    // v4: where a moved file ended up, so undo can tell whether it was touched since
    "ALTER TABLE org_ops ADD COLUMN dst_size INTEGER; ALTER TABLE org_ops ADD COLUMN dst_mtime INTEGER; ALTER TABLE org_ops ADD COLUMN dst_file_id TEXT;",
    // v5: Disk Analyzer snapshots. Folder sizes down to a depth, and the big files, as of one moment.
    "CREATE TABLE storage_snapshots(
        id INTEGER PRIMARY KEY AUTOINCREMENT, root TEXT NOT NULL, label TEXT NOT NULL DEFAULT '',
        source TEXT NOT NULL DEFAULT 'manual', created_at INTEGER NOT NULL,
        files INTEGER NOT NULL, dirs INTEGER NOT NULL, bytes INTEGER NOT NULL, alloc INTEGER NOT NULL,
        depth INTEGER NOT NULL, big_min INTEGER NOT NULL, errors INTEGER NOT NULL DEFAULT 0);
     CREATE INDEX storage_snapshots_root ON storage_snapshots(root, created_at);
     CREATE TABLE storage_snapshot_dirs(
        snapshot_id INTEGER NOT NULL REFERENCES storage_snapshots(id) ON DELETE CASCADE,
        path TEXT NOT NULL, size INTEGER NOT NULL, alloc INTEGER NOT NULL, files INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, path)) WITHOUT ROWID;
     CREATE TABLE storage_snapshot_files(
        snapshot_id INTEGER NOT NULL REFERENCES storage_snapshots(id) ON DELETE CASCADE,
        path TEXT NOT NULL, size INTEGER NOT NULL, mtime INTEGER,
        PRIMARY KEY(snapshot_id, path)) WITHOUT ROWID;",
    // v6: Active app & game time tracking (Playtime). Only foreground active seconds are stored;
    // window titles and URLs are never persisted.
    "CREATE TABLE activity_apps(
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        custom_name TEXT,
        exe_path TEXT,
        wm_class TEXT,
        category TEXT NOT NULL DEFAULT 'general',
        icon_png BLOB,
        hidden INTEGER NOT NULL DEFAULT 0,
        first_seen INTEGER NOT NULL,
        last_seen INTEGER NOT NULL,
        total_secs INTEGER NOT NULL DEFAULT 0);
     CREATE TABLE activity_hourly(
        app_id TEXT NOT NULL REFERENCES activity_apps(id) ON DELETE CASCADE,
        day TEXT NOT NULL,
        hour INTEGER NOT NULL,
        active_secs INTEGER NOT NULL DEFAULT 0,
        sessions INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY(app_id, day, hour)) WITHOUT ROWID;
     CREATE INDEX activity_hourly_day ON activity_hourly(day);
     CREATE TABLE activity_sessions(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_id TEXT NOT NULL REFERENCES activity_apps(id) ON DELETE CASCADE,
        started_at INTEGER NOT NULL,
        ended_at INTEGER NOT NULL,
        active_secs INTEGER NOT NULL);
     CREATE INDEX activity_sessions_time ON activity_sessions(started_at DESC);
     CREATE INDEX activity_sessions_app ON activity_sessions(app_id, started_at DESC);",
    // v7: Focus mode blocks (a timed stretch of work or a break) and how they ended.
    "CREATE TABLE focus_blocks(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        started_at INTEGER NOT NULL,
        ended_at INTEGER,
        planned_secs INTEGER NOT NULL,
        kind TEXT NOT NULL,
        label TEXT NOT NULL DEFAULT '',
        completed INTEGER NOT NULL DEFAULT 0);
     CREATE INDEX focus_blocks_time ON focus_blocks(started_at DESC);",
    // v8: Downloads that wait for a chosen time before they start.
    "CREATE TABLE scheduled_downloads(
        id TEXT PRIMARY KEY,
        url TEXT NOT NULL,
        title TEXT,
        choice TEXT NOT NULL,
        run_at INTEGER NOT NULL,
        created_at INTEGER NOT NULL);
     CREATE INDEX scheduled_downloads_time ON scheduled_downloads(run_at);",
    // v9: Backup plans and a record of each run.
    "CREATE TABLE backup_plans(
        id TEXT PRIMARY KEY,
        json TEXT NOT NULL,
        created_at INTEGER NOT NULL);
     CREATE TABLE backup_runs(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        plan_id TEXT NOT NULL,
        started_at INTEGER NOT NULL,
        ended_at INTEGER NOT NULL,
        ok INTEGER NOT NULL,
        files INTEGER NOT NULL DEFAULT 0,
        bytes INTEGER NOT NULL DEFAULT 0,
        copied INTEGER NOT NULL DEFAULT 0,
        version TEXT,
        message TEXT);
     CREATE INDEX backup_runs_plan ON backup_runs(plan_id, id DESC);",
    // v10: Playtime goals, the alerts already shown for them, and the weekly reports.
    "CREATE TABLE activity_goals(
        id TEXT PRIMARY KEY,
        scope TEXT NOT NULL,
        kind TEXT NOT NULL,
        period TEXT NOT NULL,
        minutes INTEGER NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at INTEGER NOT NULL);
     CREATE TABLE activity_goal_alerts(key TEXT PRIMARY KEY, at INTEGER NOT NULL) WITHOUT ROWID;
     CREATE TABLE activity_reports(week_start TEXT PRIMARY KEY, json TEXT NOT NULL, created_at INTEGER NOT NULL) WITHOUT ROWID;",
    // v11: clipboard history (text only, on this computer).
    "CREATE TABLE clipboard_items(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        text TEXT NOT NULL,
        hash TEXT NOT NULL UNIQUE,
        pinned INTEGER NOT NULL DEFAULT 0,
        copies INTEGER NOT NULL DEFAULT 1,
        created_at INTEGER NOT NULL,
        last_at INTEGER NOT NULL);
     CREATE INDEX clipboard_items_last ON clipboard_items(pinned DESC, last_at DESC);",
    // v12: channels and playlists the Downloader follows, and the videos already dealt with.
    "CREATE TABLE subscriptions(
        id TEXT PRIMARY KEY,
        url TEXT NOT NULL UNIQUE,
        title TEXT NOT NULL,
        kind TEXT NOT NULL,
        every_hours INTEGER NOT NULL DEFAULT 6,
        enabled INTEGER NOT NULL DEFAULT 1,
        last_checked INTEGER,
        started INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL);
     CREATE TABLE subscription_seen(
        sub_id TEXT NOT NULL,
        video_id TEXT NOT NULL,
        PRIMARY KEY(sub_id, video_id)) WITHOUT ROWID;",
    // v13: reminders and quick notes.
    "CREATE TABLE reminders(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        text TEXT NOT NULL,
        due_at INTEGER NOT NULL,
        repeat TEXT,
        done INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL);
     CREATE INDEX reminders_due ON reminders(done, due_at);
     CREATE TABLE notes(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        text TEXT NOT NULL,
        pinned INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL);
     CREATE INDEX notes_order ON notes(pinned DESC, updated_at DESC);",
    // v14: the notices NomNom showed, kept for a few days.
    "CREATE TABLE notifications(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        at INTEGER NOT NULL,
        kind TEXT NOT NULL,
        title TEXT NOT NULL,
        body TEXT NOT NULL,
        action TEXT);
     CREATE INDEX notifications_at ON notifications(at DESC);",
    // v15: what folder rules did, and how far each rule had looked.
    "CREATE TABLE rule_log(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        at INTEGER NOT NULL,
        rule TEXT NOT NULL,
        step TEXT NOT NULL,
        src TEXT NOT NULL,
        dst TEXT NOT NULL,
        ok INTEGER NOT NULL,
        note TEXT NOT NULL,
        undone INTEGER NOT NULL DEFAULT 0);
     CREATE TABLE rule_state(
        rule_id TEXT PRIMARY KEY,
        scanned_at INTEGER NOT NULL) WITHOUT ROWID;",
    // v16: text snippets.
    "CREATE TABLE snippets(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        body TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        used_at INTEGER NOT NULL DEFAULT 0);",
    // v17: Mobile Manager timeline, custom deck shortcuts and muted notification apps.
    "CREATE TABLE mobile_timeline(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        device_id TEXT NOT NULL,
        device_name TEXT NOT NULL,
        kind TEXT NOT NULL,
        title TEXT NOT NULL,
        detail TEXT NOT NULL,
        at INTEGER NOT NULL);
     CREATE INDEX mobile_timeline_at ON mobile_timeline(at DESC);
     CREATE TABLE mobile_deck(
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        icon TEXT NOT NULL,
        kind TEXT NOT NULL,
        payload TEXT NOT NULL,
        created_at INTEGER NOT NULL);
     CREATE TABLE mobile_muted(
        package_name TEXT PRIMARY KEY,
        muted_at INTEGER NOT NULL) WITHOUT ROWID;",
    // v18: Saved Wi-Fi ADB devices for 1-click and automatic wireless reconnect.
    "CREATE TABLE mobile_wifi_devices(
        endpoint TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        ip TEXT NOT NULL,
        last_seen INTEGER NOT NULL) WITHOUT ROWID;",
];

/// How many row numbers are tried per table when a damaged table has to be read row by row.
const MAX_SALVAGE_ROWID: i64 = 200_000;

/// True when SQLite says the file is damaged. Other failures (a locked file, say) are real errors
/// and are passed on, so a busy database is never mistaken for a broken one.
fn is_damaged(conn: &Connection) -> Result<bool> {
    use rusqlite::ErrorCode::{DatabaseCorrupt, NotADatabase};
    match conn.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0)) {
        Ok(answer) => Ok(answer != "ok"),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if matches!(e.code, DatabaseCorrupt | NotADatabase) =>
        {
            Ok(true)
        }
        Err(e) => Err(e.into()),
    }
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::Error::io(parent, e))?;
        }
        let conn = Connection::open(path)?;
        if is_damaged(&conn)? {
            drop(conn);
            return Self::recover(path);
        }
        Self::init(conn)
    }

    /// A damaged database file is moved aside, untouched, into a `damaged-db-<time>` folder next to
    /// it, and a fresh database is filled with whatever the old one can still give back.
    fn recover(path: &Path) -> Result<Self> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let keep = parent.join(format!("damaged-db-{stamp}"));
        std::fs::create_dir_all(&keep).map_err(|e| crate::Error::io(&keep, e))?;
        let name = path.file_name().unwrap_or_default().to_os_string();
        for suffix in ["", "-wal", "-shm"] {
            let mut file = name.clone();
            file.push(suffix);
            let from = parent.join(&file);
            if from.exists() {
                std::fs::rename(&from, keep.join(&file)).map_err(|e| crate::Error::io(&from, e))?;
            }
        }
        tracing::error!(
            "the database was damaged; the original is kept in {}",
            keep.display()
        );
        let db = Self::init(Connection::open(path)?)?;
        match db.copy_readable_from(&keep.join(&name)) {
            Ok(report) => tracing::warn!("database rebuilt: {report}"),
            Err(e) => tracing::error!("couldn't read anything from the damaged database: {e}"),
        }
        Ok(db)
    }

    /// Copies every table from `old`, table by table. A table that can't be read in one go is read
    /// row by row, keeping the rows that are still intact.
    fn copy_readable_from(&self, old: &Path) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "ATTACH DATABASE ?1 AS old",
            [old.to_string_lossy().as_ref()],
        )?;
        let tables: Vec<String> = {
            let mut st = conn.prepare(
                "SELECT name FROM main.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            )?;
            let rows = st.query_map([], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut report = Vec::new();
        for table in tables {
            let columns = |schema: &str| -> rusqlite::Result<Vec<String>> {
                let mut st = conn.prepare(&format!("PRAGMA {schema}.table_info(\"{table}\")"))?;
                let rows = st.query_map([], |r| r.get::<_, String>(1))?;
                rows.collect()
            };
            let (Ok(mine), Ok(theirs)) = (columns("main"), columns("old")) else {
                report.push(format!("{table}: unreadable"));
                continue;
            };
            let common: Vec<String> = mine
                .into_iter()
                .filter(|c| theirs.contains(c))
                .map(|c| format!("\"{c}\""))
                .collect();
            if common.is_empty() {
                continue;
            }
            let list = common.join(", ");
            let bulk = conn
                .execute_batch(&format!(
                    "BEGIN; INSERT INTO main.\"{table}\" ({list}) SELECT {list} FROM old.\"{table}\"; COMMIT;"
                ))
                .is_ok();
            if bulk {
                continue;
            }
            let _ = conn.execute_batch("ROLLBACK");
            let marks = vec!["?"; common.len()].join(", ");
            let mut kept = 0usize;
            let _ = conn.execute_batch("BEGIN");
            for rowid in 1..=MAX_SALVAGE_ROWID {
                let row = conn
                    .prepare_cached(&format!(
                        "SELECT {list} FROM old.\"{table}\" WHERE rowid = ?1"
                    ))
                    .and_then(|mut read| {
                        read.query_row([rowid], |r| {
                            (0..common.len())
                                .map(|i| r.get::<_, rusqlite::types::Value>(i))
                                .collect::<rusqlite::Result<Vec<_>>>()
                        })
                    });
                if let Ok(values) = row {
                    let put = conn.execute(
                        &format!(
                            "INSERT OR IGNORE INTO main.\"{table}\" ({list}) VALUES ({marks})"
                        ),
                        rusqlite::params_from_iter(values),
                    );
                    if matches!(put, Ok(n) if n > 0) {
                        kept += 1;
                    }
                }
            }
            let _ = conn.execute_batch("COMMIT");
            report.push(format!("{table}: {kept} rows salvaged"));
        }
        let _ = conn.execute_batch("DETACH DATABASE old");
        Ok(if report.is_empty() {
            "every table copied in full".to_string()
        } else {
            report.join("; ")
        })
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        // The organizer journal must survive a crash, so keep full durability.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        let version: usize = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
            tx.commit()?;
        }
        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        let guard = self.conn.lock().unwrap();
        Ok(f(&guard)?)
    }

    pub fn schema_version(&self) -> Result<usize> {
        self.with(|c| c.query_row("PRAGMA user_version", [], |r| r.get(0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_once_and_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        assert_eq!(
            Db::open(&path).unwrap().schema_version().unwrap(),
            MIGRATIONS.len()
        );
        let db = Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), MIGRATIONS.len());
        let n: i64 = db
            .with(|c| c.query_row("SELECT count(*) FROM org_ops", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(n, 0);
    }

    fn damaged_copy_of_full_db(path: &Path) {
        let db = Db::open(path).unwrap();
        db.with(|c| {
            c.execute_batch(
                "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 3000)
                 INSERT INTO backup_runs(plan_id, started_at, ended_at, ok, message)
                 SELECT 'plan', i, i, 1, 'padding padding padding padding padding padding' FROM n;
                 PRAGMA wal_checkpoint(TRUNCATE);",
            )
        })
        .unwrap();
        drop(db);
        let mut bytes = std::fs::read(path).unwrap();
        let page = 4096;
        assert!(bytes.len() > 8 * page, "needs a few pages to damage one");
        for b in &mut bytes[3 * page..7 * page] {
            *b = 0xFF;
        }
        std::fs::write(path, &bytes).unwrap();
    }

    #[test]
    fn a_damaged_database_is_moved_aside_and_rebuilt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        damaged_copy_of_full_db(&path);

        let db = Db::open(&path).expect("the app still starts");
        assert_eq!(db.schema_version().unwrap(), MIGRATIONS.len());
        let ok: String = db
            .with(|c| c.query_row("PRAGMA integrity_check", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(ok, "ok");
        let rows: i64 = db
            .with(|c| c.query_row("SELECT count(*) FROM backup_runs", [], |r| r.get(0)))
            .unwrap();
        assert!(
            rows > 0 && rows <= 3000,
            "kept the readable rows, got {rows}"
        );
        db.with(|c| {
            c.execute(
                "INSERT INTO backup_runs(plan_id, started_at, ended_at, ok) VALUES ('p', 1, 1, 1)",
                [],
            )
        })
        .expect("the new database takes writes");
        let kept: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("damaged-db-"))
            .collect();
        assert_eq!(kept.len(), 1, "the damaged original is kept");
        assert!(kept[0].path().join("t.db").exists());
    }

    #[test]
    fn a_healthy_database_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        drop(Db::open(&path).unwrap());
        drop(Db::open(&path).unwrap());
        assert!(!std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("damaged-db-")));
    }
}
