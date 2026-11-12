//! Snapshots of throwaway trees: saving, loading, comparing and pruning.

use nomnom_core::db::Db;
use nomnom_storage::snapshot::{self, Change, NewSnapshot};
use nomnom_storage::{ScanOptions, Session};
use std::path::Path;
use std::time::{Duration, Instant};

fn write(path: &Path, bytes: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![3u8; bytes]).unwrap();
}

fn scan(root: &Path) -> std::sync::Arc<Session> {
    let s = Session::start(
        1,
        root.to_path_buf(),
        ScanOptions {
            cluster: 4096,
            ..ScanOptions::default()
        },
    );
    let t = Instant::now();
    while !s.is_finished() {
        assert!(t.elapsed() < Duration::from_secs(60), "scan didn't finish");
        std::thread::sleep(Duration::from_millis(10));
    }
    s
}

fn capture(root: &Path, depth: u32, big: u64) -> snapshot::Captured {
    let s = scan(root);
    let idx = s.index.read().unwrap();
    snapshot::capture(&idx, depth, big)
}

fn tree() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    write(&r.join("Videos/a.mp4"), 5000);
    write(&r.join("Videos/clips/b.mp4"), 2000);
    write(&r.join("Docs/x.txt"), 100);
    write(&r.join("Old/y.bin"), 700);
    write(&r.join("Deep/l1/l2/l3/z.bin"), 300);
    d
}

fn size_of(c: &snapshot::Captured, path: &str) -> Option<u64> {
    c.dirs.iter().find(|d| d.path == path).map(|d| d.size)
}

#[test]
fn capture_keeps_folders_to_the_depth_and_big_files_anywhere() {
    let d = tree();
    let c = capture(d.path(), 2, 1000);
    assert_eq!(size_of(&c, ""), Some(5000 + 2000 + 100 + 700 + 300));
    assert_eq!(size_of(&c, "Videos"), Some(7000));
    assert_eq!(size_of(&c, "Videos\\clips"), Some(2000));
    assert_eq!(size_of(&c, "Deep\\l1"), Some(300));
    assert_eq!(
        size_of(&c, "Deep\\l1\\l2"),
        None,
        "deeper than the snapshot goes"
    );
    let files: Vec<&str> = c.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(files, vec!["Videos\\a.mp4", "Videos\\clips\\b.mp4"]);
    assert_eq!(c.total_files, 5);
}

#[test]
fn saving_and_loading_gives_back_the_same_snapshot() {
    let d = tree();
    let c = capture(d.path(), 4, 1000);
    let db = Db::open_in_memory().unwrap();
    let id = db
        .with(|conn| {
            snapshot::save(
                conn,
                &NewSnapshot {
                    root: "C:\\Data",
                    label: "before",
                    source: "manual",
                    created_at: 100,
                    errors: 2,
                },
                &c,
            )
        })
        .unwrap();
    let back = db.with(|conn| snapshot::load(conn, id)).unwrap().unwrap();
    assert_eq!(back.dirs, c.dirs);
    assert_eq!(back.files, c.files);
    assert_eq!(
        (back.depth, back.big_min, back.total_bytes),
        (4, 1000, c.total_bytes)
    );
    let listed = db.with(|conn| snapshot::list(conn, "c:\\data")).unwrap();
    assert_eq!(
        listed.len(),
        1,
        "the folder name is matched without regard to letter case"
    );
    assert_eq!(
        (
            listed[0].label.as_str(),
            listed[0].errors,
            listed[0].created_at
        ),
        ("before", 2, 100)
    );
    assert!(db
        .with(|conn| snapshot::list(conn, "D:\\"))
        .unwrap()
        .is_empty());
}

