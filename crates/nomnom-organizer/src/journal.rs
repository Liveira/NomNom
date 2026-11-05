//! The record of every move, written to SQLite before and after each file moves, so a crash or
//! power cut can always be understood afterwards and every batch can be undone.

use crate::model::{Fingerprint, Move, OrganizerError};
use nomnom_core::db::Db;
use rusqlite::{params, Row};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn err(e: impl std::fmt::Display) -> OrganizerError {
    OrganizerError::Journal(e.to_string())
}

/// Where a file ended up, recorded once it has moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DstPrint {
    pub size: u64,
    pub mtime: i64,
    pub volume: u32,
    pub index: u64,
}

#[derive(Debug, Clone)]
pub struct OpRow {
    pub id: String,
    pub batch_id: String,
    pub seq: i64,
    pub kind: String,
    pub src: PathBuf,
    pub dst: PathBuf,
    pub fp: Fingerprint,
    pub state: String,
    pub reason: Option<String>,
    pub dst_print: Option<DstPrint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchRow {
    pub id: String,
    pub mode: String,
    pub source_root: String,
    pub dest_root: String,
    pub created_at: i64,
    pub state: String,
    pub moved: i64,
    pub undone: i64,
    pub failed: i64,
}

#[derive(Clone)]
pub struct Journal {
    db: Arc<Db>,
}

fn hex_u64(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap_or(0)
}

fn map_op(r: &Row<'_>) -> rusqlite::Result<OpRow> {
    let fp = Fingerprint {
        size: r.get::<_, i64>(7)? as u64,
        mtime: r.get(8)?,
        index: hex_u64(&r.get::<_, String>(9)?),
        volume: r.get::<_, i64>(10)? as u32,
    };
    let dst_size: Option<i64> = r.get(13)?;
    let dst_print = match (
        dst_size,
        r.get::<_, Option<i64>>(14)?,
        r.get::<_, Option<String>>(15)?,
    ) {
        (Some(size), Some(mtime), Some(id)) => {
            let (v, i) = id.split_once(':').unwrap_or(("0", "0"));
            Some(DstPrint {
                size: size as u64,
                mtime,
                volume: hex_u64(v) as u32,
                index: hex_u64(i),
            })
        }
        _ => None,
    };
    Ok(OpRow {
        id: r.get(0)?,
        batch_id: r.get(1)?,
        seq: r.get(2)?,
        kind: r.get(3)?,
        src: PathBuf::from(r.get::<_, String>(4)?),
        dst: PathBuf::from(r.get::<_, String>(5)?),
        fp,
        state: r.get(11)?,
        reason: r.get(12)?,
        dst_print,
    })
}

const OP_COLS: &str = "id,batch_id,seq,kind,src,dst,0,fp_size,fp_mtime,fp_file_id,fp_volume,state,reason,dst_size,dst_mtime,dst_file_id";

impl Journal {
    pub fn new(db: Arc<Db>) -> Self {
        Journal { db }
    }

    /// Records a batch and all its planned moves in one step, before anything moves.
    pub fn begin(
        &self,
        mode: &str,
        source: &Path,
        dest: &Path,
        moves: &[Move],
    ) -> Result<String, OrganizerError> {
        let batch = Uuid::new_v4().to_string();
        self.db
            .with(|c| {
                let tx = c.unchecked_transaction()?;
                tx.execute(
                    "INSERT INTO org_batches(id,mode,source_root,dest_root,created_at,state) VALUES(?1,?2,?3,?4,?5,'running')",
                    params![batch, mode, source.to_string_lossy(), dest.to_string_lossy(), now()],
                )?;
                for (i, m) in moves.iter().enumerate() {
                    tx.execute(
                        "INSERT INTO org_ops(id,batch_id,seq,kind,src,dst,fp_size,fp_mtime,fp_file_id,fp_volume,state,updated_at) \
                         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'planned',?11)",
                        params![
                            Uuid::new_v4().to_string(),
                            batch,
                            i as i64,
                            if m.cross_volume { "copy_delete" } else { "move" },
                            m.src.to_string_lossy(),
                            m.dst.to_string_lossy(),
                            m.fp.size as i64,
                            m.fp.mtime,
                            format!("{:x}", m.fp.index),
                            m.fp.volume as i64,
                            now()
                        ],
                    )?;
                }
                tx.commit()
            })
            .map_err(err)?;
        Ok(batch)
    }

