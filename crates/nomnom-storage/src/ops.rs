//! File operations. Nothing here trusts the scan: every operation is planned first (the plan
//! records what is on disk right now and refuses anything that changed since the scan), and run
//! later against the disk again, item by item. A deleted folder is never followed into a link.

use crate::index::{Index, NodeId, F_LINK, F_LOSSY_NAME};
use crate::protect::{valid_new_name, Protected, Verdict};
use crate::scan::long_os_path;
use nomnom_organizer::winfs::{self, FileId};
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
const ATTR_READONLY: u32 = 0x1;
#[cfg(windows)]
const ATTR_DIRECTORY: u32 = 0x10;
#[cfg(windows)]
const ATTR_REPARSE: u32 = 0x400;

pub(crate) fn is_link_metadata(md: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        md.file_attributes() & ATTR_REPARSE != 0
    }
    #[cfg(not(windows))]
    {
        md.file_type().is_symlink()
    }
}

fn is_dir_metadata(md: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        md.file_attributes() & ATTR_DIRECTORY != 0
    }
    #[cfg(not(windows))]
    {
        md.file_type().is_dir()
    }
}

fn is_readonly_metadata(md: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        md.file_attributes() & ATTR_READONLY != 0
    }
    #[cfg(not(windows))]
    {
        md.permissions().readonly()
    }
}

/// Past these a permanent delete asks for more than one click.
pub const BIG_FILES: u64 = 1000;
pub const BIG_BYTES: u64 = 10 * 1024 * 1024 * 1024;

#[derive(Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Recycle,
    Delete,
    Copy,
    Move,
    Rename,
    /// Many items, each with its own new name from a rule (see `batch`).
    BatchRename,
    /// Put the items in one ZIP file in the destination folder.
    Zip,
    /// Compress files or folders in place using NTFS compression (`FSCTL_SET_COMPRESSION`).
    Compress,
    /// Uncompress NTFS-compressed files or folders in place.
    Decompress,
    /// Replace duplicate copies with NTFS hardlinks to the kept file on the same drive.
    Hardlink,
}

/// What was on disk when the plan was made.
#[derive(Clone, Copy, Debug)]
pub struct Snap {
    pub id: FileId,
    pub size: u64,
    pub mtime: i64,
    pub is_dir: bool,
    pub is_reparse: bool,
}

/// For deleting a duplicate: the copy that stays, and which the item must still match.
#[derive(Clone, Debug)]
pub struct Keep {
    pub path: PathBuf,
    pub id: FileId,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub node: NodeId,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub files: u64,
    pub snap: Snap,
    pub warn: bool,
    pub keep: Option<Keep>,
    /// For a batch rename: what this item will be called.
    pub new_name: Option<String>,
}

/// Plans the removal of duplicate copies. Each group names the copy to keep. A copy is only
/// removed after its content is compared, byte for byte, with the one kept (checked again at run
/// time), and the kept copy can never be in the list.
pub fn plan_duplicates(
    idx: &Index,
    kind: Kind,
    groups: &[(NodeId, Vec<NodeId>)],
    protect: &Protected,
) -> Plan {
    assert!(matches!(
        kind,
        Kind::Recycle | Kind::Delete | Kind::Hardlink
    ));
    let removing: Vec<NodeId> = groups.iter().flat_map(|(_, r)| r.iter().copied()).collect();
    let mut plan = plan(idx, kind, &removing, None, None, protect);
    let mut kept_somewhere: Vec<NodeId> = Vec::new();
    for (keep, _) in groups {
        kept_somewhere.push(*keep);
    }
    let mut accepted = Vec::new();
    for item in std::mem::take(&mut plan.items) {
        let Some((keep, _)) = groups.iter().find(|(_, r)| r.contains(&item.node)) else {
            continue;
        };
        let why = if kept_somewhere.contains(&item.node) {
            Some("This copy is the one being kept in another group.")
        } else {
            match snap_of(&idx.path_of(*keep)) {
                Ok(k) if k.id == item.snap.id => Some("This is another name for the same file as the copy kept, so deleting it frees nothing."),
                Ok(k) if kind == Kind::Hardlink && k.id.volume != item.snap.id.volume => {
                    Some("Hardlinks only work when both files are on the same drive.")
                }
                Ok(k) if k.size != item.snap.size || k.is_dir || k.is_reparse => Some("It isn't the same size as the copy kept."),
                Ok(_) => None,
                Err(_) => Some("The copy to keep can't be found."),
            }
        };
        match why {
            Some(w) => plan.refused.push(Refusal {
                path: item.path.to_string_lossy().to_string(),
                reason: w.to_string(),
            }),
            None => accepted.push(item),
        }
    }
    plan.items = accepted;
    for item in &mut plan.items {
        let (keep, _) = groups.iter().find(|(_, r)| r.contains(&item.node)).unwrap();
        let keep_path = idx.path_of(*keep);
        if let Ok(k) = snap_of(&keep_path) {
            item.keep = Some(Keep {
                path: keep_path,
                id: k.id,
                size: k.size,
            });
        }
    }
    plan.total_size = plan.items.iter().map(|i| i.size).sum();
    plan.total_files = plan.items.len() as u64;
    plan.needs_typed = plan.kind == Kind::Delete
        && (plan.total_files > BIG_FILES
            || plan.total_size > BIG_BYTES
            || plan.items.iter().any(|i| i.warn));
    plan
}

