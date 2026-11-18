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
];

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::Error::io(parent, e))?;
        }
        Self::init(Connection::open(path)?)
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
}