    fn set_state(&self, op: &str, state: &str, reason: Option<&str>) -> Result<(), OrganizerError> {
        self.db
            .with(|c| {
                c.execute(
                    "UPDATE org_ops SET state=?2, reason=?3, updated_at=?4 WHERE id=?1",
                    params![op, state, reason, now()],
                )
            })
            .map(|_| ())
            .map_err(err)
    }

    pub fn op_in_progress(&self, op: &str) -> Result<(), OrganizerError> {
        self.set_state(op, "in_progress", None)
    }

    pub fn op_done(&self, op: &str, dst: DstPrint) -> Result<(), OrganizerError> {
        self.db
            .with(|c| {
                c.execute(
                    "UPDATE org_ops SET state='done', reason=NULL, dst_size=?2, dst_mtime=?3, dst_file_id=?4, updated_at=?5 WHERE id=?1",
                    params![op, dst.size as i64, dst.mtime, format!("{:x}:{:x}", dst.volume, dst.index), now()],
                )
            })
            .map(|_| ())
            .map_err(err)
    }

    pub fn op_failed(&self, op: &str, reason: &str) -> Result<(), OrganizerError> {
        self.set_state(op, "failed", Some(reason))
    }
    pub fn op_skipped(&self, op: &str, reason: &str) -> Result<(), OrganizerError> {
        self.set_state(op, "skipped", Some(reason))
    }
    pub fn op_undone(&self, op: &str) -> Result<(), OrganizerError> {
        self.set_state(op, "undone", None)
    }
    pub fn op_undo_skipped(&self, op: &str, reason: &str) -> Result<(), OrganizerError> {
        // The file stays where it is, so it is still a completed move as far as the journal goes.
        self.db
            .with(|c| {
                c.execute(
                    "UPDATE org_ops SET reason=?2, updated_at=?3 WHERE id=?1",
                    params![op, reason, now()],
                )
            })
            .map(|_| ())
            .map_err(err)
    }

    pub fn finish(&self, batch: &str, state: &str) -> Result<(), OrganizerError> {
        self.db
            .with(|c| {
                c.execute(
                    "UPDATE org_batches SET state=?2 WHERE id=?1",
                    params![batch, state],
                )
            })
            .map(|_| ())
            .map_err(err)
    }

    pub fn ops(&self, batch: &str) -> Result<Vec<OpRow>, OrganizerError> {
        self.db
            .with(|c| {
                let mut s = c.prepare(&format!(
                    "SELECT {OP_COLS} FROM org_ops WHERE batch_id=?1 ORDER BY seq"
                ))?;
                let rows = s
                    .query_map(params![batch], map_op)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .map_err(err)
    }

    /// Moves that were started and never recorded as finished: the app stopped in the middle.
    pub fn interrupted(&self) -> Result<Vec<OpRow>, OrganizerError> {
        self.db
            .with(|c| {
                let mut s = c.prepare(&format!("SELECT {OP_COLS} FROM org_ops WHERE state='in_progress' ORDER BY batch_id, seq"))?;
                let rows = s.query_map([], map_op)?.collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .map_err(err)
    }

    pub fn batches(&self, limit: u32) -> Result<Vec<BatchRow>, OrganizerError> {
        self.db
            .with(|c| {
                let mut s = c.prepare(
                    "SELECT b.id,b.mode,b.source_root,b.dest_root,b.created_at,b.state, \
                     COALESCE(SUM(o.state='done'),0), COALESCE(SUM(o.state='undone'),0), COALESCE(SUM(o.state='failed'),0) \
                     FROM org_batches b LEFT JOIN org_ops o ON o.batch_id=b.id GROUP BY b.id ORDER BY b.created_at DESC, b.rowid DESC LIMIT ?1",
                )?;
                let rows = s
                    .query_map(params![limit], |r| {
                        Ok(BatchRow {
                            id: r.get(0)?,
                            mode: r.get(1)?,
                            source_root: r.get(2)?,
                            dest_root: r.get(3)?,
                            created_at: r.get(4)?,
                            state: r.get(5)?,
                            moved: r.get(6)?,
                            undone: r.get(7)?,
                            failed: r.get(8)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .map_err(err)
    }

    /// True when a file at `path` is already the result of a move that is still undoable. Used so a
    /// file that was just organized is never planned again.
    pub fn was_moved_here(&self, path: &Path) -> bool {
        self.db
            .with(|c| {
                c.query_row(
                    "SELECT COUNT(*) FROM org_ops WHERE state='done' AND lower(dst)=lower(?1)",
                    params![path.to_string_lossy()],
                    |r| r.get::<_, i64>(0),
                )
            })
            .map(|n| n > 0)
            .unwrap_or(false)
    }
}
