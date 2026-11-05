//! Moving files. Each file is checked again immediately before it moves, written to the journal
//! before and after, and moved in a way that cannot replace anything.

use crate::journal::{DstPrint, Journal, OpRow};
use crate::model::{Fingerprint, OrganizerError, Plan};
use crate::pathutil::{has_traversal, is_inside};
use crate::winfs;
use nomnom_core::fs::move_no_replace;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub struct ExecOptions {
    /// The user agreed to copy files to another drive (the original is removed after a check).
    pub allow_cross_volume: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Report {
    pub batch_id: String,
    pub moved: usize,
    pub failed: Vec<Problem>,
    /// Set when the run stopped early and why (disk full, drive removed, or the user cancelled).
    pub stopped: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Problem {
    pub path: PathBuf,
    pub message: String,
}

struct Fail {
    message: String,
    /// The whole batch should stop (the disk is full, the drive went away...).
    systemic: bool,
}

fn fail(msg: impl Into<String>) -> Fail {
    Fail {
        message: msg.into(),
        systemic: false,
    }
}

fn from_io(what: &str, e: &io::Error) -> Fail {
    // disk full, not ready, device gone, I/O error, device not connected
    let systemic = matches!(e.raw_os_error(), Some(112 | 21 | 55 | 1117 | 1167 | 53));
    Fail {
        message: format!("{what}: {e}"),
        systemic,
    }
}

fn print_of(path: &Path) -> Result<DstPrint, Fail> {
    let i = winfs::info(path).map_err(|e| from_io("Couldn't check the moved file", &e))?;
    Ok(DstPrint {
        size: i.size,
        mtime: i.mtime,
        volume: i.id.volume,
        index: i.id.index,
    })
}

fn blake(path: &Path) -> io::Result<blake3::Hash> {
    let mut f = OpenOptions::new().read(true).open(path)?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize())
}

/// Moves `from` to `to`, never replacing anything. On the same drive this is a rename. Across
/// drives the file is copied beside its destination, compared byte for byte (hash), checked to be
/// unchanged, renamed into place, and only then is the original removed.
fn transfer(
    from: &Path,
    to: &Path,
    expected_size: u64,
    expected_mtime: i64,
    allow_cross: bool,
) -> Result<(), Fail> {
    let to_parent = to.parent().ok_or_else(|| fail("Bad destination."))?;
    let from_vol = winfs::info(from)
        .map_err(|e| from_io("Couldn't read the file", &e))?
        .id
        .volume;
    let to_vol = winfs::info(to_parent)
        .map_err(|e| from_io("Couldn't read the destination", &e))?
        .id
        .volume;

    if from_vol == to_vol {
        return move_no_replace(from, to).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                fail("A file with that name appeared in the destination.")
            } else {
                from_io("Couldn't move the file", &e)
            }
        });
    }
    if !allow_cross {
        return Err(fail(
            "This file is on another drive and you haven't allowed copying between drives.",
        ));
    }

    copy_verify_remove(from, to, expected_size, expected_mtime)
}

/// The cross-drive path: copy beside the destination, verify, move into place, then remove the
/// original. Public (but hidden) so it can be tested on one drive.
#[doc(hidden)]
pub fn copy_verify_remove_for_tests(
    from: &Path,
    to: &Path,
    size: u64,
    mtime: i64,
) -> Result<(), String> {
    copy_verify_remove(from, to, size, mtime).map_err(|f| f.message)
}

fn copy_verify_remove(
    from: &Path,
    to: &Path,
    expected_size: u64,
    expected_mtime: i64,
) -> Result<(), Fail> {
    let name = to
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = to.with_file_name(format!("{name}.nomnom-tmp"));
    let cleanup = |tmp: &Path| {
        let _ = std::fs::remove_file(tmp); // only ever our own temporary copy
    };
    let mut src = OpenOptions::new()
        .read(true)
        .open(from)
        .map_err(|e| from_io("Couldn't open the file", &e))?;
    // create_new: if anything is already called that, it is somebody's file and not ours. Stop here,
    // before the cleanup below could ever remove it.
    let mut dst = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| {
            from_io(
                "Couldn't create the temporary copy (a file with that name may already exist)",
                &e,
            )
        })?;
    let copied = (|| -> Result<(), Fail> {
        io::copy(&mut src, &mut dst).map_err(|e| from_io("Couldn't copy the file", &e))?;
        dst.flush()
            .map_err(|e| from_io("Couldn't finish the copy", &e))?;
        dst.sync_all()
            .map_err(|e| from_io("Couldn't finish the copy", &e))?;
        drop(dst);
        let (a, b) = (blake(from), blake(&tmp));
        match (a, b) {
            (Ok(a), Ok(b)) if a == b => {}
            _ => {
                return Err(fail(
                    "The copy didn't match the original, so nothing was changed.",
                ))
            }
        }
        let now = winfs::info(from).map_err(|_| fail("The original disappeared while copying."))?;
        if now.size != expected_size || now.mtime != expected_mtime {
            return Err(fail(
                "The file changed while it was being copied, so nothing was changed.",
            ));
        }
        Ok(())
    })();
    if let Err(f) = copied {
        cleanup(&tmp);
        return Err(f);
    }
    if let Err(e) = move_no_replace(&tmp, to) {
        cleanup(&tmp);
        return Err(if e.kind() == io::ErrorKind::AlreadyExists {
            fail("A file with that name appeared in the destination.")
        } else {
            from_io("Couldn't put the copy in place", &e)
        });
    }
    // The copy is safely in place and verified. A read-only original is never forced: Rust's
    // remove_file would delete it anyway on Windows, so ask first. Keep both and say so.
    let read_only = std::fs::metadata(from)
        .map(|m| m.permissions().readonly())
        .unwrap_or(false);
    if read_only {
        return Err(Fail {
            message: "The file was copied, but the original is read-only so it wasn't removed. Both copies were kept.".into(),
            systemic: false,
        });
    }
    std::fs::remove_file(from).map_err(|e| Fail {
        message: format!("The file was copied, but the original couldn't be removed ({e}). Both copies were kept."),
        systemic: false,
    })
}

