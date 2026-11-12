//! Scans throwaway folders and checks the numbers against what was put there.

use nomnom_storage::analyze::{self, Filter, FolderMode, SortKey};
use nomnom_storage::index::{F_DIR, F_HARDLINK_DUP, F_LINK};
use nomnom_storage::scan::round_up;
use nomnom_storage::{drives, ScanOptions, Session, State};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn write(path: &Path, bytes: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![7u8; bytes]).unwrap();
}

fn scan(root: &Path) -> Arc<Session> {
    let s = Session::start(
        1,
        root.to_path_buf(),
        ScanOptions {
            cluster: 4096,
            ..ScanOptions::default()
        },
    );
    wait(&s);
    s
}

fn wait(s: &Session) {
    let t = Instant::now();
    while !s.is_finished() {
        assert!(t.elapsed() < Duration::from_secs(60), "scan didn't finish");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn find(s: &Session, rel: &str) -> u32 {
    let idx = s.index.read().unwrap();
    let mut cur = idx.root();
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        cur = idx
            .children(cur)
            .find(|c| idx.name(*c) == part)
            .unwrap_or_else(|| panic!("{rel}: no {part}"));
    }
    cur
}

#[test]
fn totals_are_recursive_and_exact() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("a.bin"), 1000);
    write(&r.join("sub/b.bin"), 5000);
    write(&r.join("sub/deeper/c.bin"), 20_000);
    write(&r.join("sub/deeper/evenmore/d.txt"), 300);
    std::fs::create_dir_all(r.join("empty")).unwrap();
    let s = scan(r);
    assert_eq!(s.state(), State::Done);
    let idx = s.index.read().unwrap();
    let root = idx.node(idx.root()).unwrap();
    assert_eq!(root.size, 26_300);
    assert_eq!(root.files, 4);
    assert_eq!(root.dirs, 4); // sub, deeper, evenmore, empty
    assert_eq!(root.own_size, 1000);
    let sub = find_in(&idx, "sub");
    assert_eq!(idx.node(sub).unwrap().size, 25_300);
    assert_eq!(idx.node(sub).unwrap().own_size, 5000);
    assert_eq!(idx.node(find_in(&idx, "sub/deeper")).unwrap().files, 2);
    // Allocated space rounds each file up to a 4 KB cluster.
    let expect: u64 = [1000u64, 5000, 20_000, 300]
        .iter()
        .map(|n| round_up(*n, 4096))
        .sum();
    assert_eq!(root.alloc, expect);
    assert_eq!(s.stats.files.load(std::sync::atomic::Ordering::Relaxed), 4);
}

fn find_in(idx: &nomnom_storage::Index, rel: &str) -> u32 {
    let mut cur = idx.root();
    for part in rel.split('/') {
        cur = idx.children(cur).find(|c| idx.name(*c) == part).unwrap();
    }
    cur
}

#[test]
fn very_deep_paths_beyond_260_characters_are_scanned() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = dir.path().to_path_buf();
    for _ in 0..12 {
        p.push("a_folder_with_a_fairly_long_name");
    }
    let long = nomnom_storage::scan::long_os_path(&p);
    std::fs::create_dir_all(&long).unwrap();
    std::fs::write(long.join("deep.bin"), vec![1u8; 4242]).unwrap();
    assert!(p.to_string_lossy().len() > 260);
    let s = scan(dir.path());
    let idx = s.index.read().unwrap();
    assert_eq!(idx.node(idx.root()).unwrap().size, 4242);
    assert_eq!(idx.node(idx.root()).unwrap().dirs, 12);
}

#[test]
fn global_ranking_matches_a_brute_force_check() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    let mut sizes = Vec::new();
    for i in 0..40usize {
        let size = 100 + (i * 7919) % 5000;
        sizes.push(size as u64);
        write(&r.join(format!("d{}/e{}/f{i}.dat", i % 5, i % 3)), size);
    }
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let top = analyze::top_files(&idx, idx.root(), 10, &Filter::default());
    sizes.sort_by(|a, b| b.cmp(a));
    let got: Vec<u64> = top.iter().map(|v| v.size).collect();
    assert_eq!(got, sizes[..10].to_vec());
    // A size filter and an extension filter narrow the same list.
    let f = Filter {
        min_size: Some(4000),
        extensions: vec!["dat".into()],
        ..Filter::default()
    };
    let big = analyze::top_files(&idx, idx.root(), 1000, &f);
    assert_eq!(big.len(), sizes.iter().filter(|s| **s >= 4000).count());
}

#[test]
fn inclusive_folders_overlap_and_exclusive_folders_do_not() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("big/inner/x.bin"), 9000);
    write(&r.join("big/y.bin"), 1000);
    write(&r.join("other/z.bin"), 3000);
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let inc = analyze::top_folders(&idx, idx.root(), 10, FolderMode::Inclusive, 0);
    let names: Vec<_> = inc.iter().map(|v| (v.name.clone(), v.size)).collect();
    assert_eq!(names[1], ("big".to_string(), 10_000));
    assert!(names.contains(&("inner".to_string(), 9000)), "{names:?}");
    let exc = analyze::top_folders(&idx, idx.root(), 10, FolderMode::Exclusive, 0);
    let total: u64 = exc.iter().map(|v| v.size).sum();
    assert_eq!(
        total, 13_000,
        "exclusive sizes add up to the whole, nothing counted twice"
    );
}

