use nomnom_storage::dupes::{self, Stage};
use nomnom_storage::ops::{self, Kind, RunOptions, Status};
use nomnom_storage::protect::Protected;
use nomnom_storage::{ScanOptions, Session};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn put(path: &Path, size: usize, seed: u8) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let data: Vec<u8> = (0..size)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect();
    std::fs::write(path, data).unwrap();
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

fn wait(job: &dupes::Job) {
    let t = Instant::now();
    while !job.is_finished() {
        assert!(
            t.elapsed() < Duration::from_secs(60),
            "the search never finished"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn find_in(
    s: &Arc<Session>,
    min: u64,
) -> (Arc<dupes::Job>, &std::sync::RwLock<nomnom_storage::Index>) {
    let job = dupes::start(s.clone(), 0, min, 2);
    wait(&job);
    (job, &s.index)
}

#[test]
fn identical_files_are_grouped_and_lookalikes_are_not() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("a/photo.bin"), 300_000, 1);
    put(&r.join("b/photo copy.bin"), 300_000, 1);
    put(&r.join("c/again.bin"), 300_000, 1);
    // Same size, same first and last 64 KB, different in the middle.
    put(&r.join("d/lookalike.bin"), 300_000, 1);
    let mut data = std::fs::read(r.join("d/lookalike.bin")).unwrap();
    data[150_000] ^= 0xFF;
    std::fs::write(r.join("d/lookalike.bin"), data).unwrap();
    put(&r.join("e/other.bin"), 300_000, 9); // same size, different everywhere
    put(&r.join("f/tiny1.bin"), 100, 5);
    put(&r.join("f/tiny2.bin"), 100, 5);
    let s = scan(r);
    let (job, _idx) = find_in(&s, 1000);
    assert_eq!(job.stage(), Stage::Done);
    let groups = job.groups(0, 10);
    assert_eq!(groups.len(), 1, "{groups:#?}");
    let g = &groups[0];
    assert_eq!(g.copies.len(), 3);
    assert_eq!(g.size, 300_000);
    assert_eq!(g.recoverable, 600_000);
    assert_eq!(g.sha256.len(), 64);
    assert!(g
        .copies
        .iter()
        .all(|c| !c.path.contains("lookalike") && !c.path.contains("other")));
}

#[test]
fn the_small_file_limit_is_respected() {
    let d = tempfile::tempdir().unwrap();
    put(&d.path().join("x/a.bin"), 500, 2);
    put(&d.path().join("y/b.bin"), 500, 2);
    let s = scan(d.path());
    assert_eq!(find_in(&s, 1000).0.groups(0, 5).len(), 0);
    let s2 = scan(d.path());
    assert_eq!(find_in(&s2, 100).0.groups(0, 5).len(), 1);
}

#[test]
fn two_names_of_one_file_are_not_copies() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("one/data.bin"), 200_000, 3);
    std::fs::create_dir_all(r.join("two")).unwrap();
    if std::fs::hard_link(r.join("one/data.bin"), r.join("two/link.bin")).is_err() {
        eprintln!("SKIPPED: hardlinks aren't supported here");
        return;
    }
    let s = scan(r);
    let (job, _) = find_in(&s, 1000);
    assert!(
        job.groups(0, 5).is_empty(),
        "a file and its hardlink share their data; nothing can be recovered"
    );

    // With a genuine copy as well, the group has two files and one of them has two names.
    put(&r.join("three/real copy.bin"), 200_000, 3);
    let s2 = scan(r);
    let (job2, _) = find_in(&s2, 1000);
    let g = job2.groups(0, 5);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].copies.len(), 2);
    assert_eq!(g[0].copies.iter().map(|c| c.names).max(), Some(2));
    assert_eq!(g[0].recoverable, 200_000);
}

#[test]
fn a_search_can_be_stopped() {
    let d = tempfile::tempdir().unwrap();
    for i in 0..30 {
        put(&d.path().join(format!("d{i}/f.bin")), 400_000, 7);
    }
    let s = scan(d.path());
    let job = dupes::start(s.clone(), 0, 1000, 1);
    job.cancel();
    wait(&job);
    assert_eq!(job.stage(), Stage::Cancelled);
}