#[test]
fn compare_finds_what_grew_shrank_appeared_and_vanished() {
    let d = tree();
    let r = d.path();
    let before = capture(r, 6, 1000);
    // Videos grows, Docs shrinks, Old goes away, New appears with a subfolder, a big file appears and one vanishes.
    write(&r.join("Videos/a.mp4"), 9000);
    write(&r.join("Docs/x.txt"), 10);
    std::fs::remove_dir_all(r.join("Old")).unwrap();
    write(&r.join("Fresh/sub/f.bin"), 4000);
    std::fs::remove_file(r.join("Videos/clips/b.mp4")).unwrap();
    let after = capture(r, 6, 1000);

    let c = snapshot::compare(&before, &after, 50);
    let find = |p: &str| c.dirs.iter().find(|x| x.path == p);
    assert_eq!(find("Videos").unwrap().change, Change::Grew);
    // a.mp4 5000 -> 9000, and clips lost its 2000: 7000 -> 9000
    assert_eq!(find("Videos").unwrap().delta, 2000);
    assert_eq!(find("Docs").unwrap().change, Change::Shrank);
    assert_eq!(find("Docs").unwrap().delta, -90);
    assert_eq!(find("Old").unwrap().change, Change::Gone);
    assert_eq!(find("Fresh").unwrap().change, Change::New);
    assert!(
        find("Fresh\\sub").is_none(),
        "a new folder is listed once, not for everything inside"
    );
    assert!(
        find("Videos\\clips").is_some(),
        "b.mp4 left, so the clips folder is now empty (smaller)"
    );
    assert_eq!(
        c.delta,
        after.total_bytes as i64 - before.total_bytes as i64
    );
    let big = |p: &str| c.files.iter().find(|x| x.path == p).map(|x| x.change);
    assert_eq!(big("Videos\\a.mp4"), Some(Change::Grew));
    assert_eq!(big("Videos\\clips\\b.mp4"), Some(Change::Gone));
    assert_eq!(
        big("Fresh\\sub\\f.bin"),
        None,
        "inside a new folder, so it isn't repeated"
    );
    // sorted by how much changed
    let deltas: Vec<u64> = c.dirs.iter().map(|x| x.delta.unsigned_abs()).collect();
    assert!(deltas.windows(2).all(|w| w[0] >= w[1]));
}

#[test]
fn comparing_an_unchanged_tree_finds_nothing() {
    let d = tree();
    let a = capture(d.path(), 6, 1000);
    let b = capture(d.path(), 6, 1000);
    let c = snapshot::compare(&a, &b, 50);
    assert_eq!((c.dirs_changed, c.files_changed, c.delta), (0, 0, 0));
}

#[test]
fn only_what_both_snapshots_kept_is_compared() {
    let d = tree();
    let shallow = capture(d.path(), 1, 1000);
    let deep = capture(d.path(), 6, 10);
    let c = snapshot::compare(&shallow, &deep, 50);
    assert_eq!(c.depth, 1);
    assert_eq!(c.big_min, 1000);
    // The deeper folders and the small files exist in one snapshot only, but they are not reported as new.
    assert_eq!((c.dirs_changed, c.files_changed), (0, 0));
}

#[test]
fn the_limit_caps_the_list_but_not_the_count() {
    let d = tree();
    let r = d.path();
    let before = capture(r, 6, 1000);
    for i in 0..5 {
        write(&r.join(format!("N{i}/f.bin")), 10 + i);
    }
    let after = capture(r, 6, 1000);
    let c = snapshot::compare(&before, &after, 2);
    assert_eq!(c.dirs.len(), 2);
    assert_eq!(c.dirs_changed, 5);
}

#[test]
fn prune_keeps_the_newest_scheduled_ones_and_never_touches_manual_ones() {
    let d = tree();
    let c = capture(d.path(), 3, 1000);
    let db = Db::open_in_memory().unwrap();
    let save = |source: &str, t: i64| {
        db.with(|conn| {
            snapshot::save(
                conn,
                &NewSnapshot {
                    root: "E:\\",
                    label: "",
                    source,
                    created_at: t,
                    errors: 0,
                },
                &c,
            )
        })
        .unwrap()
    };
    let manual = save("manual", 1);
    for t in 2..=6 {
        save("scheduled", t);
    }
    let removed = db
        .with(|conn| snapshot::prune(conn, "E:\\", "scheduled", 2))
        .unwrap();
    assert_eq!(removed, 3);
    let left = db.with(|conn| snapshot::list(conn, "E:\\")).unwrap();
    assert_eq!(
        left.iter().map(|m| m.created_at).collect::<Vec<_>>(),
        vec![1, 5, 6]
    );
    assert!(left.iter().any(|m| m.id == manual));
    // The rows of removed snapshots are gone too.
    let orphans: i64 = db
        .with(|conn| {
            conn.query_row(
                "SELECT count(*) FROM storage_snapshot_dirs WHERE snapshot_id NOT IN (SELECT id FROM storage_snapshots)",
                [],
                |r| r.get(0),
            )
        })
        .unwrap();
    assert_eq!(orphans, 0);
    assert_eq!(
        db.with(|conn| snapshot::last_time(conn, "E:\\", "scheduled"))
            .unwrap(),
        Some(6)
    );
    assert_eq!(
        db.with(|conn| snapshot::last_time(conn, "E:\\", "nothing"))
            .unwrap(),
        None
    );
}

