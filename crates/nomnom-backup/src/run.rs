use crate::{glob::matches_name, inside, restore, Plan, MANIFEST, MARKER, MAX_FILES};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    /// Path inside the version, with `/` between parts.
    pub p: String,
    pub s: u64,
    /// Modified time, Unix seconds.
    pub m: i64,
    /// True when the file is linked to the one in the version before, not copied.
    pub l: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub plan: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub sources: Vec<String>,
    pub files: Vec<Entry>,
}

#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub files: u64,
    pub bytes: u64,
    pub current: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RunReport {
    pub version: String,
    pub files: u64,
    pub bytes: u64,
    /// Bytes actually copied; the rest was linked to the version before.
    pub copied_bytes: u64,
    pub linked: u64,
    /// Things that couldn't be read or copied, so they are not in this version.
    pub skipped: Vec<String>,
    pub removed_versions: Vec<String>,
    pub cancelled: bool,
}

/// `2026-11-18_04-15-30` for Unix seconds, in UTC. Sorts the same as time.
pub fn version_name(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}_{:02}-{:02}-{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn mtime_secs(md: &fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn copy_with_time(src: &Path, dst: &Path, mtime: std::time::SystemTime) -> std::io::Result<u64> {
    let n = fs::copy(src, dst)?;
    // Keep the original modified time, so the next backup can tell the file hasn't changed.
    if let Ok(f) = fs::OpenOptions::new().write(true).open(dst) {
        let _ = f.set_modified(mtime);
    }
    Ok(n)
}

fn unique_top_names(plan: &Plan) -> Vec<(PathBuf, String)> {
    let mut used: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for s in &plan.sources {
        let base = s
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Folder".into());
        let mut name = base.clone();
        let mut n = 2;
        while used.iter().any(|u| u.eq_ignore_ascii_case(&name)) {
            name = format!("{base} ({n})");
            n += 1;
        }
        used.push(name.clone());
        out.push((s.clone(), name));
    }
    out
}

/// Makes one new version of the plan. `now` is the time to record, in Unix seconds.
pub fn run(
    plan: &Plan,
    now: i64,
    cancel: &AtomicBool,
    progress: &dyn Fn(Progress),
) -> Result<RunReport, String> {
    if let Some(p) = plan.problem() {
        return Err(p);
    }
    let home = plan.home();
    fs::create_dir_all(&home).map_err(|e| format!("The backup folder couldn't be made: {e}"))?;
    let marker = home.join(MARKER);
    match fs::read_to_string(&marker) {
        Ok(id) if id.trim() != plan.id => {
            return Err(
                "Another backup already uses this folder. Pick another name or folder.".into(),
            )
        }
        Ok(_) => {}
        Err(_) => fs::write(&marker, &plan.id)
            .map_err(|e| format!("The backup folder can't be written: {e}"))?,
    }

    // The version before, to link unchanged files to.
    let prev = restore::list_versions(&home).into_iter().next();
    let (prev_dir, prev_map): (Option<PathBuf>, HashMap<String, (u64, i64)>) = match &prev {
        Some(v) => match restore::read_manifest(&home, &v.name) {
            Ok(m) => (
                Some(home.join(&v.name)),
                m.files.into_iter().map(|e| (e.p, (e.s, e.m))).collect(),
            ),
            Err(_) => (None, HashMap::new()),
        },
        None => (None, HashMap::new()),
    };

    let name = {
        let mut n = version_name(now);
        let mut i = 2;
        while home.join(&n).exists() {
            n = format!("{}_{i}", version_name(now));
            i += 1;
        }
        n
    };
    let part = home.join(format!(".partial-{name}"));
    let _ = fs::remove_dir_all(&part);
    fs::create_dir_all(&part).map_err(|e| format!("The backup couldn't start: {e}"))?;

    let exclude = plan.exclude_list();
    let skip = |n: &str| exclude.iter().any(|p| matches_name(p, n));
    let mut report = RunReport {
        version: name.clone(),
        ..RunReport::default()
    };
    let mut entries: Vec<Entry> = Vec::new();

    'sources: for (root, top) in unique_top_names(plan) {
        let mut stack: Vec<(PathBuf, String)> = vec![(root.clone(), top.clone())];
        while let Some((dir, rel)) = stack.pop() {
            if let Err(e) = fs::create_dir_all(part.join(&rel)) {
                let _ = fs::remove_dir_all(&part);
                return Err(format!("The backup folder can't be written: {e}"));
            }
            let rd = match fs::read_dir(&dir) {
                Ok(r) => r,
                Err(_) => {
                    if report.skipped.len() < 200 {
                        report.skipped.push(dir.to_string_lossy().into_owned());
                    }
                    continue;
                }
            };
            for e in rd.flatten() {
                if cancel.load(Ordering::Relaxed) {
                    report.cancelled = true;
                    break 'sources;
                }
                let fname = e.file_name().to_string_lossy().into_owned();
                if skip(&fname) {
                    continue;
                }
                let Ok(ft) = e.file_type() else { continue };
                let path = e.path();
                // Links are left out: a link could lead anywhere, and following it could copy the
                // wrong thing or loop forever.
                if ft.is_symlink() || inside(&path, &home) {
                    continue;
                }
                let child_rel = format!("{rel}/{fname}");
                if ft.is_dir() {
                    stack.push((path, child_rel));
                } else if ft.is_file() {
                    let Ok(md) = e.metadata() else { continue };
                    let (size, m) = (md.len(), mtime_secs(&md));
                    let dst = part.join(&child_rel);
                    let linkable = prev_map
                        .get(&child_rel)
                        .is_some_and(|(ps, pm)| *ps == size && (pm - m).abs() <= 1);
                    let mut linked = false;
                    if linkable {
                        if let Some(pd) = &prev_dir {
                            linked = fs::hard_link(pd.join(&child_rel), &dst).is_ok();
                        }
                    }
                    if !linked {
                        let when = md.modified().unwrap_or(std::time::UNIX_EPOCH);
                        match copy_with_time(&path, &dst, when) {
                            Ok(n) => {
                                if n != size {
                                    // The file changed while it was being read; the next run picks it up.
                                }
                                report.copied_bytes += n;
                            }
                            Err(_) => {
                                let _ = fs::remove_file(&dst);
                                if report.skipped.len() < 200 {
                                    report.skipped.push(path.to_string_lossy().into_owned());
                                }
                                continue;
                            }
                        }
                    } else {
                        report.linked += 1;
                    }
                    report.files += 1;
                    report.bytes += size;
                    entries.push(Entry {
                        p: child_rel,
                        s: size,
                        m,
                        l: linked,
                    });
                    if entries.len() > MAX_FILES {
                        let _ = fs::remove_dir_all(&part);
                        return Err(format!(
                            "That is more than {MAX_FILES} files. Pick fewer or smaller folders, or leave some out."
                        ));
                    }
                    if report.files.is_multiple_of(50) {
                        progress(Progress {
                            files: report.files,
                            bytes: report.bytes,
                            current: fname,
                        });
                    }
                }
            }
        }
    }

    if report.cancelled {
        let _ = fs::remove_dir_all(&part);
        return Ok(report);
    }

    let manifest = Manifest {
        format: 1,
        plan: plan.name.clone(),
        started_at: now,
        ended_at: now,
        sources: plan
            .sources
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect(),
        files: entries,
    };
    let text = serde_json::to_string(&manifest).map_err(|e| e.to_string())?;
    if let Err(e) = fs::write(part.join(MANIFEST), text) {
        let _ = fs::remove_dir_all(&part);
        return Err(format!("The backup couldn't be finished: {e}"));
    }
    if let Err(e) = fs::rename(&part, home.join(&name)) {
        let _ = fs::remove_dir_all(&part);
        return Err(format!("The backup couldn't be finished: {e}"));
    }
    progress(Progress {
        files: report.files,
        bytes: report.bytes,
        current: String::new(),
    });
    report.removed_versions = prune(&home, plan.keep as usize);
    Ok(report)
}

/// Removes the oldest versions beyond `keep`. Only folders in the plan's own folder that hold a
/// manifest are ever touched.
fn prune(home: &Path, keep: usize) -> Vec<String> {
    let versions = restore::list_versions(home);
    let mut gone = Vec::new();
    for v in versions.into_iter().skip(keep.max(1)) {
        let dir = home.join(&v.name);
        if dir.parent() != Some(home) || !dir.join(MANIFEST).is_file() {
            continue;
        }
        if remove_tree(&dir).is_ok() {
            gone.push(v.name);
        }
    }
    gone
}

/// Deletes a folder, clearing the read-only flag first where Windows needs it.
fn remove_tree(dir: &Path) -> std::io::Result<()> {
    if fs::remove_dir_all(dir).is_ok() {
        return Ok(());
    }
    fn unlock(d: &Path) {
        if let Ok(rd) = fs::read_dir(d) {
            for e in rd.flatten() {
                let p = e.path();
                if let Ok(md) = e.metadata() {
                    let mut perm = md.permissions();
                    if perm.readonly() {
                        #[allow(clippy::permissions_set_readonly_false)]
                        perm.set_readonly(false);
                        let _ = fs::set_permissions(&p, perm);
                    }
                    if md.is_dir() && !md.file_type().is_symlink() {
                        unlock(&p);
                    }
                }
            }
        }
    }
    unlock(dir);
    fs::remove_dir_all(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_names_are_dates_that_sort() {
        assert_eq!(version_name(0), "1970-01-01_00-00-00");
        assert_eq!(version_name(1_794_981_417), "2026-11-18_05-56-57");
        assert!(version_name(1_700_000_000) < version_name(1_800_000_000));
        // A leap day.
        assert_eq!(version_name(1_709_164_800), "2024-02-29_00-00-00");
    }
}
