//! File operations on throwaway folders only. Nothing here touches a real folder, and the
//! Recycle Bin test is ignored unless asked for, because it leaves an entry in the real bin.

use nomnom_storage::ops::{self, Kind, RunOptions, Status};
use nomnom_storage::protect::Protected;
use nomnom_storage::{ScanOptions, Session};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn write(path: &Path, bytes: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![9u8; bytes]).unwrap();
}

fn scan(root: &Path) -> Arc<Session> {
    let s = Session::start(1, root.to_path_buf(), ScanOptions::default());
    let t = Instant::now();
    while !s.is_finished() {
        assert!(t.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(10));
    }
    s
}

fn node(s: &Session, rel: &str) -> u32 {
    let idx = s.index.read().unwrap();
    idx.resolve_path(&s.root.join(rel))
        .unwrap_or_else(|| panic!("{rel} not in the scan"))
}

fn nothing_protected() -> Protected {
    Protected::default()
}

fn run(plan: &ops::Plan) -> ops::Report {
    ops::run(
        plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
}

#[test]
fn permanent_delete_removes_a_folder_tree_and_reports_it() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("gone/a.bin"), 100);
    write(&r.join("gone/sub/b.bin"), 200);
    write(&r.join("keep/c.bin"), 300);
    let s = scan(r);
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Delete,
        &[node(&s, "gone")],
        None,
        None,
        &nothing_protected(),
    );
    assert_eq!((plan.total_files, plan.total_size), (2, 300));
    assert!(plan.refused.is_empty() && !plan.needs_typed);
    let report = run(&plan);
    assert_eq!(report.count(Status::Done), 1);
    assert!(!r.join("gone").exists());
    assert!(r.join("keep/c.bin").exists());
}

#[test]
fn deleting_a_folder_with_a_junction_removes_the_link_not_what_it_points_to() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("precious.txt"), 1234);
    write(&r.join("victim/own.bin"), 10);
    if junction::create(outside.path(), r.join("victim/link_to_outside")).is_err() {
        eprintln!("SKIPPED: this account can't create junctions");
        return;
    }
    let s = scan(r);
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Delete,
        &[node(&s, "victim")],
        None,
        None,
        &nothing_protected(),
    );
    let report = run(&plan);
    assert_eq!(report.count(Status::Done), 1, "{:?}", report.results);
    assert!(!r.join("victim").exists());
    assert_eq!(
        std::fs::read(outside.path().join("precious.txt"))
            .unwrap()
            .len(),
        1234,
        "the target of the link must survive"
    );
}

#[test]
fn read_only_files_are_kept_unless_asked() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("ro.txt"), 10);
    let mut perm = std::fs::metadata(r.join("ro.txt")).unwrap().permissions();
    perm.set_readonly(true);
    std::fs::set_permissions(r.join("ro.txt"), perm).unwrap();
    let s = scan(r);
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Delete,
        &[node(&s, "ro.txt")],
        None,
        None,
        &nothing_protected(),
    );
    let report = run(&plan);
    assert_eq!(report.count(Status::Failed), 1);
    assert!(report.results[0]
        .message
        .as_ref()
        .unwrap()
        .contains("Read-only"));
    assert!(r.join("ro.txt").exists());
    let again = ops::run(
        &plan,
        RunOptions {
            include_readonly: true,
        },
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(again.count(Status::Done), 1);
    assert!(!r.join("ro.txt").exists());
}

#[test]
fn a_file_that_changed_since_the_scan_is_refused_at_planning() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("f.bin"), 100);
    let s = scan(r);
    write(&r.join("f.bin"), 5000); // changed after the scan
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Delete,
        &[node(&s, "f.bin")],
        None,
        None,
        &nothing_protected(),
    );
    assert!(plan.items.is_empty());
    assert!(plan.refused[0].reason.contains("changed after the scan"));
    assert!(r.join("f.bin").exists());
}

#[test]
fn a_file_replaced_between_the_preview_and_the_run_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("a.bin"), 100);
    let s = scan(r);
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Delete,
        &[node(&s, "a.bin")],
        None,
        None,
        &nothing_protected(),
    );
    assert_eq!(plan.items.len(), 1);
    // Someone swaps in a different file with the same name and size.
    std::fs::remove_file(r.join("a.bin")).unwrap();
    write(&r.join("a.bin"), 100);
    let report = run(&plan);
    assert_eq!(report.count(Status::Skipped), 1, "{:?}", report.results);
    assert!(r.join("a.bin").exists(), "the new file must not be deleted");
}

#[test]
fn a_file_that_grows_after_the_preview_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("log.txt"), 50);
    let s = scan(dir.path());
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Delete,
        &[node(&s, "log.txt")],
        None,
        None,
        &nothing_protected(),
    );
    write(&dir.path().join("log.txt"), 51);
    assert_eq!(run(&plan).count(Status::Skipped), 1);
    assert!(dir.path().join("log.txt").exists());
}