fn ensure_plain_dir(dir: &Path) -> Result<(), Fail> {
    match winfs::info(dir) {
        Ok(i) if i.is_dir() && !i.is_reparse() => Ok(()),
        Ok(_) => Err(fail(format!("{} isn't a normal folder.", dir.display()))),
        Err(_) => std::fs::create_dir_all(dir)
            .map_err(|e| from_io("Couldn't create the folder", &e))
            .and_then(|_| match winfs::info(dir) {
                Ok(i) if i.is_dir() && !i.is_reparse() => Ok(()),
                _ => Err(fail("The new folder isn't a normal folder.")),
            }),
    }
}

/// The checks done right before one file moves. Anything that doesn't match the plan exactly
/// stops this file.
fn revalidate(plan: &Plan, op: &OpRow) -> Result<(), Fail> {
    if has_traversal(&op.src) || has_traversal(&op.dst) {
        return Err(fail("That path isn't allowed."));
    }
    if !is_inside(&op.src, &plan.source) {
        return Err(fail("That file isn't inside the folder being organized."));
    }
    let expected_parent = op.dst.parent().and_then(|p| p.parent());
    if !expected_parent.is_some_and(|p| crate::pathutil::is_same(p, &plan.dest_root)) {
        return Err(fail(
            "That destination isn't inside the destination folder.",
        ));
    }
    let now = winfs::info(&op.src).map_err(|_| fail("It's no longer there."))?;
    if now.is_dir() || now.is_reparse() || now.is_hidden_or_system() || now.is_cloud_or_offline() {
        return Err(fail("It has changed into something NomNom doesn't move."));
    }
    let fp = op.fp;
    if now.size != fp.size
        || now.mtime != fp.mtime
        || now.id.volume != fp.volume
        || now.id.index != fp.index
    {
        return Err(fail("It changed after the preview, so it was left alone."));
    }
    if winfs::is_locked(&op.src) {
        return Err(fail("Another program is using it."));
    }
    Ok(())
}

/// Runs a plan. `stop` is asked before every file.
pub fn execute(
    journal: &Journal,
    plan: &Plan,
    mode: &str,
    opts: &ExecOptions,
    stop: &dyn Fn() -> bool,
) -> Result<Report, OrganizerError> {
    let batch = journal.begin(mode, &plan.source, &plan.dest_root, &plan.moves)?;
    let ops = journal.ops(&batch)?;
    let mut report = Report {
        batch_id: batch.clone(),
        ..Default::default()
    };

    for op in &ops {
        if stop() {
            report.stopped = Some("Stopped before the remaining files were reached.".into());
            journal.op_skipped(&op.id, "Stopped before this file was reached.")?;
            continue;
        }
        if report.stopped.is_some() {
            journal.op_skipped(&op.id, "Not reached because the run stopped.")?;
            continue;
        }
        let result = (|| -> Result<DstPrint, Fail> {
            revalidate(plan, op)?;
            let category_dir = op.dst.parent().ok_or_else(|| fail("Bad destination."))?;
            ensure_plain_dir(&plan.dest_root)?;
            ensure_plain_dir(category_dir)?;
            if winfs::info(&op.dst).is_ok() {
                return Err(fail("A file with that name is already in the destination."));
            }
            journal
                .op_in_progress(&op.id)
                .map_err(|e| fail(e.to_string()))?;
            transfer(
                &op.src,
                &op.dst,
                op.fp.size,
                op.fp.mtime,
                opts.allow_cross_volume,
            )?;
            print_of(&op.dst)
        })();
        match result {
            Ok(print) => {
                journal.op_done(&op.id, print)?;
                report.moved += 1;
            }
            Err(f) => {
                journal.op_failed(&op.id, &f.message)?;
                report.failed.push(Problem {
                    path: op.src.clone(),
                    message: f.message.clone(),
                });
                if f.systemic {
                    report.stopped = Some(f.message);
                }
            }
        }
    }
    let state = if report.failed.is_empty() && report.stopped.is_none() {
        "done"
    } else {
        "partial"
    };
    journal.finish(&batch, state)?;
    Ok(report)
}

