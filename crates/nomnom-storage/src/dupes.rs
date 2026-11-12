//! Finds files with identical content. Cheap checks first (size, then a hash of the first and last
//! 64 KB), the full SHA-256 only for what is left. Names of one file (hardlinks) are never
//! reported as copies: they already share their space.

use crate::index::{Index, NodeId, F_LINK};
use crate::scan::long_os_path;
use nomnom_organizer::winfs::{self, FileId};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

const QUICK: u64 = 64 * 1024;
const BLOCK: usize = 1 << 20;

#[derive(Serialize, Clone, Debug)]
pub struct DupCopy {
    pub node: NodeId,
    pub path: String,
    pub mtime: Option<i64>,
    /// More than one means several names of the same file; deleting one of them frees nothing.
    pub names: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct DupGroup {
    pub size: u64,
    pub sha256: String,
    pub copies: Vec<DupCopy>,
    /// What deleting every copy but one would give back.
    pub recoverable: u64,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Listing,
    Comparing,
    Hashing,
    Done,
    Cancelled,
}

pub struct Job {
    stage: AtomicU8,
    pub candidates: AtomicU64,
    pub files_done: AtomicU64,
    pub files_total: AtomicU64,
    pub bytes_done: AtomicU64,
    pub bytes_total: AtomicU64,
    pub unreadable: AtomicU64,
    cancel: AtomicBool,
    result: Mutex<Vec<DupGroup>>,
}

#[derive(Serialize, Clone, Debug)]
pub struct JobProgress {
    pub stage: Stage,
    pub candidates: u64,
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub unreadable: u64,
    pub groups: usize,
    pub recoverable: u64,
}

impl Job {
    pub fn stage(&self) -> Stage {
        match self.stage.load(Ordering::SeqCst) {
            0 => Stage::Listing,
            1 => Stage::Comparing,
            2 => Stage::Hashing,
            3 => Stage::Done,
            _ => Stage::Cancelled,
        }
    }
    fn set(&self, s: Stage) {
        self.stage.store(s as u8, Ordering::SeqCst);
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
    pub fn is_finished(&self) -> bool {
        matches!(self.stage(), Stage::Done | Stage::Cancelled)
    }
    pub fn progress(&self) -> JobProgress {
        let r = self.result.lock().unwrap();
        JobProgress {
            stage: self.stage(),
            candidates: self.candidates.load(Ordering::Relaxed),
            files_done: self.files_done.load(Ordering::Relaxed),
            files_total: self.files_total.load(Ordering::Relaxed),
            bytes_done: self.bytes_done.load(Ordering::Relaxed),
            bytes_total: self.bytes_total.load(Ordering::Relaxed),
            unreadable: self.unreadable.load(Ordering::Relaxed),
            groups: r.len(),
            recoverable: r.iter().map(|g| g.recoverable).sum(),
        }
    }
    pub fn groups(&self, offset: usize, limit: usize) -> Vec<DupGroup> {
        self.result
            .lock()
            .unwrap()
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }
    pub fn group_of(&self, node: NodeId) -> Option<DupGroup> {
        self.result
            .lock()
            .unwrap()
            .iter()
            .find(|g| g.copies.iter().any(|c| c.node == node))
            .cloned()
    }
}

#[derive(Clone)]
struct Entry {
    nodes: Vec<(NodeId, PathBuf, Option<i64>)>,
    size: u64,
}

/// Starts looking for duplicates under `scope`. Files smaller than `min_size` are ignored.
pub fn start(
    session: Arc<crate::scan::Session>,
    scope: NodeId,
    min_size: u64,
    workers: usize,
) -> Arc<Job> {
    let job = Arc::new(Job {
        stage: AtomicU8::new(0),
        candidates: AtomicU64::new(0),
        files_done: AtomicU64::new(0),
        files_total: AtomicU64::new(0),
        bytes_done: AtomicU64::new(0),
        bytes_total: AtomicU64::new(0),
        unreadable: AtomicU64::new(0),
        cancel: AtomicBool::new(false),
        result: Mutex::new(Vec::new()),
    });
    let j = job.clone();
    std::thread::Builder::new()
        .name("storage-dupes".into())
        .spawn(move || {
            let groups = find(
                &session.index,
                scope,
                min_size.max(1),
                workers.clamp(1, 8),
                &j,
            );
            *j.result.lock().unwrap() = groups;
            j.set(if j.cancel.load(Ordering::SeqCst) {
                Stage::Cancelled
            } else {
                Stage::Done
            });
        })
        .expect("couldn't start the duplicate search");
    job
}

fn find(
    index: &std::sync::RwLock<Index>,
    scope: NodeId,
    min_size: u64,
    workers: usize,
    job: &Job,
) -> Vec<DupGroup> {
    // 1. List files by size. The index is only locked while listing.
    let mut by_size: HashMap<u64, Vec<(NodeId, PathBuf, Option<i64>)>> = HashMap::new();
    {
        let idx = index.read().unwrap();
        let mut stack = vec![scope];
        while let Some(id) = stack.pop() {
            if job.cancel.load(Ordering::Relaxed) {
                return Vec::new();
            }
            let n = idx.node(id).unwrap();
            if n.is_dir() {
                if !n.is_link() {
                    stack.extend(idx.children(id));
                }
            } else if n.size >= min_size && n.flags & F_LINK == 0 {
                by_size.entry(n.size).or_default().push((
                    id,
                    idx.path_of(id),
                    crate::index::filetime_to_unix(n.mtime),
                ));
            }
        }
    }
    by_size.retain(|_, v| v.len() > 1);
    job.candidates.store(
        by_size.values().map(|v| v.len() as u64).sum(),
        Ordering::Relaxed,
    );
    job.set(Stage::Comparing);

    // 2. Names of one file become one entry.
    let mut groups: Vec<Vec<Entry>> = Vec::new();
    for (size, list) in by_size {
        let mut by_id: HashMap<FileId, Entry> = HashMap::new();
        for (node, path, mtime) in list {
            if job.cancel.load(Ordering::Relaxed) {
                return Vec::new();
            }
            match winfs::info(&long_os_path(&path)) {
                Ok(i) if !i.is_reparse() && !i.is_dir() => by_id
                    .entry(i.id)
                    .or_insert_with(|| Entry {
                        nodes: Vec::new(),
                        size,
                    })
                    .nodes
                    .push((node, path, mtime)),
                _ => {
                    job.unreadable.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        if by_id.len() > 1 {
            groups.push(by_id.into_values().collect());
        }
    }

    // 3. A quick hash of the head and tail splits most groups.
    let entries: Vec<Entry> = groups.into_iter().flatten().collect();
    job.files_total
        .store(entries.len() as u64 * 2, Ordering::Relaxed);
    job.bytes_total.store(
        entries.iter().map(|e| e.size).sum::<u64>(),
        Ordering::Relaxed,
    );
    job.set(Stage::Hashing);
    let quick = hash_all(entries, workers, job, |e| {
        quick_hash(&e.nodes[0].1, e.size).map(|h| (h, 0))
    });
    let mut by_quick: HashMap<(u64, [u8; 32]), Vec<Entry>> = HashMap::new();
    for (e, h) in quick {
        if let Some(h) = h {
            by_quick.entry((e.size, h)).or_default().push(e);
        }
    }
    let survivors: Vec<Entry> = by_quick
        .into_values()
        .filter(|v| v.len() > 1)
        .flatten()
        .collect();

    // 4. Full SHA-256 for what is left.
    let full = hash_all(survivors, workers, job, |e| {
        full_hash(&e.nodes[0].1, &job.cancel, &job.bytes_done).map(|h| (h, e.size))
    });
    let mut by_full: HashMap<(u64, [u8; 32]), Vec<Entry>> = HashMap::new();
    for (e, h) in full {
        if let Some(h) = h {
            by_full.entry((e.size, h)).or_default().push(e);
        }
    }
    let mut out: Vec<DupGroup> = by_full
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|((size, hash), v)| {
            let mut copies: Vec<DupCopy> = v
                .into_iter()
                .map(|e| {
                    let (node, path, mtime) = e.nodes[0].clone();
                    DupCopy {
                        node,
                        path: path.to_string_lossy().to_string(),
                        mtime,
                        names: e.nodes.len() as u32,
                    }
                })
                .collect();
            copies.sort_by_key(|c| c.path.to_lowercase());
            let recoverable = size * (copies.len() as u64 - 1);
            DupGroup {
                size,
                sha256: hash.iter().map(|b| format!("{b:02x}")).collect(),
                copies,
                recoverable,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.recoverable
            .cmp(&a.recoverable)
            .then(a.sha256.cmp(&b.sha256))
    });
    out
}

type Hashed = Vec<(Entry, Option<[u8; 32]>)>;

fn hash_all(
    entries: Vec<Entry>,
    workers: usize,
    job: &Job,
    f: impl Fn(&Entry) -> io::Result<([u8; 32], u64)> + Sync,
) -> Hashed {
    let queue = Mutex::new(entries.into_iter().collect::<VecDeque<_>>());
    let out = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                if job.cancel.load(Ordering::Relaxed) {
                    return;
                }
                let Some(e) = queue.lock().unwrap().pop_front() else {
                    return;
                };
                let r = match f(&e) {
                    Ok((h, _)) => Some(h),
                    Err(_) => {
                        job.unreadable.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                };
                job.files_done.fetch_add(1, Ordering::Relaxed);
                out.lock().unwrap().push((e, r));
            });
        }
    });
    out.into_inner().unwrap()
}

fn quick_hash(path: &std::path::Path, size: u64) -> io::Result<[u8; 32]> {
    let mut f = File::open(long_os_path(path))?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; QUICK as usize];
    let n = f.read(&mut buf)?;
    h.update(&buf[..n]);
    if size > QUICK * 2 {
        f.seek(SeekFrom::Start(size - QUICK))?;
        let n = f.read(&mut buf)?;
        h.update(&buf[..n]);
    }
    Ok(*h.finalize().as_bytes())
}

fn full_hash(
    path: &std::path::Path,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> io::Result<[u8; 32]> {
    let mut f = File::open(long_os_path(path))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; BLOCK];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(io::Error::other("stopped"));
        }
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        progress.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(h.finalize().into())
}
