//! Comparing two folders and bringing them closer. A comparison only reads. Copying never deletes: a
//! file that would be replaced is first moved into a `.nomnom-replaced` folder inside the side it was
//! on, so the old version can always be found.

use crate::state::AppState;
use nomnom_core::compare::{self as rules, Action, Entry, Row, Status};
use nomnom_organizer::pathutil;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;

type CmdResult<T> = Result<T, String>;

const MAX_FILES: usize = 100_000;
const MAX_DEPTH: usize = 30;
const MAX_ACTIONS: usize = 5000;
const KEPT: &str = ".nomnom-replaced";

fn secs(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The files under a folder (links are not followed, and the folder of replaced versions is skipped).
fn walk(root: &Path) -> (Vec<Entry>, bool) {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new(), 0usize)];
    let mut truncated = false;
    while let Some((dir, prefix, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            if ft.is_dir() {
                if name != KEPT && depth < MAX_DEPTH {
                    stack.push((e.path(), rel, depth + 1));
                }
            } else if ft.is_file() {
                if out.len() >= MAX_FILES {
                    truncated = true;
                    continue;
                }
                if let Ok(m) = e.metadata() {
                    out.push(Entry {
                        rel,
                        size: m.len(),
                        modified: m.modified().map(secs).unwrap_or(0),
                    });
                }
            }
        }
    }
    (out, truncated)
}

fn check_side(state: &AppState, folder: &str) -> CmdResult<PathBuf> {
    let p = PathBuf::from(folder.trim());
    crate::folder_rules::allowed(state, &p)?;
    if !p.is_dir() {
        return Err("One of the folders isn't there.".into());
    }
    Ok(p)
}

fn check_pair(state: &AppState, left: &str, right: &str) -> CmdResult<(PathBuf, PathBuf)> {
    let (l, r) = (check_side(state, left)?, check_side(state, right)?);
    if pathutil::is_same(&l, &r) || pathutil::is_inside(&l, &r) || pathutil::is_inside(&r, &l) {
        return Err("Choose two different folders, neither inside the other.".into());
    }
    Ok((l, r))
}

fn same_content(a: &Path, b: &Path) -> bool {
    fn hash(p: &Path) -> Option<[u8; 32]> {
        let mut f = std::fs::File::open(p).ok()?;
        let mut h = blake3::Hasher::new();
        std::io::copy(&mut f, &mut h).ok()?;
        Some(*h.finalize().as_bytes())
    }
    matches!((hash(a), hash(b)), (Some(x), Some(y)) if x == y)
}

#[derive(Debug, Serialize)]
pub struct Scan {
    pub rows: Vec<Row>,
    /// A folder had more files than are compared at once.
    pub truncated: bool,
}