/// After a crash: works out what each interrupted move actually did and records it. Removes only
/// NomNom's own half-finished copies, never a user's file.
pub fn recover(journal: &Journal) -> Result<Vec<Problem>, OrganizerError> {
    let mut notes = Vec::new();
    for op in journal.interrupted()? {
        let src_here = winfs::info(&op.src).is_ok();
        let dst_info = winfs::info(&op.dst).ok();
        let name = op
            .dst
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tmp = op.dst.with_file_name(format!("{name}.nomnom-tmp"));
        let tmp_exists = winfs::info(&tmp).is_ok();
        if tmp_exists {
            let _ = std::fs::remove_file(&tmp);
        }
        match (src_here, dst_info) {
            (true, None) => journal.op_failed(
                &op.id,
                "NomNom stopped before this file moved. It is still where it was.",
            )?,
            (false, Some(d)) if d.size == op.fp.size => {
                journal.op_done(
                    &op.id,
                    DstPrint {
                        size: d.size,
                        mtime: d.mtime,
                        volume: d.id.volume,
                        index: d.id.index,
                    },
                )?;
            }
            (true, Some(_)) => {
                journal.op_failed(
                    &op.id,
                    "NomNom stopped while copying. Both copies exist and nothing was removed.",
                )?;
                notes.push(Problem {
                    path: op.src.clone(),
                    message: "Both the original and the copy exist. Nothing was removed.".into(),
                });
            }
            _ => {
                journal.op_failed(&op.id, "NomNom stopped while moving this file, and it can't be found where it should be.")?;
                notes.push(Problem {
                    path: op.src.clone(),
                    message: "The file can't be found at either place.".into(),
                });
            }
        }
        journal.finish(&op.batch_id, "partial")?;
    }
    Ok(notes)
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UndoReport {
    pub restored: usize,
    pub skipped: Vec<Problem>,
}

/// Puts files back, newest first. A file is only restored if it's exactly as NomNom left it and
/// nothing now occupies its old place. Anything else stays where it is, with the reason.
pub fn undo(
    journal: &Journal,
    batch: &str,
    opts: &ExecOptions,
) -> Result<UndoReport, OrganizerError> {
    let mut report = UndoReport::default();
    let mut ops = journal.ops(batch)?;
    ops.reverse();
    for op in ops.iter().filter(|o| o.state == "done") {
        let check = (|| -> Result<(), Fail> {
            let expected = op
                .dst_print
                .ok_or_else(|| fail("There's no record of where it went."))?;
            let now = winfs::info(&op.dst)
                .map_err(|_| fail("It's no longer in the destination folder."))?;
            if now.is_dir() || now.is_reparse() {
                return Err(fail("What's there now isn't the file NomNom moved."));
            }
            if now.size != expected.size || now.mtime != expected.mtime {
                return Err(fail(
                    "It was changed after it was moved, so it was left alone.",
                ));
            }
            if op.kind == "move"
                && (now.id.volume != expected.volume || now.id.index != expected.index)
            {
                return Err(fail(
                    "It was replaced after it was moved, so it was left alone.",
                ));
            }
            if winfs::info(&op.src).is_ok() {
                return Err(fail(
                    "Something with the original name is already in the original folder.",
                ));
            }
            let parent = op
                .src
                .parent()
                .ok_or_else(|| fail("Bad original location."))?;
            match winfs::info(parent) {
                Ok(i) if i.is_dir() && !i.is_reparse() => {}
                _ => return Err(fail("The original folder is gone.")),
            }
            if winfs::is_locked(&op.dst) {
                return Err(fail("Another program is using it."));
            }
            transfer(
                &op.dst,
                &op.src,
                expected.size,
                expected.mtime,
                opts.allow_cross_volume,
            )
        })();
        match check {
            Ok(()) => {
                journal.op_undone(&op.id)?;
                report.restored += 1;
            }
            Err(f) => {
                journal.op_undo_skipped(&op.id, &f.message)?;
                report.skipped.push(Problem {
                    path: op.dst.clone(),
                    message: f.message,
                });
            }
        }
    }
    let all_back = journal.ops(batch)?.iter().all(|o| o.state != "done");
    journal.finish(batch, if all_back { "undone" } else { "partly_undone" })?;
    Ok(report)
}

pub fn fingerprint_of(path: &Path) -> Option<Fingerprint> {
    let i = winfs::info(path).ok()?;
    Some(Fingerprint {
        size: i.size,
        mtime: i.mtime,
        volume: i.id.volume,
        index: i.id.index,
    })
}