/// True when two files hold exactly the same bytes.
pub fn verify_same(a: &Path, b: &Path) -> io::Result<bool> {
    let (mut fa, mut fb) = (File::open(long_os_path(a))?, File::open(long_os_path(b))?);
    if fa.metadata()?.len() != fb.metadata()?.len() {
        return Ok(false);
    }
    let (mut ba, mut bb) = (vec![0u8; 1 << 20], vec![0u8; 1 << 20]);
    loop {
        let na = read_full(&mut fa, &mut ba)?;
        let nb = read_full(&mut fb, &mut bb)?;
        if na != nb || ba[..na] != bb[..nb] {
            return Ok(false);
        }
        if na == 0 {
            return Ok(true);
        }
    }
}

fn read_full(f: &mut File, buf: &mut [u8]) -> io::Result<usize> {
    let mut got = 0;
    while got < buf.len() {
        match f.read(&mut buf[got..])? {
            0 => break,
            n => got += n,
        }
    }
    Ok(got)
}

#[derive(Serialize, Clone, Debug)]
pub struct Refusal {
    pub path: String,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub kind: Kind,
    pub items: Vec<Item>,
    pub dest: Option<PathBuf>,
    pub new_name: Option<String>,
    pub refused: Vec<Refusal>,
    pub warnings: Vec<String>,
    pub total_size: u64,
    pub total_files: u64,
    /// A permanent delete this big or this risky needs the count typed in.
    pub needs_typed: bool,
}

fn name_of(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string()
}

fn snap_of(path: &Path) -> io::Result<Snap> {
    let i = winfs::info(&long_os_path(path))?;
    Ok(Snap {
        id: i.id,
        size: i.size,
        mtime: i.mtime,
        is_dir: i.is_dir(),
        is_reparse: i.is_reparse(),
    })
}

/// Builds a plan from items picked in a scan. Nothing on disk is changed.
pub fn plan(
    idx: &Index,
    kind: Kind,
    nodes: &[NodeId],
    dest: Option<&Path>,
    new_name: Option<&str>,
    protect: &Protected,
) -> Plan {
    let mut p = Plan {
        kind,
        items: Vec::new(),
        dest: dest.map(Path::to_path_buf),
        new_name: new_name.map(str::to_string),
        refused: Vec::new(),
        warnings: Vec::new(),
        total_size: 0,
        total_files: 0,
        needs_typed: false,
    };
    let mut chosen: Vec<(NodeId, PathBuf)> = Vec::new();
    for &n in nodes {
        match idx.node(n) {
            Some(_) => chosen.push((n, idx.path_of(n))),
            None => p.refused.push(Refusal {
                path: format!("#{n}"),
                reason: "That item isn't in the scan.".into(),
            }),
        }
    }
    chosen.sort_by_key(|(_, path)| path.components().count());
    chosen.dedup_by(|a, b| a.0 == b.0);
    let mut kept: Vec<(NodeId, PathBuf)> = Vec::new();
    for (n, path) in chosen {
        // A folder and something inside it: the folder already covers it.
        if kept
            .iter()
            .any(|(_, k)| nomnom_organizer::pathutil::is_inside(&path, k))
        {
            continue;
        }
        kept.push((n, path));
    }

    for (n, path) in kept {
        let node = idx.node(n).unwrap();
        let shown = path.to_string_lossy().to_string();
        let refuse = |p: &mut Plan, why: &str| {
            p.refused.push(Refusal {
                path: shown.clone(),
                reason: why.to_string(),
            })
        };
        if node.flags & F_LOSSY_NAME != 0
            || idx
                .ancestors(n)
                .iter()
                .any(|a| idx.node(*a).unwrap().flags & F_LOSSY_NAME != 0)
        {
            refuse(
                &mut p,
                "The name has characters Windows can't write back safely.",
            );
            continue;
        }
        let mut warn = false;
        match protect.check(&path) {
            Verdict::Refuse(why) => {
                refuse(&mut p, &why);
                continue;
            }
            Verdict::Warn(why) => {
                warn = true;
                p.warnings.push(format!("{}: {why}", name_of(&path)));
            }
            Verdict::Ok => {}
        }
        let snap = match snap_of(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                refuse(&mut p, "It no longer exists. Refresh the folder.");
                continue;
            }
            Err(e) => {
                refuse(&mut p, &format!("It can't be read: {e}"));
                continue;
            }
        };
        let was_dir = node.is_dir();
        let stale = snap.is_dir != was_dir
            || snap.is_reparse != (node.flags & F_LINK != 0 || node.attrs & 0x400 != 0)
            || (!was_dir && (snap.size != node.size || snap.mtime != node.mtime));
        if stale {
            refuse(
                &mut p,
                "It changed after the scan. Refresh the folder and try again.",
            );
            continue;
        }
        if kind == Kind::Recycle {
            if let Err(why) = recycle_ok(&path, node.size) {
                refuse(&mut p, &why);
                continue;
            }
        }
        p.total_size += node.size;
        p.total_files += if was_dir { u64::from(node.files) } else { 1 };
        p.items.push(Item {
            node: n,
            path,
            is_dir: was_dir,
            size: node.size,
            files: if was_dir { u64::from(node.files) } else { 1 },
            snap,
            warn,
            keep: None,
            new_name: None,
        });
    }

    match kind {
        Kind::Copy | Kind::Move | Kind::Zip => match dest {
            None => p.refused.push(Refusal {
                path: String::new(),
                reason: "Choose a destination folder.".into(),
            }),
            Some(d) => check_destination(&mut p, d, protect),
        },
        Kind::Rename => {
            if p.items.len() != 1 {
                p.items.clear();
                p.refused.push(Refusal {
                    path: String::new(),
                    reason: "Rename works on one item at a time.".into(),
                });
            } else if let Err(why) =
                new_name.map_or(Err("Type a new name.".to_string()), valid_new_name)
            {
                p.items.clear();
                p.refused.push(Refusal {
                    path: String::new(),
                    reason: why,
                });
            }
        }
        _ => {}
    }
    if kind == Kind::Delete {
        p.needs_typed =
            p.total_files > BIG_FILES || p.total_size > BIG_BYTES || p.items.iter().any(|i| i.warn);
    }
    p
}