#[test]
fn delete_removes_one_snapshot_and_trend_follows_a_folder() {
    let d = tree();
    let r = d.path();
    let db = Db::open_in_memory().unwrap();
    let mut ids = Vec::new();
    for (t, grow) in [(10, 0), (20, 1000), (30, 3000)] {
        write(&r.join("Videos/a.mp4"), 5000 + grow);
        let c = capture(r, 3, 1000);
        ids.push(
            db.with(|conn| {
                snapshot::save(
                    conn,
                    &NewSnapshot {
                        root: "F:\\",
                        label: "",
                        source: "manual",
                        created_at: t,
                        errors: 0,
                    },
                    &c,
                )
            })
            .unwrap(),
        );
    }
    let t = db
        .with(|conn| snapshot::trend(conn, "F:\\", "Videos"))
        .unwrap();
    assert_eq!(
        t.iter().map(|p| p.size).collect::<Vec<_>>(),
        vec![Some(7000), Some(8000), Some(10000)]
    );
    let none = db
        .with(|conn| snapshot::trend(conn, "F:\\", "Missing"))
        .unwrap();
    assert!(none.iter().all(|p| p.size.is_none()));
    assert!(db.with(|conn| snapshot::delete(conn, ids[1])).unwrap());
    assert!(!db.with(|conn| snapshot::delete(conn, ids[1])).unwrap());
    assert!(db
        .with(|conn| snapshot::load(conn, ids[1]))
        .unwrap()
        .is_none());
    assert_eq!(
        db.with(|conn| snapshot::list(conn, "F:\\")).unwrap().len(),
        2
    );
}

#[test]
fn a_comparison_becomes_a_report_with_numbers_kept_as_numbers() {
    use nomnom_storage::report::{self, Format};
    let d = tree();
    let r = d.path();
    let before = capture(r, 6, 1000);
    write(&r.join("Docs/x.txt"), 10); // shrinks by 90
    write(&r.join("Videos/a.mp4"), 6000);
    let after = capture(r, 6, 1000);
    let c = snapshot::compare(&before, &after, 50);
    let t = report::comparison_table(&c, "1 Jan", "now");
    assert!(t.title.contains("1 Jan") && t.title.contains("now"));
    let csv = String::from_utf8(report::render(&t, Format::Csv).unwrap()).unwrap();
    assert!(
        csv.contains("Folder,Docs,Shrank,100,10,-90"),
        "a negative number is not turned into text: {csv}"
    );
    let json: serde_json::Value =
        serde_json::from_slice(&report::render(&t, Format::Json).unwrap()).unwrap();
    assert!(json["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["path"] == "Docs" && r["delta"] == -90));
}

#[test]
fn folder_names_are_kept_the_same_way_and_the_schedule_knows_when_it_is_due() {
    use std::path::Path;
    assert_eq!(snapshot::root_key(Path::new("C:\\")), "C:\\");
    assert_eq!(snapshot::root_key(Path::new("C:")), "C:\\");
    assert_eq!(
        snapshot::root_key(Path::new(r"C:\Users\Me\")),
        r"C:\Users\Me"
    );
    assert_eq!(snapshot::root_key(Path::new("C:/Users/Me")), r"C:\Users\Me");
    let day = 86_400;
    assert!(!snapshot::schedule_due("off", None, 1000, false));
    assert!(snapshot::schedule_due("on_start", Some(5), 1000, false));
    assert!(!snapshot::schedule_due("on_start", None, 1000, true));
    assert!(snapshot::schedule_due("daily", None, 1000, true));
    assert!(!snapshot::schedule_due(
        "daily",
        Some(1000 - day + 1),
        1000,
        false
    ));
    assert!(snapshot::schedule_due(
        "daily",
        Some(1000 - day),
        1000,
        false
    ));
    assert!(!snapshot::schedule_due(
        "weekly",
        Some(1000 - 6 * day),
        1000,
        false
    ));
    assert!(snapshot::schedule_due(
        "weekly",
        Some(1000 - 7 * day),
        1000,
        false
    ));
    assert!(!snapshot::schedule_due("sometimes", None, 1000, false));
}
