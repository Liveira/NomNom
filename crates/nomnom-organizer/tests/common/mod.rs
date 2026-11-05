#![allow(dead_code)]
//! Test helpers: a throwaway "Downloads" and "Documents", realistic file contents, and a
//! byte-for-byte snapshot of a folder tree so "untouched" can be proven rather than assumed.

use nomnom_core::db::Db;
use nomnom_organizer::journal::Journal;
use nomnom_organizer::model::Options;
use nomnom_organizer::winfs;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub struct Sandbox {
    pub _root: tempfile::TempDir,
    pub downloads: PathBuf,
    pub dest: PathBuf,
}

impl Sandbox {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let downloads = root.path().join("Downloads");
        let dest = root.path().join("Documents").join("Files");
        fs::create_dir_all(&downloads).unwrap();
        Sandbox {
            _root: root,
            downloads,
            dest,
        }
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.downloads.join(rel)
    }

    /// Writes a file (creating folders) and makes it look an hour old, so it counts as settled.
    pub fn put(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let p = self.path(rel);
        put_at(&p, bytes);
        p
    }

    pub fn options(&self) -> Options {
        let mut o = Options::new(self.downloads.clone(), self.dest.clone());
        o.settle_secs = 0;
        o
    }

    pub fn journal(&self) -> Journal {
        Journal::new(Arc::new(Db::open_in_memory().unwrap()))
    }
}

pub fn put_at(p: &Path, bytes: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
    age(p);
}

pub fn age(p: &Path) {
    let f = fs::OpenOptions::new().write(true).open(p).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(3600))
        .unwrap();
}

pub fn mp4(tag: u8) -> Vec<u8> {
    let mut v = vec![0, 0, 0, 0x18];
    v.extend_from_slice(b"ftypisom\0\0\0\0isomiso2");
    v.extend(std::iter::repeat_n(tag, 64));
    v
}
pub fn mkv(tag: u8) -> Vec<u8> {
    let mut v = vec![0x1A, 0x45, 0xDF, 0xA3, 0x9F];
    v.extend(std::iter::repeat_n(tag, 64));
    v
}
pub fn png(tag: u8) -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10];
    v.extend(std::iter::repeat_n(tag, 64));
    v
}
pub fn jpg(tag: u8) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10];
    v.extend(std::iter::repeat_n(tag, 64));
    v
}
pub fn pdf(tag: u8) -> Vec<u8> {
    let mut v = b"%PDF-1.7\n".to_vec();
    v.extend(std::iter::repeat_n(tag, 64));
    v
}
pub fn mp3(tag: u8) -> Vec<u8> {
    let mut v = b"ID3\x03\0\0\0\0\0\0".to_vec();
    v.extend(std::iter::repeat_n(tag, 64));
    v
}
pub fn exe() -> Vec<u8> {
    let mut v = b"MZ\x90\0\x03\0\0\0".to_vec();
    v.extend(std::iter::repeat_n(0xAA, 200));
    v
}
pub fn text(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

/// What a tree looks like: for every entry, its size, modified time and (for files) content hash.
/// Links are recorded as links and never followed.
pub type Snapshot = BTreeMap<String, String>;

pub fn snapshot(root: &Path) -> Snapshot {
    fn walk(base: &Path, dir: &Path, out: &mut Snapshot) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            let rel = p
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .to_lowercase();
            let Ok(i) = winfs::info(&p) else {
                out.insert(rel, "unreadable".into());
                continue;
            };
            if i.is_reparse() {
                out.insert(rel, "link".into());
            } else if i.is_dir() {
                out.insert(rel.clone(), format!("dir attrs={:x}", i.attrs));
                walk(base, &p, out);
            } else {
                let h = blake3::hash(&fs::read(&p).unwrap_or_default());
                out.insert(
                    rel,
                    format!(
                        "file size={} mtime={} attrs={:x} hash={}",
                        i.size, i.mtime, i.attrs, h
                    ),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Hashes of every regular file under the given folders, sorted. Moving files around must never
/// change this list: nothing may be lost, duplicated or altered.
pub fn all_content(roots: &[&Path]) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            let Ok(i) = winfs::info(&p) else { continue };
            if i.is_reparse() {
                continue;
            }
            if i.is_dir() {
                walk(&p, out);
            } else {
                out.push(blake3::hash(&fs::read(&p).unwrap_or_default()).to_string());
            }
        }
    }
    let mut out = Vec::new();
    for r in roots {
        walk(r, &mut out);
    }
    out.sort();
    out
}

pub fn exists(p: &Path) -> bool {
    winfs::info(p).is_ok()
}