fn check_destination(p: &mut Plan, d: &Path, protect: &Protected) {
    let bad = |p: &mut Plan, why: String| {
        p.items.clear();
        p.refused.push(Refusal {
            path: d.to_string_lossy().to_string(),
            reason: why,
        });
    };
    if !d.is_dir() {
        return bad(p, "The destination folder doesn't exist.".into());
    }
    match protect.check(d) {
        Verdict::Refuse(why) => return bad(p, format!("Can't put files there. {why}")),
        Verdict::Warn(why) => p.warnings.push(format!("Destination: {why}")),
        Verdict::Ok => {}
    }
    if let Some(it) = p
        .items
        .iter()
        .find(|i| i.is_dir && nomnom_organizer::pathutil::is_inside(d, &i.path))
    {
        let why = format!("{} can't be put inside itself.", name_of(&it.path));
        bad(p, why);
    }
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Done,
    Failed,
    Skipped,
}

#[derive(Serialize, Clone, Debug)]
pub struct ItemResult {
    pub path: String,
    pub status: Status,
    pub message: Option<String>,
    pub size: u64,
    /// Where it went (copy, move, rename).
    pub to: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Report {
    pub results: Vec<ItemResult>,
    pub cancelled: bool,
}

impl Report {
    pub fn count(&self, s: Status) -> usize {
        self.results.iter().filter(|r| r.status == s).count()
    }
    pub fn bytes_done(&self) -> u64 {
        self.results
            .iter()
            .filter(|r| r.status == Status::Done)
            .map(|r| r.size)
            .sum()
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub current: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RunOptions {
    /// Also delete files marked read-only (they are reported, not deleted, otherwise).
    pub include_readonly: bool,
}

/// Runs a plan. Each item is checked against the disk again right before it is touched.
pub fn run(
    plan: &Plan,
    opts: RunOptions,
    cancel: &AtomicBool,
    progress: &dyn Fn(Progress),
) -> Report {
    if plan.kind == Kind::Zip {
        return crate::batch::run_zip(plan, cancel, progress);
    }
    let mut report = Report::default();
    let total = plan.items.len();
    for (i, item) in plan.items.iter().enumerate() {
        progress(Progress {
            done: i,
            total,
            current: name_of(&item.path),
        });
        if cancel.load(Ordering::SeqCst) {
            report.cancelled = true;
            for rest in &plan.items[i..] {
                report.results.push(result(
                    rest,
                    Status::Skipped,
                    Some("Stopped before this item.".into()),
                    None,
                ));
            }
            break;
        }
        if let Some(keep) = &item.keep {
            // The copy that stays must still be there and unchanged, and identical to this one.
            let kept_ok = snap_of(&keep.path)
                .map(|k| k.id == keep.id && k.size == keep.size && !k.is_reparse)
                .unwrap_or(false);
            if !kept_ok {
                report.results.push(result(
                    item,
                    Status::Skipped,
                    Some(
                        "The copy that was to be kept is gone or changed, so nothing was deleted."
                            .into(),
                    ),
                    None,
                ));
                continue;
            }
            if !verify_same(&keep.path, &item.path).unwrap_or(false) {
                report.results.push(result(
                    item,
                    Status::Skipped,
                    Some(
                        "This file is not identical to the copy kept, so it was left alone.".into(),
                    ),
                    None,
                ));
                continue;
            }
        }
        if let Err(why) = revalidate(item) {
            report
                .results
                .push(result(item, Status::Skipped, Some(why), None));
            continue;
        }
        let outcome = match plan.kind {
            Kind::Recycle => recycle(&item.path).map(|_| None),
            Kind::Delete => delete_item(item, opts, cancel).map(|_| None),
            Kind::Copy => copy_item(item, plan.dest.as_deref().unwrap(), cancel).map(Some),
            Kind::Move => move_item(item, plan.dest.as_deref().unwrap(), cancel),
            Kind::Rename => rename_item(item, plan.new_name.as_deref().unwrap()).map(Some),
            Kind::BatchRename => match item.new_name.as_deref() {
                Some(n) => rename_item(item, n).map(Some),
                None => Err(skip("It has no new name.")),
            },
            Kind::Compress => compress_item(item, true, cancel).map(|_| None),
            Kind::Decompress => compress_item(item, false, cancel).map(|_| None),
            Kind::Hardlink => match item.keep.as_ref() {
                Some(keep) => crate::ntfs::replace_with_hardlink(&keep.path, &item.path)
                    .map(|_| Some(keep.path.clone())),
                None => Err(skip("It has no kept file to link to.")),
            },
            // Zipping is done as a whole before this loop.
            Kind::Zip => Err(skip("Handled with the archive.")),
        };
        report.results.push(match outcome {
            Ok(to) => result(item, Status::Done, None, to),
            Err(e) if e.kind() == io::ErrorKind::Other && e.to_string().starts_with("skip:") => {
                result(
                    item,
                    Status::Skipped,
                    Some(e.to_string().trim_start_matches("skip:").trim().to_string()),
                    None,
                )
            }
            Err(e) => result(item, Status::Failed, Some(explain(&e)), None),
        });
    }
    progress(Progress {
        done: report.results.len(),
        total,
        current: String::new(),
    });
    report
}

fn result(item: &Item, status: Status, message: Option<String>, to: Option<PathBuf>) -> ItemResult {
    ItemResult {
        path: item.path.to_string_lossy().to_string(),
        status,
        message,
        size: item.size,
        to: to.map(|t| t.to_string_lossy().to_string()),
    }
}

#[cfg(windows)]
fn explain(e: &io::Error) -> String {
    match e.raw_os_error() {
        Some(5) => "Access denied. NomNom doesn't take over permissions.".into(),
        Some(32) => "Another program is using it.".into(),
        Some(112) => "The disk is full.".into(),
        Some(145) => "The folder isn't empty.".into(),
        Some(3) | Some(2) => "It was already gone.".into(),
        _ => e.to_string(),
    }
}

#[cfg(unix)]
fn explain(e: &io::Error) -> String {
    match e.raw_os_error() {
        Some(c) if c == libc::EACCES || c == libc::EPERM => {
            "Permission denied. NomNom doesn't override file permissions.".into()
        }
        Some(c) if c == libc::EBUSY => "The item or mount is busy.".into(),
        Some(c) if c == libc::ENOSPC => "The disk is full.".into(),
        Some(c) if c == libc::ENOTEMPTY => "The folder isn't empty.".into(),
        Some(c) if c == libc::ENOENT => "It was already gone.".into(),
        _ => e.to_string(),
    }
}

fn skip(why: &str) -> io::Error {
    io::Error::other(format!("skip:{why}"))
}

/// The disk must still show what the plan saw. A scan is a photograph, never an authority.
fn revalidate(item: &Item) -> Result<(), String> {
    let now = snap_of(&item.path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            "It was already gone.".to_string()
        } else {
            format!("It can't be read now: {e}")
        }
    })?;
    let s = item.snap;
    if now.id != s.id || now.is_dir != s.is_dir || now.is_reparse != s.is_reparse {
        return Err("A different file or folder is at this path now.".into());
    }
    if !s.is_dir && (now.size != s.size || now.mtime != s.mtime) {
        return Err("The file changed after the preview.".into());
    }
    Ok(())
}

// ---- delete ------------------------------------------------------------------------------

#[derive(Default)]
struct Failures {
    count: usize,
    first: Option<String>,
}

fn delete_item(item: &Item, opts: RunOptions, cancel: &AtomicBool) -> io::Result<()> {
    let mut f = Failures::default();
    delete_tree(&item.path, opts.include_readonly, cancel, &mut f);
    match f.first {
        None => Ok(()),
        Some(first) => Err(io::Error::other(format!(
            "{} {} couldn't be deleted. First problem: {first}",
            f.count,
            if f.count == 1 { "item" } else { "items" }
        ))),
    }
}

/// Deletes a file or folder. A link (junction or symlink) is removed itself and never entered.
fn delete_tree(path: &Path, include_readonly: bool, cancel: &AtomicBool, f: &mut Failures) {
    let long = long_os_path(path);
    let md = match std::fs::symlink_metadata(&long) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return,
        Err(e) => return fail(f, path, &e),
    };
    if is_link_metadata(&md) {
        #[cfg(windows)]
        let r = if is_dir_metadata(&md) {
            std::fs::remove_dir(&long)
        } else {
            std::fs::remove_file(&long)
        };
        #[cfg(not(windows))]
        let r = std::fs::remove_file(&long);
        if let Err(e) = r {
            fail(f, path, &e);
        }
    } else if is_dir_metadata(&md) {
        match std::fs::read_dir(&long) {
            Ok(rd) => {
                for entry in rd.flatten() {
                    if cancel.load(Ordering::SeqCst) {
                        return fail_msg(f, path, "Stopped before everything was deleted.");
                    }
                    delete_tree(&path.join(entry.file_name()), include_readonly, cancel, f);
                }
            }
            Err(e) => return fail(f, path, &e),
        }
        if let Err(e) = std::fs::remove_dir(&long) {
            // Something inside couldn't be deleted, or appeared meanwhile.
            if f.count == 0 {
                fail(f, path, &e);
            }
        }
    } else {
        if is_readonly_metadata(&md) {
            if !include_readonly {
                return fail_msg(
                    f,
                    path,
                    "Read-only file. Tick \"also delete read-only files\" to remove it.",
                );
            }
            let mut perm = md.permissions();
            #[cfg(windows)]
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                perm.set_mode(perm.mode() | 0o200);
            }
            if let Err(e) = std::fs::set_permissions(&long, perm) {
                return fail(f, path, &e);
            }
        }
        if let Err(e) = std::fs::remove_file(&long) {
            fail(f, path, &e);
        }
    }
}