#[test]
fn deleting_copies_keeps_one_and_compares_content_first() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("keep/song.bin"), 200_000, 4);
    put(&r.join("dup1/song.bin"), 200_000, 4);
    put(&r.join("dup2/song.bin"), 200_000, 4);
    let s = scan(r);
    let (job, idx) = find_in(&s, 1000);
    let g = &job.groups(0, 1)[0];
    let node_of = |needle: &str| {
        g.copies
            .iter()
            .find(|c| c.path.contains(needle))
            .unwrap()
            .node
    };
    let (keep, d1, d2) = (node_of("keep"), node_of("dup1"), node_of("dup2"));

    // One copy changes after the search: it must not be deleted.
    std::fs::write(r.join("dup2/song.bin"), vec![0u8; 200_000]).unwrap();
    let plan = {
        let i = idx.read().unwrap();
        ops::plan_duplicates(
            &i,
            Kind::Delete,
            &[(keep, vec![d1, d2])],
            &Protected::default(),
        )
    };
    // dup2 changed on disk since the scan, so the plan itself refuses it.
    assert_eq!(plan.items.len(), 1, "{:?}", plan.refused);
    assert_eq!(plan.refused.len(), 1);
    let report = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(report.count(Status::Done), 1);
    assert!(!r.join("dup1/song.bin").exists());
    assert!(r.join("keep/song.bin").exists());
    assert!(r.join("dup2/song.bin").exists());
}

#[test]
fn the_kept_copy_can_never_be_removed_and_a_changed_keep_stops_everything() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("a/f.bin"), 100_000, 6);
    put(&r.join("b/f.bin"), 100_000, 6);
    let s = scan(r);
    let (job, idx) = find_in(&s, 1000);
    let g = &job.groups(0, 1)[0];
    let (a, b) = (g.copies[0].node, g.copies[1].node);
    let i = idx.read().unwrap();

    // Asking to keep a and remove a: refused.
    let plan = ops::plan_duplicates(&i, Kind::Delete, &[(a, vec![a])], &Protected::default());
    assert!(plan.items.is_empty());

    // Keep a, remove b, then a is replaced by something else before the run.
    let plan = ops::plan_duplicates(&i, Kind::Delete, &[(a, vec![b])], &Protected::default());
    assert_eq!(plan.items.len(), 1);
    let keep_path = plan.items[0].keep.as_ref().unwrap().path.clone();
    std::fs::remove_file(&keep_path).unwrap();
    put(&keep_path, 100_000, 99);
    let report = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(report.count(Status::Skipped), 1, "{:?}", report.results);
    assert!(
        std::fs::metadata(r.join("a/f.bin")).is_ok()
            && std::fs::metadata(r.join("b/f.bin")).is_ok(),
        "nothing was deleted"
    );
}

#[test]
fn a_hardlink_of_the_kept_file_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("a/f.bin"), 100_000, 8);
    put(&r.join("c/f.bin"), 100_000, 8);
    std::fs::create_dir_all(r.join("b")).unwrap();
    if std::fs::hard_link(r.join("a/f.bin"), r.join("b/link.bin")).is_err() {
        eprintln!("SKIPPED: hardlinks aren't supported here");
        return;
    }
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let node = |p: &str| idx.resolve_path(&r.join(p)).unwrap();
    let plan = ops::plan_duplicates(
        &idx,
        Kind::Delete,
        &[(node("a/f.bin"), vec![node("b/link.bin"), node("c/f.bin")])],
        &Protected::default(),
    );
    assert_eq!(plan.items.len(), 1, "{:?}", plan.refused);
    assert!(plan.items[0].path.ends_with("c\\f.bin"));
    assert!(plan.refused[0].reason.contains("same file"));
}

#[test]
fn content_changed_with_the_same_size_and_date_is_still_caught_at_run_time() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("keep/f.bin"), 120_000, 2);
    put(&r.join("copy/f.bin"), 120_000, 2);
    let s = scan(r);
    let (job, idx) = find_in(&s, 1000);
    let g = &job.groups(0, 1)[0];
    let keep = g
        .copies
        .iter()
        .find(|c| c.path.contains("keep"))
        .unwrap()
        .node;
    let copy = g
        .copies
        .iter()
        .find(|c| c.path.contains("copy"))
        .unwrap()
        .node;
    let i = idx.read().unwrap();
    let plan = ops::plan_duplicates(
        &i,
        Kind::Delete,
        &[(keep, vec![copy])],
        &Protected::default(),
    );
    assert_eq!(plan.items.len(), 1);
    // Rewrite the copy in place with different bytes, then put its modified time back.
    let path = r.join("copy/f.bin");
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    std::io::Write::write_all(&mut &f, &vec![0xEEu8; 120_000]).unwrap();
    f.set_modified(before).unwrap();
    drop(f);
    let report = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(report.count(Status::Skipped), 1, "{:?}", report.results);
    assert!(report.results[0]
        .message
        .as_ref()
        .unwrap()
        .contains("not identical"));
    assert!(path.exists() && r.join("keep/f.bin").exists());
}
