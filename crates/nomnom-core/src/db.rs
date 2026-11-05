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
        Ok(Db { conn: Mutex::new(conn) })
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
        assert_eq!(Db::open(&path).unwrap().schema_version().unwrap(), MIGRATIONS.len());
        let db = Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), MIGRATIONS.len());
        let n: i64 = db
            .with(|c| c.query_row("SELECT count(*) FROM org_ops", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(n, 0);
    }
}