fn fail(f: &mut Failures, path: &Path, e: &io::Error) {
    fail_msg(f, path, &format!("{} ({})", explain(e), name_of(path)));
}
fn fail_msg(f: &mut Failures, path: &Path, msg: &str) {
    let _ = path;
    f.count += 1;
    f.first.get_or_insert_with(|| msg.to_string());
}

// ---- recycle ---------------------------------------------------------------------------

/// Checks that Windows can really keep this in the Recycle Bin. When it can't, Windows may delete
/// the item for good without asking, so NomNom refuses instead.
#[cfg(windows)]
pub fn recycle_ok(path: &Path, size: u64) -> Result<(), String> {
    let drive = crate::drives::drive_of(path)
        .ok_or("The Recycle Bin only works for items on a local drive.")?;
    if drive.kind != crate::drives::DriveKind::Fixed {
        return Err(
            "This drive has no Recycle Bin. Use Shift+Delete to delete it permanently.".into(),
        );
    }
    if path.to_string_lossy().len() >= 255 {
        return Err(
            "The path is too long for the Recycle Bin. Use Shift+Delete to delete it permanently."
                .into(),
        );
    }
    let mut limit = drive.total / 20;
    if let Some((nuke, max_mb)) = bitbucket_settings(&drive.root) {
        if nuke {
            return Err(
                "Windows is set to delete files on this drive without using the Recycle Bin."
                    .into(),
            );
        }
        if let Some(mb) = max_mb {
            limit = limit.min(mb * 1024 * 1024);
        }
    }
    if drive.total > 0 && size > limit {
        return Err(
            "Too big for the Recycle Bin. Use Shift+Delete if you want it gone for good.".into(),
        );
    }
    Ok(())
}

