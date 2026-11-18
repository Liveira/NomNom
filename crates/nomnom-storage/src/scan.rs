//! The scanner. It reads each folder with one `FindFirstFileExW` pass (names, sizes, times and
//! attributes arrive together, no file is opened for basic metadata) and never enters a junction,
//! symlink or mount point. Several workers walk different folders at once; each folder's result is
//! added to the index in one short step, so the window can browse a partial result while scanning.

use crate::index::*;
use crate::ntfs::{self, OwnerCache};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, HANDLE};
use windows::Win32::Storage::FileSystem::{
    FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    GetCompressedFileSizeW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FIND_FIRST_EX_LARGE_FETCH,
    WIN32_FIND_DATAW,
};

const ATTR_SPARSE: u32 = 0x200;
const ATTR_COMPRESSED: u32 = 0x800;
const ATTR_OFFLINE: u32 = 0x1000;
const ATTR_RECALL_ON_OPEN: u32 = 0x4_0000;
const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
const MAX_ERRORS_KEPT: usize = 2000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanOptions {
    pub workers: usize,
    /// Files at least this big are checked for hardlinks (it costs one open each).
    pub hardlink_min: u64,
    pub cluster: u32,
    pub volume_serial: u32,
    /// Look up NTFS file/folder owners (cached by raw SID so account lookup runs once per user).
    pub collect_owners: bool,
    /// Check files for NTFS alternate data streams (`FindFirstStreamW`).
    pub collect_streams: bool,
    /// Entry names or wildcards (`*.tmp`, `node_modules`) to leave out of the scan.
    pub exclude: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            workers: 4,
            hardlink_min: 1 << 20,
            cluster: 4096,
            volume_serial: 0,
            collect_owners: true,
            collect_streams: false,
            exclude: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Running,
    Paused,
    Done,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanError {
    pub path: String,
    pub code: u32,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct SavedScan {
    pub index: IndexArchive,
    pub errors: Vec<ScanError>,
    pub elapsed_ms: u64,
    pub elevated: bool,
    pub mft_bytes: u64,
}

#[derive(Default)]
pub struct Stats {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub errors: AtomicU64,
}

enum ExcludeRule {
    Exact(String),
    Pattern(Regex),
}

impl ExcludeRule {
    fn matches(&self, name: &str) -> bool {
        match self {
            ExcludeRule::Exact(s) => name.eq_ignore_ascii_case(s),
            ExcludeRule::Pattern(re) => re.is_match(name),
        }
    }
}

fn compile_excludes(patterns: &[String]) -> Vec<ExcludeRule> {
    let mut out = Vec::new();
    for raw in patterns {
        let p = raw.trim();
        if p.is_empty() {
            continue;
        }
        if p.contains('*') || p.contains('?') {
            let mut re = String::from("^");
            for ch in p.chars() {
                match ch {
                    '*' => re.push_str(".*"),
                    '?' => re.push('.'),
                    c => re.push_str(&regex::escape(&c.to_string())),
                }
            }
            re.push('$');
            if let Ok(built) = RegexBuilder::new(&re)
                .case_insensitive(true)
                .size_limit(1 << 16)
                .build()
            {
                out.push(ExcludeRule::Pattern(built));
            }
        } else {
            out.push(ExcludeRule::Exact(p.to_string()));
        }
    }
    out
}

pub struct Session {
    pub id: u64,
    pub root: PathBuf,
    pub index: RwLock<Index>,
    pub stats: Stats,
    pub opts: ScanOptions,
    pub started: Instant,
    pub elapsed_ms: AtomicU64,
    pub elevated: AtomicBool,
    pub mft_bytes: AtomicU64,
    state: AtomicU8,
    pause: AtomicBool,
    cancel: AtomicBool,
    finished: AtomicBool,
    errors: Mutex<Vec<ScanError>>,
    queue: Mutex<Queue>,
    wake: Condvar,
    seen_links: Mutex<HashSet<(u32, u64)>>,
    owners: OwnerCache,
    excludes: Vec<ExcludeRule>,
}

#[derive(Default)]
struct Queue {
    items: Vec<NodeId>,
    active: usize,
}

impl Session {
    pub fn state(&self) -> State {
        match self.state.load(Ordering::SeqCst) {
            0 => State::Running,
            1 => State::Paused,
            2 => State::Done,
            _ => State::Cancelled,
        }
    }
    pub(crate) fn set_state(&self, s: State) {
        self.state.store(s as u8, Ordering::SeqCst);
    }
    pub fn is_finished(&self) -> bool {
        matches!(self.state(), State::Done | State::Cancelled)
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
    pub fn is_paused(&self) -> bool {
        self.pause.load(Ordering::SeqCst)
    }
    pub fn pause(&self) {
        if self.state() == State::Running {
            self.pause.store(true, Ordering::SeqCst);
            self.set_state(State::Paused);
        }
    }
    pub fn resume(&self) {
        if self.state() == State::Paused {
            self.pause.store(false, Ordering::SeqCst);
            self.set_state(State::Running);
        }
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.pause.store(false, Ordering::SeqCst);
        self.wake.notify_all();
    }
    pub fn errors(&self) -> Vec<ScanError> {
        self.errors.lock().unwrap().clone()
    }
    pub(crate) fn set_errors(&self, errs: Vec<ScanError>) {
        self.stats.errors.store(errs.len() as u64, Ordering::SeqCst);
        *self.errors.lock().unwrap() = errs;
    }
    pub fn elapsed(&self) -> Duration {
        if self.is_finished() {
            Duration::from_millis(self.elapsed_ms.load(Ordering::SeqCst))
        } else {
            self.started.elapsed()
        }
    }

    /// Creates an empty running session shell whose progress and final index will be populated by
    /// an external worker (such as the elevated UAC helper).
    pub fn new_shell(id: u64, root: PathBuf, opts: ScanOptions, elevated: bool) -> Arc<Session> {
        let name = root.to_string_lossy().trim_end_matches('\\').to_string();
        let excludes = compile_excludes(&opts.exclude);
        Arc::new(Session {
            id,
            index: RwLock::new(Index::new(root.clone(), &name)),
            root,
            stats: Stats::default(),
            opts,
            started: Instant::now(),
            elapsed_ms: AtomicU64::new(0),
            elevated: AtomicBool::new(elevated),
            mft_bytes: AtomicU64::new(0),
            state: AtomicU8::new(0),
            pause: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            errors: Mutex::new(Vec::new()),
            queue: Mutex::new(Queue::default()),
            wake: Condvar::new(),
            seen_links: Mutex::new(HashSet::new()),
            owners: OwnerCache::default(),
            excludes,
        })
    }

    pub fn to_saved(&self) -> SavedScan {
        let index = self.index.read().unwrap().to_archive();
        SavedScan {
            index,
            errors: self.errors(),
            elapsed_ms: self.elapsed().as_millis() as u64,
            elevated: self.elevated.load(Ordering::Relaxed),
            mft_bytes: self.mft_bytes.load(Ordering::Relaxed),
        }
    }

    pub fn from_saved(id: u64, saved: SavedScan) -> Result<Arc<Session>, String> {
        let idx = Index::from_archive(saved.index)?;
        let root = idx.root_path.clone();
        let root_node = idx.node(0).ok_or("Scan has no root.")?;
        let (files, dirs, bytes) = (
            u64::from(root_node.files),
            u64::from(root_node.dirs),
            root_node.size,
        );
        let err_count = saved.errors.len() as u64;
        let s = Arc::new(Session {
            id,
            root,
            index: RwLock::new(idx),
            stats: Stats {
                files: AtomicU64::new(files),
                dirs: AtomicU64::new(dirs),
                bytes: AtomicU64::new(bytes),
                errors: AtomicU64::new(err_count),
            },
            opts: ScanOptions::default(),
            started: Instant::now(),
            elapsed_ms: AtomicU64::new(saved.elapsed_ms),
            elevated: AtomicBool::new(saved.elevated),
            mft_bytes: AtomicU64::new(saved.mft_bytes),
            state: AtomicU8::new(State::Done as u8),
            pause: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            finished: AtomicBool::new(true),
            errors: Mutex::new(saved.errors),
            queue: Mutex::new(Queue::default()),
            wake: Condvar::new(),
            seen_links: Mutex::new(HashSet::new()),
            owners: OwnerCache::default(),
            excludes: Vec::new(),
        });
        Ok(s)
    }

    /// Starts scanning `root`. Returns at once; the work runs on its own threads.
    pub fn start(id: u64, root: PathBuf, opts: ScanOptions) -> Arc<Session> {
        let elevated = ntfs::is_elevated();
        if elevated {
            let _ = ntfs::enable_backup_privilege();
        }
        let session = Self::new_shell(id, root.clone(), opts, elevated);
        if elevated {
            if let Some(mft) = ntfs::ntfs_mft_bytes(&root) {
                session.mft_bytes.store(mft, Ordering::Relaxed);
            }
        }
        if session.opts.collect_owners {
            let root_owner = session.owners.owner_of(&root);
            session
                .index
                .write()
                .unwrap()
                .set_owner(0, root_owner.as_deref());
        }
        session.launch(vec![0]);
        session
    }

    /// Scans one folder again and replaces what the index held for it. Totals above it are
    /// corrected as the new result arrives.
    pub fn rescan(self: &Arc<Self>, node: NodeId) -> bool {
        if !self.is_finished() {
            return false;
        }
        {
            let mut idx = self.index.write().unwrap();
            match idx.node(node) {
                Some(n) if n.is_dir() && !n.is_link() => {}
                _ => return false,
            }
            idx.detach_children(node);
        }
        self.finished.store(false, Ordering::SeqCst);
        self.cancel.store(false, Ordering::SeqCst);
        self.pause.store(false, Ordering::SeqCst);
        self.set_state(State::Running);
        self.launch(vec![node]);
        true
    }

    fn launch(self: &Arc<Self>, start: Vec<NodeId>) {
        {
            let mut q = self.queue.lock().unwrap();
            q.items = start;
            q.active = 0;
        }
        let n = self.opts.workers.clamp(1, 32);
        for i in 0..n {
            let me = self.clone();
            std::thread::Builder::new()
                .name(format!("storage-scan-{i}"))
                .spawn(move || me.worker())
                .expect("couldn't start a scan thread");
        }
    }

    fn worker(self: Arc<Self>) {
        loop {
            let id = {
                let mut q = self.queue.lock().unwrap();
                loop {
                    if self.cancel.load(Ordering::SeqCst) {
                        drop(q);
                        self.finish(true);
                        return;
                    }
                    if let Some(id) = q.items.pop() {
                        q.active += 1;
                        break id;
                    }
                    if q.active == 0 {
                        drop(q);
                        self.finish(false);
                        return;
                    }
                    q = self
                        .wake
                        .wait_timeout(q, Duration::from_millis(100))
                        .unwrap()
                        .0;
                }
            };
            while self.pause.load(Ordering::SeqCst) && !self.cancel.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(50));
            }
            let found = if self.cancel.load(Ordering::SeqCst) {
                Vec::new()
            } else {
                self.scan_dir(id)
            };
            let mut q = self.queue.lock().unwrap();
            q.active -= 1;
            q.items.extend(found);
            drop(q);
            self.wake.notify_all();
        }
    }

    pub(crate) fn finish(&self, cancelled: bool) {
        if self.finished.swap(true, Ordering::SeqCst) {
            return;
        }
        self.elapsed_ms
            .store(self.started.elapsed().as_millis() as u64, Ordering::SeqCst);
        self.set_state(if cancelled {
            State::Cancelled
        } else {
            State::Done
        });
        self.wake.notify_all();
    }

    pub(crate) fn record_error(&self, path: &Path, code: u32) {
        self.stats.errors.fetch_add(1, Ordering::Relaxed);
        let mut e = self.errors.lock().unwrap();
        if e.len() < MAX_ERRORS_KEPT {
            e.push(ScanError {
                path: path.to_string_lossy().to_string(),
                code,
                message: describe_error(code),
            });
        }
    }

    fn is_excluded(&self, name: &str) -> bool {
        !self.excludes.is_empty() && self.excludes.iter().any(|r| r.matches(name))
    }

    fn scan_dir(&self, id: NodeId) -> Vec<NodeId> {
        let dir = self.index.read().unwrap().path_of(id);
        let mut pattern = long_path(&dir);
        pattern.pop();
        if pattern.last() != Some(&(b'\\' as u16)) {
            pattern.push(b'\\' as u16);
        }
        pattern.push(b'*' as u16);
        pattern.push(0);
        let mut data = WIN32_FIND_DATAW::default();
        // SAFETY: pattern is NUL-terminated; `data` is a plain output struct.
        let handle = unsafe {
            FindFirstFileExW(
                PCWSTR(pattern.as_ptr()),
                FindExInfoBasic,
                &mut data as *mut _ as *mut _,
                FindExSearchNameMatch,
                None,
                FIND_FIRST_EX_LARGE_FETCH,
            )
        };
        let handle = match handle {
            Ok(h) => h,
            Err(e) => {
                self.record_error(&dir, (e.code().0 & 0xFFFF) as u32);
                return self
                    .index
                    .write()
                    .unwrap()
                    .add_children(id, Vec::new(), true);
            }
        };
        let dir_owner = if self.opts.collect_owners {
            self.owners.owner_of(&dir)
        } else {
            None
        };
        let mut entries = Vec::new();
        let mut incomplete = false;
        loop {
            let name_len = data
                .cFileName
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(data.cFileName.len());
            let wname = &data.cFileName[..name_len];
            let is_dots = wname == [b'.' as u16] || wname == [b'.' as u16, b'.' as u16];
            if !is_dots {
                let name_string = String::from_utf16_lossy(wname);
                if !self.is_excluded(&name_string) {
                    let lossy = name_string.encode_utf16().ne(wname.iter().copied());
                    entries.push(self.make_entry(
                        &dir,
                        name_string,
                        lossy,
                        &data,
                        dir_owner.as_deref(),
                    ));
                }
            }
            // SAFETY: handle is valid until FindClose.
            if let Err(e) = unsafe { FindNextFileW(handle, &mut data) } {
                let code = (e.code().0 & 0xFFFF) as u32;
                if code != 18 {
                    // 18 is "no more files", the normal end.
                    self.record_error(&dir, code);
                    incomplete = true;
                }
                break;
            }
        }
        // SAFETY: handle came from FindFirstFileExW.
        unsafe {
            let _ = FindClose(handle);
        }
        let (mut files, mut dirs, mut bytes) = (0u64, 0u64, 0u64);
        for e in &entries {
            if e.flags & F_DIR != 0 {
                dirs += 1;
            } else {
                files += 1;
                bytes += e.size;
            }
        }
        let found = self
            .index
            .write()
            .unwrap()
            .add_children(id, entries, incomplete);
        self.stats.files.fetch_add(files, Ordering::Relaxed);
        self.stats.dirs.fetch_add(dirs, Ordering::Relaxed);
        self.stats.bytes.fetch_add(bytes, Ordering::Relaxed);
        found
    }

    fn make_entry(
        &self,
        dir: &Path,
        name: String,
        lossy: bool,
        d: &WIN32_FIND_DATAW,
        dir_owner: Option<&str>,
    ) -> NewEntry {
        let attrs = d.dwFileAttributes;
        let is_dir = attrs & ATTR_DIRECTORY != 0;
        let is_reparse = attrs & ATTR_REPARSE != 0;
        let mut flags = 0u16;
        if is_dir {
            flags |= F_DIR;
        }
        if is_reparse {
            flags |= F_LINK;
        }
        let ft = |t: windows::Win32::Foundation::FILETIME| {
            (i64::from(t.dwHighDateTime) << 32) | i64::from(t.dwLowDateTime)
        };
        let size = if is_dir {
            0
        } else {
            (u64::from(d.nFileSizeHigh) << 32) | u64::from(d.nFileSizeLow)
        };
        let need_full_path = (!is_dir
            && ((attrs & (ATTR_SPARSE | ATTR_COMPRESSED) != 0)
                || (!is_reparse && size >= self.opts.hardlink_min)
                || self.opts.collect_streams))
            || (self.opts.collect_owners && (is_dir || size >= (1 << 20)));
        let full = need_full_path.then(|| dir.join(&name));

        let mut alloc = 0u64;
        if !is_dir {
            let cloud =
                attrs & (ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS) != 0;
            if cloud {
                alloc = 0;
            } else if attrs & (ATTR_SPARSE | ATTR_COMPRESSED) != 0 {
                if attrs & ATTR_SPARSE != 0 {
                    flags |= F_SPARSE;
                }
                if attrs & ATTR_COMPRESSED != 0 {
                    flags |= F_COMPRESSED;
                }
                let p = full.as_ref().unwrap();
                alloc = compressed_size(p).unwrap_or_else(|| round_up(size, self.opts.cluster));
            } else {
                alloc = round_up(size, self.opts.cluster);
            }
            if alloc > 0
                && !is_reparse
                && size >= self.opts.hardlink_min
                && self.is_duplicate_hardlink(full.as_ref().unwrap())
            {
                alloc = 0;
                flags |= F_HARDLINK_DUP;
            }
        }
        // Folders and large files query their own ACL owner; smaller files inherit the folder's
        // owner unless they have custom security, keeping 1M-file scans fast without extra opens.
        let owner = if !self.opts.collect_owners || lossy {
            None
        } else if let Some(p) = full.as_ref().filter(|_| is_dir || size >= (1 << 20)) {
            self.owners
                .owner_of(p)
                .or_else(|| dir_owner.map(str::to_string))
        } else {
            dir_owner.map(str::to_string)
        };
        let (ads_count, ads_size) = if !is_dir && self.opts.collect_streams && !lossy {
            ntfs::alternate_streams(full.as_ref().unwrap())
        } else {
            (0, 0)
        };
        NewEntry {
            name,
            lossy,
            flags,
            size,
            alloc,
            mtime: ft(d.ftLastWriteTime),
            ctime: ft(d.ftCreationTime),
            atime: ft(d.ftLastAccessTime),
            attrs,
            owner,
            ads_count,
            ads_size,
        }
    }

    /// True when this file is another name for data already counted.
    fn is_duplicate_hardlink(&self, path: &Path) -> bool {
        let Ok(file) = std::fs::OpenOptions::new()
            .access_mode(0x80) // FILE_READ_ATTRIBUTES
            .share_mode(7)
            .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
            .open(long_os_path(path))
        else {
            return false;
        };
        let mut raw = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: handle is valid while `file` lives; `raw` is a plain output struct.
        if unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut raw) }.is_err() {
            return false;
        }
        if raw.nNumberOfLinks <= 1 {
            return false;
        }
        let key = (
            raw.dwVolumeSerialNumber,
            (u64::from(raw.nFileIndexHigh) << 32) | u64::from(raw.nFileIndexLow),
        );
        !self.seen_links.lock().unwrap().insert(key)
    }
}