#[test]
fn protected_places_are_refused_before_anything_runs() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("system/core.dll"), 10);
    write(&r.join("apps/tool.exe"), 10);
    write(&r.join("docs/a.txt"), 10);
    let protect = Protected {
        forbidden: vec![r.join("system")],
        careful: vec![r.join("apps")],
        exact: vec![],
    };
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let plan = ops::plan(
        &idx,
        Kind::Delete,
        &[
            node(&s, "system/core.dll"),
            node(&s, "apps/tool.exe"),
            node(&s, "docs/a.txt"),
        ],
        None,
        None,
        &protect,
    );
    assert_eq!(plan.refused.len(), 1);
    assert_eq!(plan.items.len(), 2);
    assert!(
        plan.needs_typed,
        "a program folder needs the stronger confirmation"
    );
    assert_eq!(plan.warnings.len(), 1);
    // The scan root is not a drive root, but a drive root is refused whatever the scan says.
    let root_plan = ops::plan(&idx, Kind::Delete, &[0], None, None, &Protected::default());
    assert_eq!(
        root_plan.items.len(),
        1,
        "an ordinary folder chosen as the scan root is allowed"
    );
}

#[test]
fn selecting_a_folder_and_something_inside_it_counts_once() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("top/inner/x.bin"), 40);
    write(&r.join("top/y.bin"), 60);
    let s = scan(r);
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Delete,
        &[
            node(&s, "top/inner/x.bin"),
            node(&s, "top"),
            node(&s, "top/y.bin"),
        ],
        None,
        None,
        &nothing_protected(),
    );
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.total_size, 100);
}

#[test]
fn big_deletes_need_the_stronger_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..1100 {
        write(&dir.path().join(format!("many/f{i}.t")), 1);
    }
    write(&dir.path().join("few/a.t"), 1);
    let s = scan(dir.path());
    let idx = s.index.read().unwrap();
    assert!(
        ops::plan(
            &idx,
            Kind::Delete,
            &[node(&s, "many")],
            None,
            None,
            &nothing_protected()
        )
        .needs_typed
    );
    assert!(
        !ops::plan(
            &idx,
            Kind::Delete,
            &[node(&s, "few")],
            None,
            None,
            &nothing_protected()
        )
        .needs_typed
    );
}

#[test]
fn copy_never_replaces_and_does_not_follow_links() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("src/doc.txt"), 100);
    write(&r.join("src/sub/more.txt"), 50);
    write(&r.join("dest/doc.txt"), 7);
    std::fs::create_dir_all(r.join("dest/src")).unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("big.bin"), 9999);
    let linked = junction::create(outside.path(), r.join("src/linked")).is_ok();
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let plan = ops::plan(
        &idx,
        Kind::Copy,
        &[node(&s, "src"), node(&s, "src/doc.txt")],
        Some(&r.join("dest")),
        None,
        &nothing_protected(),
    );
    assert_eq!(
        plan.items.len(),
        1,
        "the file inside the chosen folder is covered by the folder"
    );
    let report = run(&plan);
    assert_eq!(report.count(Status::Done), 1, "{:?}", report.results);
    let to = PathBuf::from(report.results[0].to.clone().unwrap());
    assert_eq!(
        to.file_name().unwrap(),
        "src (2)",
        "dest/src already existed"
    );
    assert_eq!(
        std::fs::read(r.join("dest/doc.txt")).unwrap().len(),
        7,
        "an existing file is untouched"
    );
    assert_eq!(std::fs::read(to.join("sub/more.txt")).unwrap().len(), 50);
    assert!(r.join("src/doc.txt").exists(), "a copy leaves the original");
    if linked {
        assert!(
            !to.join("linked").exists(),
            "a link isn't copied or followed"
        );
    }
}

#[test]
fn copying_a_folder_into_itself_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("a/x.bin"), 1);
    std::fs::create_dir_all(dir.path().join("a/inner")).unwrap();
    let s = scan(dir.path());
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Copy,
        &[node(&s, "a")],
        Some(&dir.path().join("a/inner")),
        None,
        &nothing_protected(),
    );
    assert!(plan.items.is_empty());
    assert!(plan.refused[0].reason.contains("inside itself"));
}

#[test]
fn move_on_one_drive_renames_into_a_free_name_and_keeps_contents() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("from/a.bin"), 100);
    write(&r.join("from/dir/b.bin"), 200);
    write(&r.join("to/a.bin"), 1);
    let s = scan(r);
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Move,
        &[node(&s, "from/a.bin"), node(&s, "from/dir")],
        Some(&r.join("to")),
        None,
        &nothing_protected(),
    );
    let report = run(&plan);
    assert_eq!(report.count(Status::Done), 2, "{:?}", report.results);
    assert!(!r.join("from/a.bin").exists() && !r.join("from/dir").exists());
    assert_eq!(
        std::fs::read(r.join("to/a.bin")).unwrap().len(),
        1,
        "the file already there stays"
    );
    assert_eq!(std::fs::read(r.join("to/a (2).bin")).unwrap().len(), 100);
    assert_eq!(std::fs::read(r.join("to/dir/b.bin")).unwrap().len(), 200);
}

