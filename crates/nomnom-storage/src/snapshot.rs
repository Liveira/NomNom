//! Snapshots: what a scan found, kept in the database so it can be compared with a later scan.
//!
//! A snapshot keeps the size of every folder down to a chosen depth, and the big files anywhere.
//! It is always a picture of one moment. Comparing two snapshots only compares what both of them
//! kept, and says so.

use crate::index::{filetime_to_unix, Index, F_DIR, F_LINK};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

pub const DEFAULT_DEPTH: u32 = 6;
pub const DEFAULT_BIG_FILE: u64 = 50 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirRow {
    /// Path below the scanned folder, with `\` between names. Empty for the scanned folder itself.
    pub path: String,
    pub size: u64,
    pub alloc: u64,
    pub files: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    pub path: String,
    pub size: u64,
    pub mtime: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct Captured {
    pub dirs: Vec<DirRow>,
    pub files: Vec<FileRow>,
    pub depth: u32,
    pub big_min: u64,
    pub total_files: u64,
    pub total_dirs: u64,
    pub total_bytes: u64,
    pub total_alloc: u64,
}

fn depth_of(path: &str) -> u32 {
    if path.is_empty() {
        0
    } else {
        path.matches('\\').count() as u32 + 1
    }
}

/// Reads the sizes out of a finished scan. Links are not followed and not recorded.
pub fn capture(idx: &Index, depth: u32, big_min: u64) -> Captured {
    let root = idx.root();
    let r = idx.node(root).expect("an index always has a root");
    let mut out = Captured {
        depth,
        big_min,
        total_files: u64::from(r.files),
        total_dirs: u64::from(r.dirs),
        total_bytes: r.size,
        total_alloc: r.alloc,
        ..Captured::default()
    };
    let mut stack: Vec<(u32, String)> = vec![(root, String::new())];
    while let Some((id, rel)) = stack.pop() {
        let n = idx.node(id).unwrap();
        let d = depth_of(&rel);
        if d <= depth {
            out.dirs.push(DirRow {
                path: rel.clone(),
                size: n.size,
                alloc: n.alloc,
                files: u64::from(n.files),
            });
        }
        for c in idx.children(id) {
            let cn = idx.node(c).unwrap();
            if cn.flags & F_LINK != 0 {
                continue;
            }
            let path = if rel.is_empty() {
                idx.name(c).to_string()
            } else {
                format!("{rel}\\{}", idx.name(c))
            };
            if cn.flags & F_DIR != 0 {
                stack.push((c, path));
            } else if cn.size >= big_min {
                out.files.push(FileRow {
                    path,
                    size: cn.size,
                    mtime: filetime_to_unix(cn.mtime),
                });
            }
        }
    }
    out.dirs.sort_by(|a, b| a.path.cmp(&b.path));
    out.files.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

#[derive(Clone, Debug, Serialize)]
pub struct Meta {
    pub id: i64,
    pub root: String,
    pub label: String,
    /// "manual" or "scheduled".
    pub source: String,
    pub created_at: i64,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub alloc: u64,
    pub depth: u32,
    pub big_min: u64,
    /// Places the scan couldn't read. Their sizes are not in the totals.
    pub errors: u64,
}

pub struct NewSnapshot<'a> {
    pub root: &'a str,
    pub label: &'a str,
    pub source: &'a str,
    pub created_at: i64,
    pub errors: u64,
}

pub fn save(conn: &Connection, s: &NewSnapshot, c: &Captured) -> rusqlite::Result<i64> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO storage_snapshots(root, label, source, created_at, files, dirs, bytes, alloc, depth, big_min, errors)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            s.root,
            s.label,
            s.source,
            s.created_at,
            c.total_files as i64,
            c.total_dirs as i64,
            c.total_bytes as i64,
            c.total_alloc as i64,
            c.depth,
            c.big_min as i64,
            s.errors as i64
        ],
    )?;
    let id = tx.last_insert_rowid();
    {
        let mut d = tx.prepare(
            "INSERT INTO storage_snapshot_dirs(snapshot_id, path, size, alloc, files) VALUES(?1,?2,?3,?4,?5)",
        )?;
        for r in &c.dirs {
            d.execute(params![
                id,
                r.path,
                r.size as i64,
                r.alloc as i64,
                r.files as i64
            ])?;
        }
        let mut f = tx.prepare(
            "INSERT INTO storage_snapshot_files(snapshot_id, path, size, mtime) VALUES(?1,?2,?3,?4)",
        )?;
        for r in &c.files {
            f.execute(params![id, r.path, r.size as i64, r.mtime])?;
        }
    }
    tx.commit()?;
    Ok(id)
}