/// Compares two folders. With `exact`, files of the same size that look different by time are read
/// to see whether they really are.
#[tauri::command]
pub async fn cmp_scan(
    state: State<'_, AppState>,
    left: String,
    right: String,
    exact: bool,
) -> CmdResult<Scan> {
    let (l, r) = check_pair(&state, &left, &right)?;
    tokio::task::spawn_blocking(move || {
        let (le, lt) = walk(&l);
        let (re, rt) = walk(&r);
        let mut rows = rules::compare(&le, &re);
        if exact {
            for row in rows.iter_mut() {
                if matches!(
                    row.status,
                    Status::LeftNewer | Status::RightNewer | Status::Different
                ) && row.left.map(|x| x.0) == row.right.map(|x| x.0)
                    && same_content(&l.join(&row.rel), &r.join(&row.rel))
                {
                    row.status = Status::Same;
                }
            }
        }
        rows.retain(|row| rules::safe_rel(&row.rel));
        Scan {
            rows,
            truncated: lt || rt,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct Todo {
    pub rel: String,
    pub action: Action,
}

#[derive(Debug, Serialize, Default)]
pub struct Applied {
    pub copied: usize,
    /// How many of those replaced a file (the old one is in the kept folder).
    pub replaced: usize,
    pub failed: Vec<(String, String)>,
}

fn no_links_between(root: &Path, target: &Path) -> bool {
    let Ok(rest) = target.strip_prefix(root) else {
        return false;
    };
    let mut cur = root.to_path_buf();
    for part in rest.components() {
        cur.push(part);
        if std::fs::symlink_metadata(&cur).is_ok_and(|m| m.file_type().is_symlink()) {
            return false;
        }
    }
    true
}

fn copy_one(src_root: &Path, dst_root: &Path, rel: &str, stamp: &str) -> Result<bool, String> {
    if !rules::safe_rel(rel) {
        return Err("That path is not allowed.".into());
    }
    let src = src_root.join(rel);
    let dst = dst_root.join(rel);
    let meta = std::fs::symlink_metadata(&src).map_err(|_| "The file is gone.".to_string())?;
    if !meta.is_file() {
        return Err("It is not a plain file.".into());
    }
    if !no_links_between(src_root, &src) || !no_links_between(dst_root, &dst) {
        return Err("A link is in the way.".into());
    }
    let parent = dst.parent().ok_or("No folder.")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Couldn't make the folder: {e}"))?;
    // The copy is written beside its place and moved in, so a half-copied file is never left behind.
    let tmp = parent.join(format!(".nomnom-{}.tmp", uuid::Uuid::new_v4().simple()));
    let result = (|| -> Result<bool, String> {
        std::fs::copy(&src, &tmp).map_err(|e| format!("Couldn't copy it: {e}"))?;
        if let Ok(t) = meta.modified() {
            if let Ok(f) = std::fs::OpenOptions::new().write(true).open(&tmp) {
                let _ = f.set_modified(t);
            }
        }
        let mut replaced = false;
        if dst.exists() {
            let kept = dst_root.join(KEPT).join(stamp).join(rel);
            std::fs::create_dir_all(kept.parent().ok_or("No folder.")?)
                .map_err(|e| e.to_string())?;
            nomnom_core::fs::move_no_replace(&dst, &kept)
                .map_err(|e| format!("Couldn't keep the old version: {e}"))?;
            replaced = true;
        }
        nomnom_core::fs::move_no_replace(&tmp, &dst)
            .map_err(|e| format!("Couldn't put it in place: {e}"))?;
        Ok(replaced)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Does what was chosen: copies, one file at a time. Each file that would be replaced keeps its old
/// version in `.nomnom-replaced`.
#[tauri::command]
pub async fn cmp_apply(
    state: State<'_, AppState>,
    left: String,
    right: String,
    todo: Vec<Todo>,
) -> CmdResult<Applied> {
    let (l, r) = check_pair(&state, &left, &right)?;
    if todo.len() > MAX_ACTIONS {
        return Err(format!("At most {MAX_ACTIONS} files at a time."));
    }
    tokio::task::spawn_blocking(move || {
        let stamp = chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string();
        let mut out = Applied::default();
        for t in todo {
            let (from, to) = match t.action {
                Action::CopyRight => (&l, &r),
                Action::CopyLeft => (&r, &l),
            };
            match copy_one(from, to, &t.rel, &stamp) {
                Ok(replaced) => {
                    out.copied += 1;
                    out.replaced += usize::from(replaced);
                }
                Err(e) => out.failed.push((t.rel, e)),
            }
        }
        out
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(dir: &Path, rel: &str, text: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn walking_lists_files_with_slashes_and_skips_the_kept_folder() {
        let d = tempfile::tempdir().unwrap();
        put(d.path(), "a.txt", "1");
        put(d.path(), "sub/b.txt", "22");
        put(d.path(), ".nomnom-replaced/x/old.txt", "old");
        let (mut e, truncated) = walk(d.path());
        e.sort_by(|a, b| a.rel.cmp(&b.rel));
        assert!(!truncated);
        assert_eq!(
            e.iter().map(|x| x.rel.as_str()).collect::<Vec<_>>(),
            vec!["a.txt", "sub/b.txt"]
        );
        assert_eq!(e[1].size, 2);
    }

    #[test]
    fn copying_adds_a_file_and_keeps_its_time() {
        let d = tempfile::tempdir().unwrap();
        let (l, r) = (d.path().join("l"), d.path().join("r"));
        put(&l, "docs/a.txt", "hello");
        std::fs::create_dir_all(&r).unwrap();
        let replaced = copy_one(&l, &r, "docs/a.txt", "S").unwrap();
        assert!(!replaced);
        assert_eq!(
            std::fs::read_to_string(r.join("docs/a.txt")).unwrap(),
            "hello"
        );
        let a = std::fs::metadata(l.join("docs/a.txt"))
            .unwrap()
            .modified()
            .unwrap();
        let b = std::fs::metadata(r.join("docs/a.txt"))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(secs(a), secs(b));
        // No temporary file is left behind.
        let names: Vec<_> = std::fs::read_dir(r.join("docs"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.txt".to_string()]);
    }

    #[test]
    fn replacing_keeps_the_old_version() {
        let d = tempfile::tempdir().unwrap();
        let (l, r) = (d.path().join("l"), d.path().join("r"));
        put(&l, "a.txt", "new");
        put(&r, "a.txt", "old");
        assert!(copy_one(&l, &r, "a.txt", "STAMP").unwrap());
        assert_eq!(std::fs::read_to_string(r.join("a.txt")).unwrap(), "new");
        assert_eq!(
            std::fs::read_to_string(r.join(".nomnom-replaced/STAMP/a.txt")).unwrap(),
            "old"
        );
    }

    #[test]
    fn a_bad_path_or_a_missing_file_is_refused_and_nothing_is_written() {
        let d = tempfile::tempdir().unwrap();
        let (l, r) = (d.path().join("l"), d.path().join("r"));
        put(&l, "a.txt", "x");
        std::fs::create_dir_all(&r).unwrap();
        assert!(copy_one(&l, &r, "../escape.txt", "S").is_err());
        assert!(copy_one(&l, &r, "missing.txt", "S").is_err());
        assert!(copy_one(&l, &r, "C:/x.txt", "S").is_err());
        assert_eq!(std::fs::read_dir(&r).unwrap().count(), 0);
    }

    #[test]
    fn same_content_reads_the_files() {
        let d = tempfile::tempdir().unwrap();
        put(d.path(), "a", "same");
        put(d.path(), "b", "same");
        put(d.path(), "c", "diff");
        assert!(same_content(&d.path().join("a"), &d.path().join("b")));
        assert!(!same_content(&d.path().join("a"), &d.path().join("c")));
        assert!(!same_content(&d.path().join("a"), &d.path().join("nope")));
    }
}
