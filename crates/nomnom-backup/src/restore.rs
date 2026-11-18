use crate::run::{Manifest, Progress};
use crate::MANIFEST;
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VersionInfo {
    pub name: String,
    pub started_at: i64,
    pub files: u64,
    pub bytes: u64,
}

pub fn read_manifest(home: &Path, version: &str) -> Result<Manifest, String> {
    if version.contains(['/', '\\']) || version.starts_with('.') || version.is_empty() {
        return Err("That isn't a backup version.".into());
    }
    let text = fs::read_to_string(home.join(version).join(MANIFEST))
        .map_err(|_| "That backup version can't be read.".to_string())?;
    serde_json::from_str(&text).map_err(|_| "That backup version is damaged.".to_string())
}

/// The versions in a plan's folder, newest first. A folder without a good manifest isn't a version.
pub fn list_versions(home: &Path) -> Vec<VersionInfo> {
    let Ok(rd) = fs::read_dir(home) else {
        return Vec::new();
    };
    let mut v: Vec<VersionInfo> = rd
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            let m = read_manifest(home, &name).ok()?;
            Some(VersionInfo {
                name,
                started_at: m.started_at,
                files: m.files.len() as u64,
                bytes: m.files.iter().map(|f| f.s).sum(),
            })
        })
        .collect();
    v.sort_by(|a, b| b.name.cmp(&a.name));
    v
}