#[cfg(windows)]
fn bitbucket_settings(root: &str) -> Option<(bool, Option<u64>)> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetVolumeNameForVolumeMountPointW;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    let wroot: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buf = [0u16; 64];
    unsafe { GetVolumeNameForVolumeMountPointW(PCWSTR(wroot.as_ptr()), &mut buf) }.ok()?;
    let name = String::from_utf16_lossy(&buf[..buf.iter().position(|c| *c == 0)?]);
    let guid = name.split(['{', '}']).nth(1)?;
    let key = winreg::RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            format!(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\BitBucket\Volume\{{{guid}}}"
            ),
            KEY_READ,
        )
        .ok()?;
    let nuke: u32 = key.get_value("NukeOnDelete").unwrap_or(0);
    let max: Option<u32> = key.get_value("MaxCapacity").ok();
    Some((nuke == 1, max.map(u64::from)))
}

/// Sends one item to the Recycle Bin through the Windows shell.
#[cfg(windows)]
pub fn recycle(path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOCONFIRMMKDIR, FOF_NOERRORUI,
        FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW,
    };
    let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
    from.push(0);
    from.push(0);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO
            | FOF_NOCONFIRMATION
            | FOF_NOCONFIRMMKDIR
            | FOF_NOERRORUI
            | FOF_SILENT)
            .0 as u16,
        ..Default::default()
    };
    let code = unsafe { SHFileOperationW(&mut op) };
    if code != 0 || op.fAnyOperationsAborted.as_bool() {
        return Err(io::Error::other(format!(
            "Windows couldn't move it to the Recycle Bin (code {code:#x})."
        )));
    }
    if path.exists() {
        return Err(io::Error::other("Windows left the item where it was."));
    }
    Ok(())
}