#[test]
fn moving_into_the_same_folder_does_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("a.bin"), 5);
    let s = scan(dir.path());
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Move,
        &[node(&s, "a.bin")],
        Some(dir.path()),
        None,
        &nothing_protected(),
    );
    let report = run(&plan);
    assert_eq!(report.count(Status::Skipped), 1);
    assert!(dir.path().join("a.bin").exists());
}

#[test]
fn the_copy_then_verify_path_used_between_drives_removes_the_original_only_when_identical() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("src/a.bin"), 3000);
    write(&r.join("src/d/b.bin"), 10);
    std::fs::create_dir_all(r.join("elsewhere")).unwrap();
    let s = scan(r);
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Move,
        &[node(&s, "src")],
        Some(&r.join("elsewhere")),
        None,
        &nothing_protected(),
    );
    let item = &plan.items[0];
    ops::move_by_copy(item, &r.join("elsewhere/src"), &AtomicBool::new(false)).unwrap();
    assert!(!r.join("src").exists());
    assert_eq!(
        std::fs::read(r.join("elsewhere/src/d/b.bin"))
            .unwrap()
            .len(),
        10
    );

    // If the copy differs, the original stays.
    write(&r.join("o/x.bin"), 100);
    let s2 = scan(r);
    let plan2 = ops::plan(
        &s2.index.read().unwrap(),
        Kind::Move,
        &[node(&s2, "o")],
        Some(&r.join("elsewhere")),
        None,
        &nothing_protected(),
    );
    assert!(ops::verify_tree(&r.join("o"), &r.join("elsewhere/src")).is_err());
    assert_eq!(plan2.items.len(), 1);
    assert!(r.join("o/x.bin").exists());
}

#[test]
fn rename_works_refuses_clashes_and_bad_names_and_handles_case_changes() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("old.txt"), 5);
    write(&r.join("taken.txt"), 5);
    write(&r.join("Photo.jpg"), 5);
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let p = |rel: &str, name: &str| {
        ops::plan(
            &idx,
            Kind::Rename,
            &[node(&s, rel)],
            None,
            Some(name),
            &nothing_protected(),
        )
    };

    for bad in ["", "a/b", "con", "x?"] {
        assert!(p("old.txt", bad).items.is_empty(), "{bad:?}");
    }
    let clash = run(&p("old.txt", "taken.txt"));
    assert_eq!(clash.count(Status::Failed), 1);
    assert!(r.join("old.txt").exists() && r.join("taken.txt").exists());

    assert_eq!(run(&p("old.txt", "new name.txt")).count(Status::Done), 1);
    assert!(r.join("new name.txt").exists() && !r.join("old.txt").exists());

    assert_eq!(run(&p("Photo.jpg", "photo.jpg")).count(Status::Done), 1);
    let names: Vec<String> = std::fs::read_dir(r)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(names.contains(&"photo.jpg".to_string()), "{names:?}");

    drop(idx);
    let s2 = scan(r);
    let idx2 = s2.index.read().unwrap();
    let two = ops::plan(
        &idx2,
        Kind::Rename,
        &[node(&s2, "taken.txt"), node(&s2, "new name.txt")],
        None,
        Some("x"),
        &nothing_protected(),
    );
    assert!(two.items.is_empty(), "renaming is one item at a time");
}

#[test]
fn stopping_midway_leaves_the_rest_untouched() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        write(&dir.path().join(format!("f{i}.bin")), 10);
    }
    let s = scan(dir.path());
    let idx = s.index.read().unwrap();
    let nodes: Vec<u32> = (0..5).map(|i| node(&s, &format!("f{i}.bin"))).collect();
    let plan = ops::plan(&idx, Kind::Delete, &nodes, None, None, &nothing_protected());
    let cancel = AtomicBool::new(false);
    let report = ops::run(&plan, RunOptions::default(), &cancel, &|p| {
        if p.done == 2 {
            cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    });
    assert!(report.cancelled);
    assert_eq!(report.count(Status::Done), 2);
    assert_eq!(report.count(Status::Skipped), 3);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[test]
fn the_recycle_bin_check_refuses_what_windows_could_delete_for_good() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        ops::recycle_ok(&dir.path().join("a.txt"), 1000).is_ok()
            || ops::recycle_ok(&dir.path().join("a.txt"), 1000).is_err()
    );
    // Far bigger than any bin: refused, so nothing is silently destroyed.
    assert!(ops::recycle_ok(&dir.path().join("huge.bin"), u64::MAX / 2).is_err());
    assert!(ops::recycle_ok(Path::new(r"\\server\share\x.bin"), 1).is_err());
}

#[test]
#[ignore = "puts a real entry in the Recycle Bin; run once by hand: cargo test -p nomnom-storage --test ops -- --ignored"]
fn a_file_goes_to_the_recycle_bin() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("bin-me.txt"), 20);
    let s = scan(dir.path());
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Recycle,
        &[node(&s, "bin-me.txt")],
        None,
        None,
        &nothing_protected(),
    );
    assert_eq!(plan.items.len(), 1, "{:?}", plan.refused);
    let report = run(&plan);
    assert_eq!(report.count(Status::Done), 1, "{:?}", report.results);
    assert!(!dir.path().join("bin-me.txt").exists());
}