#[test]
fn a_junction_loop_is_recorded_but_never_entered() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("real/data.bin"), 2000);
    let outside = tempfile::tempdir().unwrap();
    write(&outside.path().join("huge.bin"), 50_000);
    if junction::create(outside.path(), r.join("real/to_outside")).is_err()
        || junction::create(r, r.join("real/to_root")).is_err()
    {
        eprintln!("SKIPPED: this account can't create junctions");
        return;
    }
    let s = scan(r);
    let idx = s.index.read().unwrap();
    assert_eq!(
        idx.node(idx.root()).unwrap().size,
        2000,
        "nothing behind a junction is counted"
    );
    let link = find_in(&idx, "real/to_outside");
    let n = idx.node(link).unwrap();
    assert!(n.flags & F_LINK != 0 && n.flags & F_DIR != 0);
    assert_eq!(n.size, 0);
    assert!(idx.children(link).next().is_none());
}

#[test]
fn a_hardlink_is_counted_once_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("one/original.bin"), 2 << 20);
    std::fs::create_dir_all(r.join("two")).unwrap();
    if std::fs::hard_link(r.join("one/original.bin"), r.join("two/link.bin")).is_err() {
        eprintln!("SKIPPED: hardlinks aren't supported here");
        return;
    }
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let root = idx.node(idx.root()).unwrap();
    assert_eq!(root.size, 4 << 20, "both names are 2 MB of logical size");
    assert_eq!(root.alloc, 2 << 20, "but the data is on disk once");
    let flagged = idx
        .nodes()
        .iter()
        .filter(|n| n.flags & F_HARDLINK_DUP != 0)
        .count();
    assert_eq!(flagged, 1);
}

#[test]
fn children_are_sorted_and_paged() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("b.bin"), 300);
    write(&r.join("a.bin"), 100);
    write(&r.join("c.bin"), 200);
    write(&r.join("dir/x.bin"), 50);
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let by_size = analyze::children_page(&idx, idx.root(), SortKey::Size, true, 0, 10, None);
    let names: Vec<_> = by_size.items.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["b.bin", "c.bin", "a.bin", "dir"]);
    let by_name = analyze::children_page(&idx, idx.root(), SortKey::Name, false, 1, 2, None);
    assert_eq!(by_name.total, 4);
    assert_eq!(
        by_name
            .items
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        ["b.bin", "c.bin"]
    );
    let found = analyze::children_page(&idx, idx.root(), SortKey::Name, false, 0, 10, Some("DIR"));
    assert_eq!(found.total, 1);
    assert!((by_size.items[0].percent - 300.0 / 650.0 * 100.0).abs() < 0.01);
}

#[test]
fn rescanning_one_folder_corrects_the_totals_above_it() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("keep/k.bin"), 1000);
    write(&r.join("change/a.bin"), 4000);
    write(&r.join("change/b.bin"), 6000);
    let s = scan(r);
    std::fs::remove_file(r.join("change/b.bin")).unwrap();
    write(&r.join("change/new/c.bin"), 500);
    let node = find(&s, "change");
    assert!(s.rescan(node));
    wait(&s);
    let idx = s.index.read().unwrap();
    assert_eq!(idx.node(node).unwrap().size, 4500);
    assert_eq!(idx.node(idx.root()).unwrap().size, 5500);
    assert_eq!(idx.node(idx.root()).unwrap().files, 3);
    assert_eq!(idx.node(idx.root()).unwrap().dirs, 3);
}

#[test]
fn a_scan_can_be_paused_resumed_and_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..200 {
        write(&dir.path().join(format!("d{}/f{i}.bin", i % 20)), 10);
    }
    let s = Session::start(
        2,
        dir.path().to_path_buf(),
        ScanOptions {
            workers: 1,
            ..ScanOptions::default()
        },
    );
    s.pause();
    std::thread::sleep(Duration::from_millis(200));
    let during = s.stats.files.load(std::sync::atomic::Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        s.stats.files.load(std::sync::atomic::Ordering::Relaxed),
        during,
        "no progress while paused"
    );
    if s.state() == State::Paused {
        s.resume();
        assert_eq!(s.state(), State::Running);
    }
    wait(&s);
    assert_eq!(s.state(), State::Done);
    assert_eq!(
        s.stats.files.load(std::sync::atomic::Ordering::Relaxed),
        200
    );

    let again = Session::start(
        3,
        dir.path().to_path_buf(),
        ScanOptions {
            workers: 1,
            ..ScanOptions::default()
        },
    );
    again.pause();
    again.cancel();
    wait(&again);
    assert_eq!(again.state(), State::Cancelled);
}

#[test]
fn a_missing_folder_is_reported_not_crashed_on() {
    let s = scan(&PathBuf::from(r"C:\this\folder\does\not\exist\nomnom"));
    assert_eq!(s.state(), State::Done);
    assert_eq!(s.stats.errors.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(s.errors().len(), 1);
}

#[test]
fn extension_and_category_totals_add_up() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(&r.join("a.mp4"), 3000);
    write(&r.join("b.MP4"), 2000);
    write(&r.join("c.png"), 500);
    write(&r.join("noext"), 70);
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let ext = analyze::by_extension(&idx, idx.root(), 10);
    assert_eq!(
        (ext[0].key.as_str(), ext[0].size, ext[0].files),
        ("mp4", 5000, 2)
    );
    let cat = analyze::by_category(&idx, idx.root());
    assert_eq!(cat[0].key, "video");
    assert_eq!(cat.iter().map(|g| g.size).sum::<u64>(), 5570);
}

#[test]
fn the_system_drive_is_listed_with_real_numbers() {
    let all = drives::list();
    let c = all
        .iter()
        .find(|d| d.letter == "C")
        .expect("C: should exist");
    assert!(c.total > 0 && c.free <= c.total && c.used == c.total - c.free);
    assert_eq!(c.filesystem, "NTFS");
    assert!(c.cluster >= 512);
}