fn meta_of(r: &rusqlite::Row) -> rusqlite::Result<Meta> {
    Ok(Meta {
        id: r.get(0)?,
        root: r.get(1)?,
        label: r.get(2)?,
        source: r.get(3)?,
        created_at: r.get(4)?,
        files: r.get::<_, i64>(5)? as u64,
        dirs: r.get::<_, i64>(6)? as u64,
        bytes: r.get::<_, i64>(7)? as u64,
        alloc: r.get::<_, i64>(8)? as u64,
        depth: r.get(9)?,
        big_min: r.get::<_, i64>(10)? as u64,
        errors: r.get::<_, i64>(11)? as u64,
    })
}

const META_COLS: &str =
    "id, root, label, source, created_at, files, dirs, bytes, alloc, depth, big_min, errors";

/// Snapshots of one scanned folder (any letter case), oldest first.
pub fn list(conn: &Connection, root: &str) -> rusqlite::Result<Vec<Meta>> {
    let mut st = conn.prepare(&format!(
        "SELECT {META_COLS} FROM storage_snapshots WHERE lower(root) = lower(?1) ORDER BY created_at, id"
    ))?;
    let rows = st.query_map([root], meta_of)?;
    rows.collect()
}

pub fn meta(conn: &Connection, id: i64) -> rusqlite::Result<Option<Meta>> {
    let mut st = conn.prepare(&format!(
        "SELECT {META_COLS} FROM storage_snapshots WHERE id = ?1"
    ))?;
    let mut rows = st.query_map([id], meta_of)?;
    rows.next().transpose()
}