#[cfg(unix)]
fn linux_home_trash() -> Option<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|h| h.join(".local").join("share"))
        })?;
    Some(data_home.join("Trash"))
}

#[cfg(unix)]
fn dev_of_existing_ancestor(p: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    p.ancestors()
        .find_map(|a| std::fs::symlink_metadata(a).ok().map(|m| m.dev()))
}

#[cfg(unix)]
fn resolve_linux_trash(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path.starts_with("/proc/")
        || path.starts_with("/sys/")
        || path.starts_with("/dev/")
    {
        return Err("Trash only works for items on a local filesystem.".into());
    }
    let drive =
        crate::drives::drive_of(path).ok_or("Trash only works for items on a local filesystem.")?;
    if matches!(
        drive.kind,
        crate::drives::DriveKind::Network | crate::drives::DriveKind::Optical
    ) {
        return Err("This mount has no Trash. Use Shift+Delete to delete it permanently.".into());
    }
    let item_dev =
        dev_of_existing_ancestor(path).ok_or("Couldn't find which filesystem holds this item.")?;
    if let Some(home_trash) = linux_home_trash() {
        if dev_of_existing_ancestor(&home_trash) == Some(item_dev) {
            return Ok(home_trash);
        }
    }
    let uid = unsafe { libc::geteuid() };
    let top_trash = Path::new(&drive.root).join(format!(".Trash-{uid}"));
    if nomnom_organizer::pathutil::is_inside(path, &top_trash) {
        return Err("This item is already inside Trash.".into());
    }
    Ok(top_trash)
}

#[cfg(unix)]
pub fn recycle_ok(path: &Path, size: u64) -> Result<(), String> {
    let trash_dir = resolve_linux_trash(path)?;
    let drive =
        crate::drives::drive_of(path).ok_or("Trash only works for items on a local filesystem.")?;
    if drive.total > 0 && size > drive.total {
        return Err("Too big for Trash. Use Shift+Delete if you want it gone for good.".into());
    }
    if nomnom_organizer::pathutil::is_inside(path, &trash_dir) {
        return Err("This item is already inside Trash.".into());
    }
    Ok(())
}

/// Moves `path` to the freedesktop.org Trash (`$XDG_DATA_HOME/Trash` or `<mount>/.Trash-<uid>`).
#[cfg(unix)]
pub fn recycle(path: &Path) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::DirBuilderExt;

    let trash_dir = resolve_linux_trash(path).map_err(io::Error::other)?;
    let files_dir = trash_dir.join("files");
    let info_dir = trash_dir.join("info");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&files_dir)?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&info_dir)?;

    let raw_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| io::Error::other("Item has no file name for Trash."))?;
    let (stem, ext) = match raw_name.rfind('.') {
        Some(i) if i > 0 => (&raw_name[..i], &raw_name[i..]),
        _ => (raw_name, ""),
    };

    let encoded_path = percent_encode_trash_path(path.as_os_str().as_bytes());
    let deletion_date = local_iso8601_now();
    let info_body = format!("[Trash Info]\nPath={encoded_path}\nDeletionDate={deletion_date}\n");

    for attempt in 1..=9999u32 {
        let chosen = if attempt == 1 {
            raw_name.to_string()
        } else {
            format!("{stem} ({attempt}){ext}")
        };
        let target_file = files_dir.join(&chosen);
        if std::fs::symlink_metadata(&target_file).is_ok() {
            continue;
        }
        let info_path = info_dir.join(format!("{chosen}.trashinfo"));
        let mut info_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&info_path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        if let Err(e) = info_file
            .write_all(info_body.as_bytes())
            .and_then(|_| info_file.flush())
        {
            drop(info_file);
            let _ = std::fs::remove_file(&info_path);
            return Err(e);
        }
        drop(info_file);

        match nomnom_core::fs::move_no_replace(path, &target_file) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let _ = std::fs::remove_file(&info_path);
                continue;
            }
            Err(e) => {
                let _ = std::fs::remove_file(&info_path);
                return Err(e);
            }
        }
    }
    Err(io::Error::other(
        "Trash already has too many items with this name.",
    ))
}