/// A path from a manifest, checked to stay inside the version and the folder it is restored to.
fn safe_rel(rel: &str) -> Option<PathBuf> {
    if rel.is_empty() || rel.contains(':') || rel.starts_with(['/', '\\']) {
        return None;
    }
    let p = PathBuf::from(rel.replace('\\', "/"));
    let ok = p.components().all(|c| matches!(c, Component::Normal(_)));
    ok.then_some(p)
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RestoreReport {
    pub restored: u64,
    pub bytes: u64,
    /// Files that were put back under a new name because the old name was taken.
    pub renamed: u64,
    pub failed: Vec<String>,
    pub cancelled: bool,
}

/// A name that isn't taken, adding "(restored)" before the extension.
fn free_name(dst: &Path) -> PathBuf {
    if !dst.exists() {
        return dst.to_path_buf();
    }
    let stem = dst
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = dst
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let dir = dst.parent().unwrap_or(Path::new("."));
    for n in 1..10_000 {
        let tag = if n == 1 {
            "restored".to_string()
        } else {
            format!("restored {n}")
        };
        let cand = dir.join(format!("{stem} ({tag}){ext}"));
        if !cand.exists() {
            return cand;
        }
    }
    dst.to_path_buf()
}

/// Puts files of a version into `target`, keeping their folders. Nothing that is already there is
/// replaced: a taken name gets "(restored)" added. `only` lists files, or folders ending in `/`;
/// `None` restores everything.
pub fn restore(
    home: &Path,
    version: &str,
    only: Option<&[String]>,
    target: &Path,
    cancel: &AtomicBool,
    progress: &dyn Fn(Progress),
) -> Result<RestoreReport, String> {
    let m = read_manifest(home, version)?;
    fs::create_dir_all(target)
        .map_err(|e| format!("The folder to restore into couldn't be made: {e}"))?;
    let wants = |p: &str| match only {
        None => true,
        Some(list) => list.iter().any(|w| {
            if let Some(dir) = w.strip_suffix('/') {
                p == dir || p.starts_with(&format!("{dir}/"))
            } else {
                p == w
            }
        }),
    };
    let mut rep = RestoreReport::default();
    for e in m.files.iter().filter(|e| wants(&e.p)) {
        if cancel.load(Ordering::Relaxed) {
            rep.cancelled = true;
            break;
        }
        let (Some(rel), src) = (
            safe_rel(&e.p),
            home.join(version).join(e.p.replace('\\', "/")),
        ) else {
            rep.failed.push(e.p.clone());
            continue;
        };
        let dst = target.join(&rel);
        if let Some(parent) = dst.parent() {
            if fs::create_dir_all(parent).is_err() {
                rep.failed.push(e.p.clone());
                continue;
            }
        }
        let final_dst = free_name(&dst);
        let renamed = final_dst != dst;
        match fs::copy(&src, &final_dst) {
            Ok(n) => {
                if let Ok(f) = fs::OpenOptions::new().write(true).open(&final_dst) {
                    let _ = f.set_modified(
                        std::time::UNIX_EPOCH + std::time::Duration::from_secs(e.m.max(0) as u64),
                    );
                }
                rep.restored += 1;
                rep.bytes += n;
                if renamed {
                    rep.renamed += 1;
                }
                if rep.restored.is_multiple_of(50) {
                    progress(Progress {
                        files: rep.restored,
                        bytes: rep.bytes,
                        current: e.p.clone(),
                    });
                }
            }
            Err(_) => rep.failed.push(e.p.clone()),
        }
    }
    progress(Progress {
        files: rep.restored,
        bytes: rep.bytes,
        current: String::new(),
    });
    Ok(rep)
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct VerifyReport {
    pub checked: u64,
    pub missing: Vec<String>,
    pub wrong_size: Vec<String>,
}

/// Checks that every file listed for a version is there and has the recorded size.
pub fn verify(home: &Path, version: &str) -> Result<VerifyReport, String> {
    let m = read_manifest(home, version)?;
    let mut r = VerifyReport::default();
    for e in &m.files {
        r.checked += 1;
        match safe_rel(&e.p)
            .map(|p| home.join(version).join(p))
            .and_then(|p| fs::metadata(p).ok())
        {
            None => {
                if r.missing.len() < 200 {
                    r.missing.push(e.p.clone());
                }
            }
            Some(md) if md.len() != e.s => {
                if r.wrong_size.len() < 200 {
                    r.wrong_size.push(e.p.clone());
                }
            }
            Some(_) => {}
        }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run, Plan};

    fn put(p: &Path, text: &str) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn setup() -> (tempfile::TempDir, tempfile::TempDir, Plan) {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let plan = Plan {
            id: "p1".into(),
            name: "Docs".into(),
            sources: vec![src.path().to_path_buf()],
            destination: dst.path().to_path_buf(),
            keep: 3,
            exclude: "*.tmp, skipme".into(),
            ..Plan::default()
        };
        (src, dst, plan)
    }

    fn go(plan: &Plan, t: i64) -> run::RunReport {
        run(plan, t, &AtomicBool::new(false), &|_| {}).unwrap()
    }

    #[test]
    fn first_backup_copies_everything_except_what_is_left_out() {
        let (src, _dst, plan) = setup();
        put(&src.path().join("a.txt"), "alpha");
        put(&src.path().join("sub/b.txt"), "bravo");
        put(&src.path().join("junk.tmp"), "x");
        put(&src.path().join("skipme/c.txt"), "x");
        let r = go(&plan, 1_000_000);
        assert_eq!((r.files, r.linked), (2, 0));
        let v = list_versions(&plan.home());
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].files, v[0].bytes), (2, 10));
        let m = read_manifest(&plan.home(), &r.version).unwrap();
        let mut names: Vec<_> = m.files.iter().map(|f| f.p.clone()).collect();
        names.sort();
        let top = src
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(names, [format!("{top}/a.txt"), format!("{top}/sub/b.txt")]);
    }

    #[test]
    fn a_second_backup_links_unchanged_files_and_copies_only_changes() {
        let (src, _dst, plan) = setup();
        put(&src.path().join("same.txt"), "unchanged");
        put(&src.path().join("edit.txt"), "old text");
        let first = go(&plan, 1_000_000);
        std::thread::sleep(std::time::Duration::from_millis(50));
        put(&src.path().join("edit.txt"), "a different, longer text");
        put(&src.path().join("new.txt"), "fresh");
        let second = go(&plan, 1_000_100);
        assert_eq!(second.files, 3);
        assert_eq!(second.linked, 1, "only the unchanged file is linked");
        assert_eq!(
            second.copied_bytes,
            ("a different, longer text".len() + "fresh".len()) as u64
        );
        // The older version still has the old text.
        let top = src
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let old = fs::read_to_string(plan.home().join(&first.version).join(&top).join("edit.txt"))
            .unwrap();
        assert_eq!(old, "old text");
        assert_eq!(list_versions(&plan.home()).len(), 2);
    }

    #[test]
    fn only_the_newest_versions_are_kept() {
        let (src, _dst, plan) = setup();
        put(&src.path().join("a.txt"), "a");
        let mut names = Vec::new();
        for i in 0..5 {
            names.push(go(&plan, 1_000_000 + i * 100).version);
        }
        let kept: Vec<String> = list_versions(&plan.home())
            .into_iter()
            .map(|v| v.name)
            .collect();
        assert_eq!(kept, [names[4].clone(), names[3].clone(), names[2].clone()]);
        // The files of the kept versions are all still there.
        for v in &kept {
            assert!(verify(&plan.home(), v).unwrap().missing.is_empty());
        }
    }

    #[test]
    fn restore_puts_files_back_without_replacing_anything() {
        let (src, _dst, plan) = setup();
        put(&src.path().join("a.txt"), "original");
        put(&src.path().join("sub/b.txt"), "bravo");
        let r = go(&plan, 1_000_000);
        let out = tempfile::tempdir().unwrap();
        let top = src
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        // Something is already at the place of a.txt.
        put(&out.path().join(&top).join("a.txt"), "mine");
        let rep = restore(
            &plan.home(),
            &r.version,
            None,
            out.path(),
            &AtomicBool::new(false),
            &|_| {},
        )
        .unwrap();
        assert_eq!((rep.restored, rep.renamed, rep.failed.len()), (2, 1, 0));
        assert_eq!(
            fs::read_to_string(out.path().join(&top).join("a.txt")).unwrap(),
            "mine"
        );
        assert_eq!(
            fs::read_to_string(out.path().join(&top).join("a (restored).txt")).unwrap(),
            "original"
        );
        assert_eq!(
            fs::read_to_string(out.path().join(&top).join("sub/b.txt")).unwrap(),
            "bravo"
        );
    }

    #[test]
    fn restore_can_take_one_file_or_one_folder() {
        let (src, _dst, plan) = setup();
        put(&src.path().join("a.txt"), "a");
        put(&src.path().join("sub/b.txt"), "b");
        put(&src.path().join("sub/deep/c.txt"), "c");
        let r = go(&plan, 1_000_000);
        let top = src
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let out = tempfile::tempdir().unwrap();
        let only = [format!("{top}/sub/")];
        let rep = restore(
            &plan.home(),
            &r.version,
            Some(&only),
            out.path(),
            &AtomicBool::new(false),
            &|_| {},
        )
        .unwrap();
        assert_eq!(rep.restored, 2);
        assert!(!out.path().join(&top).join("a.txt").exists());
        let one = [format!("{top}/a.txt")];
        let out2 = tempfile::tempdir().unwrap();
        assert_eq!(
            restore(
                &plan.home(),
                &r.version,
                Some(&one),
                out2.path(),
                &AtomicBool::new(false),
                &|_| {}
            )
            .unwrap()
            .restored,
            1
        );
    }

    #[test]
    fn a_tampered_manifest_cannot_write_outside_the_target() {
        let (src, _dst, plan) = setup();
        put(&src.path().join("a.txt"), "a");
        let r = go(&plan, 1_000_000);
        let mpath = plan.home().join(&r.version).join(MANIFEST);
        let mut m: Manifest = serde_json::from_str(&fs::read_to_string(&mpath).unwrap()).unwrap();
        m.files[0].p = "../../escape.txt".into();
        fs::write(&mpath, serde_json::to_string(&m).unwrap()).unwrap();
        let out = tempfile::tempdir().unwrap();
        let rep = restore(
            &plan.home(),
            &r.version,
            None,
            out.path(),
            &AtomicBool::new(false),
            &|_| {},
        )
        .unwrap();
        assert_eq!((rep.restored, rep.failed.len()), (0, 1));
        assert!(!out.path().parent().unwrap().join("escape.txt").exists());
    }

    #[test]
    fn cancelling_leaves_no_half_made_version() {
        let (src, _dst, plan) = setup();
        for i in 0..20 {
            put(&src.path().join(format!("f{i}.txt")), "x");
        }
        let stop = AtomicBool::new(true);
        let r = run(&plan, 1_000_000, &stop, &|_| {}).unwrap();
        assert!(r.cancelled);
        assert!(list_versions(&plan.home()).is_empty());
        let leftovers: Vec<_> = fs::read_dir(plan.home())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(".partial"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn two_plans_cannot_share_one_folder() {
        let (src, _dst, plan) = setup();
        put(&src.path().join("a.txt"), "a");
        go(&plan, 1_000_000);
        let mut other = plan.clone();
        other.id = "p2".into();
        let e = run(&other, 1_000_100, &AtomicBool::new(false), &|_| {}).unwrap_err();
        assert!(e.contains("already uses"), "{e}");
    }

    #[test]
    fn verify_finds_a_damaged_version() {
        let (src, _dst, plan) = setup();
        put(&src.path().join("a.txt"), "hello");
        let r = go(&plan, 1_000_000);
        let top = src
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(verify(&plan.home(), &r.version).unwrap().checked, 1);
        // Break the stored copy.
        let file = plan.home().join(&r.version).join(&top).join("a.txt");
        let mut perm = fs::metadata(&file).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        let _ = fs::set_permissions(&file, perm);
        fs::remove_file(&file).unwrap();
        let v = verify(&plan.home(), &r.version).unwrap();
        assert_eq!(v.missing.len(), 1);
    }
}