pub fn load(conn: &Connection, id: i64) -> rusqlite::Result<Option<Captured>> {
    let Some(m) = meta(conn, id)? else {
        return Ok(None);
    };
    let mut c = Captured {
        depth: m.depth,
        big_min: m.big_min,
        total_files: m.files,
        total_dirs: m.dirs,
        total_bytes: m.bytes,
        total_alloc: m.alloc,
        ..Captured::default()
    };
    let mut d = conn.prepare(
        "SELECT path, size, alloc, files FROM storage_snapshot_dirs WHERE snapshot_id = ?1",
    )?;
    c.dirs = d
        .query_map([id], |r| {
            Ok(DirRow {
                path: r.get(0)?,
                size: r.get::<_, i64>(1)? as u64,
                alloc: r.get::<_, i64>(2)? as u64,
                files: r.get::<_, i64>(3)? as u64,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut f = conn
        .prepare("SELECT path, size, mtime FROM storage_snapshot_files WHERE snapshot_id = ?1")?;
    c.files = f
        .query_map([id], |r| {
            Ok(FileRow {
                path: r.get(0)?,
                size: r.get::<_, i64>(1)? as u64,
                mtime: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(c))
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM storage_snapshot_dirs WHERE snapshot_id = ?1",
        [id],
    )?;
    tx.execute(
        "DELETE FROM storage_snapshot_files WHERE snapshot_id = ?1",
        [id],
    )?;
    let n = tx.execute("DELETE FROM storage_snapshots WHERE id = ?1", [id])?;
    tx.commit()?;
    Ok(n > 0)
}

/// Keeps the newest `keep` snapshots of this source for this folder and deletes the rest. Snapshots
/// the person made by hand are never pruned (call it with source "scheduled").
pub fn prune(conn: &Connection, root: &str, source: &str, keep: usize) -> rusqlite::Result<usize> {
    let mut st = conn.prepare(
        "SELECT id FROM storage_snapshots WHERE lower(root) = lower(?1) AND source = ?2 ORDER BY created_at DESC, id DESC",
    )?;
    let ids: Vec<i64> = st
        .query_map(params![root, source], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut n = 0;
    for id in ids.into_iter().skip(keep) {
        if delete(conn, id)? {
            n += 1;
        }
    }
    Ok(n)
}

/// When the folder was last snapshotted by this source, as Unix seconds.
pub fn last_time(conn: &Connection, root: &str, source: &str) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT max(created_at) FROM storage_snapshots WHERE lower(root) = lower(?1) AND source = ?2",
        params![root, source],
        |r| r.get(0),
    )
}

/// The folder name as snapshots store it: no closing `\`, except for a drive root like `C:\`.
pub fn root_key(root: &std::path::Path) -> String {
    let s = root.to_string_lossy().replace('/', "\\");
    let t = s.trim_end_matches('\\');
    if t.len() == 2 && t.ends_with(':') {
        format!("{t}\\")
    } else {
        t.to_string()
    }
}

/// Whether a scheduled scan is due. `ran_since_start` is whether this run of NomNom already did it.
pub fn schedule_due(schedule: &str, last: Option<i64>, now: i64, ran_since_start: bool) -> bool {
    let every = |secs: i64| last.is_none_or(|l| now - l >= secs);
    match schedule {
        "on_start" => !ran_since_start,
        "daily" => every(86_400),
        "weekly" => every(7 * 86_400),
        _ => false,
    }
}

// ---- comparing -----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    Grew,
    Shrank,
    New,
    Gone,
}

#[derive(Clone, Debug, Serialize)]
pub struct DirChange {
    pub path: String,
    pub change: Change,
    pub before: Option<u64>,
    pub after: Option<u64>,
    pub delta: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileChange {
    pub path: String,
    pub change: Change,
    pub before: Option<u64>,
    pub after: Option<u64>,
    pub delta: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Comparison {
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub delta: i64,
    pub files_before: u64,
    pub files_after: u64,
    /// Folders this deep (and shallower) were compared; both snapshots kept that much.
    pub depth: u32,
    /// Files at least this big were compared.
    pub big_min: u64,
    pub dirs: Vec<DirChange>,
    pub dirs_changed: usize,
    pub files: Vec<FileChange>,
    pub files_changed: usize,
}

fn key(p: &str) -> String {
    p.to_lowercase()
}

fn inside(path: &str, folders: &HashSet<String>) -> bool {
    // Is `path` below one of the folders that are new or gone as a whole?
    let k = key(path);
    let mut end = 0;
    while let Some(i) = k[end..].find('\\') {
        end += i;
        if folders.contains(&k[..end]) {
            return true;
        }
        end += 1;
    }
    false
}

/// What changed between two captures of the same folder. Only what both kept is compared. A folder
/// that is new or gone is listed once, not once for everything inside it.
pub fn compare(before: &Captured, after: &Captured, limit: usize) -> Comparison {
    let depth = before.depth.min(after.depth);
    let big_min = before.big_min.max(after.big_min);

    let map = |c: &Captured| -> HashMap<String, (String, u64)> {
        c.dirs
            .iter()
            .filter(|d| depth_of(&d.path) <= depth)
            .map(|d| (key(&d.path), (d.path.clone(), d.size)))
            .collect()
    };
    let a = map(before);
    let b = map(after);

    let mut whole: HashSet<String> = HashSet::new();
    for k in a.keys().chain(b.keys()) {
        if !k.is_empty() && (!a.contains_key(k) || !b.contains_key(k)) {
            whole.insert(k.clone());
        }
    }

    let mut dirs: Vec<DirChange> = Vec::new();
    for (k, (path, old)) in &a {
        if k.is_empty() {
            continue;
        }
        match b.get(k) {
            None => {
                if !inside(path, &whole) {
                    dirs.push(DirChange {
                        path: path.clone(),
                        change: Change::Gone,
                        before: Some(*old),
                        after: None,
                        delta: -(*old as i64),
                    });
                }
            }
            Some((_, new)) if new != old => dirs.push(DirChange {
                path: path.clone(),
                change: if new > old {
                    Change::Grew
                } else {
                    Change::Shrank
                },
                before: Some(*old),
                after: Some(*new),
                delta: *new as i64 - *old as i64,
            }),
            _ => {}
        }
    }
    for (k, (path, new)) in &b {
        if k.is_empty() || a.contains_key(k) {
            continue;
        }
        if !inside(path, &whole) {
            dirs.push(DirChange {
                path: path.clone(),
                change: Change::New,
                before: None,
                after: Some(*new),
                delta: *new as i64,
            });
        }
    }
    dirs.sort_by(|x, y| {
        y.delta
            .unsigned_abs()
            .cmp(&x.delta.unsigned_abs())
            .then_with(|| x.path.cmp(&y.path))
    });

    let fmap = |c: &Captured| -> HashMap<String, (String, u64)> {
        c.files
            .iter()
            .filter(|f| f.size >= big_min)
            .map(|f| (key(&f.path), (f.path.clone(), f.size)))
            .collect()
    };
    let fa = fmap(before);
    let fb = fmap(after);
    let mut files: Vec<FileChange> = Vec::new();
    for (k, (path, old)) in &fa {
        match fb.get(k) {
            None => {
                if !inside(path, &whole) {
                    files.push(FileChange {
                        path: path.clone(),
                        change: Change::Gone,
                        before: Some(*old),
                        after: None,
                        delta: -(*old as i64),
                    });
                }
            }
            Some((_, new)) if new != old => files.push(FileChange {
                path: path.clone(),
                change: if new > old {
                    Change::Grew
                } else {
                    Change::Shrank
                },
                before: Some(*old),
                after: Some(*new),
                delta: *new as i64 - *old as i64,
            }),
            _ => {}
        }
    }
    for (k, (path, new)) in &fb {
        if !fa.contains_key(k) && !inside(path, &whole) {
            files.push(FileChange {
                path: path.clone(),
                change: Change::New,
                before: None,
                after: Some(*new),
                delta: *new as i64,
            });
        }
    }
    files.sort_by(|x, y| {
        y.delta
            .unsigned_abs()
            .cmp(&x.delta.unsigned_abs())
            .then_with(|| x.path.cmp(&y.path))
    });

    let dirs_changed = dirs.len();
    let files_changed = files.len();
    dirs.truncate(limit);
    files.truncate(limit);
    Comparison {
        bytes_before: before.total_bytes,
        bytes_after: after.total_bytes,
        delta: after.total_bytes as i64 - before.total_bytes as i64,
        files_before: before.total_files,
        files_after: after.total_files,
        depth,
        big_min,
        dirs,
        dirs_changed,
        files,
        files_changed,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TrendPoint {
    pub id: i64,
    pub created_at: i64,
    /// `None` when that snapshot didn't keep this folder (it didn't exist, or it is deeper than the snapshot went).
    pub size: Option<u64>,
}

/// The size of one folder (`""` for the scanned folder) in each snapshot of this root, oldest first.
pub fn trend(conn: &Connection, root: &str, folder: &str) -> rusqlite::Result<Vec<TrendPoint>> {
    let mut out = Vec::new();
    for m in list(conn, root)? {
        let size: Option<i64> = conn
            .query_row(
                "SELECT size FROM storage_snapshot_dirs WHERE snapshot_id = ?1 AND lower(path) = lower(?2)",
                params![m.id, folder],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e),
            })?;
        out.push(TrendPoint {
            id: m.id,
            created_at: m.created_at,
            size: size.map(|s| s as u64),
        });
    }
    Ok(out)
}