pub const ATTR_DIRECTORY_FLAG: u32 = ATTR_DIRECTORY;

pub fn round_up(size: u64, cluster: u32) -> u64 {
    if size == 0 || cluster == 0 {
        0
    } else {
        size.div_ceil(u64::from(cluster)) * u64::from(cluster)
    }
}

/// Space a compressed or sparse file really uses.
pub fn compressed_size(path: &Path) -> Option<u64> {
    let w = long_path(path);
    let mut high = 0u32;
    // SAFETY: w is NUL-terminated; `high` is a valid out pointer.
    let low = unsafe { GetCompressedFileSizeW(PCWSTR(w.as_ptr()), Some(&mut high)) };
    if low == u32::MAX {
        // SAFETY: plain thread-local error read.
        if unsafe { GetLastError() }.0 != 0 {
            return None;
        }
    }
    Some((u64::from(high) << 32) | u64::from(low))
}

/// Extended-length form so paths over 260 characters work: `\\?\C:\...` or `\\?\UNC\server\...`.
pub fn long_os_path(p: &Path) -> PathBuf {
    // The extended-length form isn't normalized by Windows, so forward slashes must go here.
    let s = p.to_string_lossy().replace('/', "\\");
    let s = s.as_str();
    if s.starts_with(r"\\?\") {
        p.to_path_buf()
    } else if let Some(rest) = s.strip_prefix(r"\\") {
        PathBuf::from(format!(r"\\?\UNC\{rest}"))
    } else {
        PathBuf::from(format!(r"\\?\{s}"))
    }
}

fn long_path(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    long_os_path(p)
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn describe_error(code: u32) -> String {
    match code {
        5 => "Access denied".into(),
        32 => "In use by another program".into(),
        3 | 2 => "Not found (changed during the scan)".into(),
        53 | 67 => "Network path unavailable".into(),
        1920 => "Can't be opened (cloud or special folder)".into(),
        c => format!("Windows error {c}"),
    }
}