#[cfg(unix)]
fn percent_encode_trash_path(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 16);
    for &b in bytes {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(unix)]
fn local_iso8601_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as libc::time_t)
        .unwrap_or(0);
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: `secs` and `tm` are valid local stack values for reentrant `localtime_r`.
    if unsafe { !libc::localtime_r(&secs, &mut tm).is_null() } {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    } else {
        "1970-01-01T00:00:00".to_string()
    }
}

// ---- copy and move ---------------------------------------------------------------------

/// `name`, or `name (2)`, `name (3)` … so nothing is ever replaced.
pub fn free_name(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !long_os_path(&first).exists() {
        return first;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..10_000)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !long_os_path(p).exists())
        .unwrap_or_else(|| dir.join(format!("{stem} ({}){ext}", uuid_like())))
}

fn uuid_like() -> String {
    format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn copy_item(item: &Item, dest: &Path, cancel: &AtomicBool) -> io::Result<PathBuf> {
    let target = free_name(dest, &name_of(&item.path));
    let mut skipped = Vec::new();
    copy_tree(&item.path, &target, cancel, &mut skipped)?;
    Ok(target)
}

fn move_item(item: &Item, dest: &Path, cancel: &AtomicBool) -> io::Result<Option<PathBuf>> {
    let same_dir = item
        .path
        .parent()
        .is_some_and(|p| nomnom_organizer::pathutil::is_same(p, dest));
    if same_dir {
        return Err(skip("It is already in that folder."));
    }
    let target = free_name(dest, &name_of(&item.path));
    let same_volume = winfs::info(&long_os_path(dest))
        .map(|d| d.id.volume == item.snap.id.volume)
        .unwrap_or(false);
    if same_volume {
        nomnom_core::fs::move_no_replace(&long_os_path(&item.path), &long_os_path(&target))?;
        return Ok(Some(target));
    }
    move_by_copy(item, &target, cancel)?;
    Ok(Some(target))
}

/// Another drive: copy, prove the copy is identical, and only then remove the original.
pub fn move_by_copy(item: &Item, target: &Path, cancel: &AtomicBool) -> io::Result<()> {
    let mut skipped = Vec::new();
    if let Err(e) = copy_tree(&item.path, target, cancel, &mut skipped) {
        let mut f = Failures::default();
        // Remove only the half-made copy.
        delete_tree(target, true, &AtomicBool::new(false), &mut f);
        return Err(e);
    }
    let verified = if skipped.is_empty() {
        verify_tree(&item.path, target)
    } else {
        Err(io::Error::other(format!(
            "{} links inside weren't copied, so the original was kept.",
            skipped.len()
        )))
    };
    if let Err(e) = verified {
        return Err(io::Error::other(format!(
            "The copy wasn't identical, so the original was kept ({e}). The copy is at {}.",
            target.display()
        )));
    }
    revalidate(item).map_err(|why| {
        io::Error::other(format!(
            "The original changed while copying, so it was kept ({why}). The copy is at {}.",
            target.display()
        ))
    })?;
    let mut f = Failures::default();
    delete_tree(&item.path, false, cancel, &mut f);
    match f.first {
        None => Ok(()),
        Some(first) => Err(io::Error::other(format!(
            "Copied, but the original couldn't be fully removed: {first}"
        ))),
    }
}

/// Copies a file or folder to a name that doesn't exist. Links inside a folder are listed in
/// `skipped` and not followed.
pub fn copy_tree(
    src: &Path,
    dst: &Path,
    cancel: &AtomicBool,
    skipped: &mut Vec<PathBuf>,
) -> io::Result<()> {
    if cancel.load(Ordering::SeqCst) {
        return Err(io::Error::other("Stopped."));
    }
    let (ls, ld) = (long_os_path(src), long_os_path(dst));
    let md = std::fs::symlink_metadata(&ls)?;
    if is_link_metadata(&md) {
        skipped.push(src.to_path_buf());
        return Ok(());
    }
    if is_dir_metadata(&md) {
        std::fs::create_dir(&ld)?;
        for e in std::fs::read_dir(&ls)? {
            let e = e?;
            copy_tree(
                &src.join(e.file_name()),
                &dst.join(e.file_name()),
                cancel,
                skipped,
            )?;
        }
        return Ok(());
    }
    let mut input = File::open(&ls)?;
    let mut out = OpenOptions::new().write(true).create_new(true).open(&ld)?;
    io::copy(&mut input, &mut out)?;
    out.flush()?;
    if let Ok(m) = md.modified() {
        let _ = out.set_modified(m);
    }
    Ok(())
}

fn hash_file(p: &Path) -> io::Result<[u8; 32]> {
    let mut f = File::open(long_os_path(p))?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(*h.finalize().as_bytes())
}

/// Same names, same sizes and the same content on both sides.
pub fn verify_tree(a: &Path, b: &Path) -> io::Result<()> {
    let (la, lb) = (long_os_path(a), long_os_path(b));
    let ma = std::fs::symlink_metadata(&la)?;
    let mb = std::fs::symlink_metadata(&lb)?;
    if ma.is_dir() != mb.is_dir() {
        return Err(io::Error::other(format!(
            "{} isn't the same kind of item",
            name_of(a)
        )));
    }
    if ma.is_dir() {
        let mut names: Vec<_> = std::fs::read_dir(&la)?
            .flatten()
            .map(|e| e.file_name())
            .collect();
        names.sort();
        let mut other: Vec<_> = std::fs::read_dir(&lb)?
            .flatten()
            .map(|e| e.file_name())
            .collect();
        other.sort();
        if names != other {
            return Err(io::Error::other(format!(
                "{} has different contents",
                name_of(a)
            )));
        }
        for n in names {
            verify_tree(&a.join(&n), &b.join(&n))?;
        }
        return Ok(());
    }
    if ma.len() != mb.len() || hash_file(a)? != hash_file(b)? {
        return Err(io::Error::other(format!("{} differs", name_of(a))));
    }
    Ok(())
}

fn rename_item(item: &Item, new_name: &str) -> io::Result<PathBuf> {
    let parent = item
        .path
        .parent()
        .ok_or_else(|| io::Error::other("It has no folder."))?;
    let target = parent.join(new_name);
    let same_item_different_case = long_os_path(&target).exists()
        && snap_of(&target)
            .map(|s| s.id == item.snap.id)
            .unwrap_or(false);
    if long_os_path(&target).exists() && !same_item_different_case {
        return Err(io::Error::other(
            "Something with that name is already in this folder.",
        ));
    }
    if same_item_different_case {
        // A change of letter case only: Windows treats both names as one, so go through a temporary name.
        let tmp = parent.join(format!(".nomnom-rename-{}", uuid_like()));
        nomnom_core::fs::move_no_replace(&long_os_path(&item.path), &long_os_path(&tmp))?;
        if let Err(e) =
            nomnom_core::fs::move_no_replace(&long_os_path(&tmp), &long_os_path(&target))
        {
            let _ =
                nomnom_core::fs::move_no_replace(&long_os_path(&tmp), &long_os_path(&item.path));
            return Err(e);
        }
    } else {
        nomnom_core::fs::move_no_replace(&long_os_path(&item.path), &long_os_path(&target))?;
    }
    Ok(target)
}

// ---- NTFS compression ----------------------------------------------------------------------

fn compress_item(item: &Item, compress: bool, cancel: &AtomicBool) -> io::Result<()> {
    let mut f = Failures::default();
    compress_tree(&item.path, compress, cancel, &mut f);
    match f.first {
        None => Ok(()),
        Some(first) => Err(io::Error::other(format!(
            "{} {} couldn't be {}. First problem: {first}",
            f.count,
            if f.count == 1 { "item" } else { "items" },
            if compress {
                "compressed"
            } else {
                "uncompressed"
            }
        ))),
    }
}

fn compress_tree(path: &Path, compress: bool, cancel: &AtomicBool, f: &mut Failures) {
    if cancel.load(Ordering::SeqCst) {
        return fail_msg(f, path, "Stopped before everything was processed.");
    }
    let long = long_os_path(path);
    let md = match std::fs::symlink_metadata(&long) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return,
        Err(e) => return fail(f, path, &e),
    };
    if is_link_metadata(&md) {
        return; // never follow junctions or symlinks
    }
    if let Err(e) = crate::ntfs::set_ntfs_compression(path, compress) {
        fail(f, path, &e);
        return;
    }
    if is_dir_metadata(&md) {
        match std::fs::read_dir(&long) {
            Ok(rd) => {
                for entry in rd.flatten() {
                    if cancel.load(Ordering::SeqCst) {
                        return fail_msg(f, path, "Stopped before everything was processed.");
                    }
                    compress_tree(&path.join(entry.file_name()), compress, cancel, f);
                }
            }
            Err(e) => fail(f, path, &e),
        }
    }
}
