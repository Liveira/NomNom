use nomnom_storage::batch::{plan_batch_rename, rename_preview, RenameRule};
use nomnom_storage::ops::{self, Kind, RunOptions, Status};
use nomnom_storage::protect::Protected;
use nomnom_storage::{ScanOptions, Session};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn put(path: &Path, size: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![3u8; size]).unwrap();
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

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn ok(r: Vec<Result<String, String>>) -> Vec<String> {
    r.into_iter().map(|x| x.unwrap()).collect()
}

#[test]
fn rules_find_replace_number_and_change_case() {
    let n = names(&["IMG_001.JPG", "IMG_002.jpg", "notes.txt"]);
    let r = RenameRule {
        find: "IMG_".into(),
        replace: "Holiday ".into(),
        ..Default::default()
    };
    assert_eq!(
        ok(rename_preview(&n, &r).unwrap()),
        ["Holiday 001.JPG", "Holiday 002.jpg", "notes.txt"]
    );

    let numbered = RenameRule {
        prefix: "trip-{n}-".into(),
        start: 1,
        step: 1,
        pad: 3,
        ..Default::default()
    };
    assert_eq!(
        ok(rename_preview(&n, &numbered).unwrap()),
        [
            "trip-001-IMG_001.JPG",
            "trip-002-IMG_002.jpg",
            "trip-003-notes.txt"
        ]
    );

    let lower = RenameRule {
        case: "lower".into(),
        suffix: "_x".into(),
        ..Default::default()
    };
    assert_eq!(ok(rename_preview(&n, &lower).unwrap())[0], "img_001_x.jpg");

    let re = RenameRule {
        find: r"IMG_(\d+)".into(),
        replace: "photo-$1".into(),
        regex: true,
        ..Default::default()
    };
    assert_eq!(ok(rename_preview(&n, &re).unwrap())[1], "photo-002.jpg");
}

#[test]
fn the_extension_is_kept_unless_asked_and_bad_names_are_reported_per_item() {
    let n = names(&["a.txt", "b.txt"]);
    let r = RenameRule {
        find: "txt".into(),
        replace: "md".into(),
        ..Default::default()
    };
    assert_eq!(
        ok(rename_preview(&n, &r).unwrap()),
        ["a.txt", "b.txt"],
        "the extension isn't part of the match"
    );
    let r2 = RenameRule {
        find: "txt".into(),
        replace: "md".into(),
        include_extension: true,
        ..Default::default()
    };
    assert_eq!(ok(rename_preview(&n, &r2).unwrap()), ["a.md", "b.md"]);
    let bad = RenameRule {
        find: "a".into(),
        replace: "x/y".into(),
        ..Default::default()
    };
    let out = rename_preview(&n, &bad).unwrap();
    assert!(out[0].is_err() && out[1].is_ok());
    assert!(rename_preview(
        &n,
        &RenameRule {
            find: "(".into(),
            regex: true,
            ..Default::default()
        }
    )
    .err()
    .unwrap()
    .contains("isn't valid"));
}

#[test]
fn a_batch_rename_runs_item_by_item_and_refuses_clashes() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("p/a1.txt"), 5);
    put(&r.join("p/a2.txt"), 5);
    put(&r.join("p/taken.txt"), 5);
    put(&r.join("q/a1.txt"), 5);
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let node = |p: &str| idx.resolve_path(&r.join(p)).unwrap();

    // Everything is called the same: only the first of each folder can have the name.
    let all_same = RenameRule {
        find: "a1".into(),
        replace: "taken".into(),
        ..Default::default()
    };
    let plan = plan_batch_rename(
        &idx,
        &[node("p/a1.txt"), node("q/a1.txt")],
        &all_same,
        &Protected::default(),
    )
    .unwrap();
    assert_eq!(
        plan.items.len(),
        1,
        "p/a1 would clash with p/taken, q/a1 is fine: {:?}",
        plan.refused
    );
    assert!(plan.refused[0].reason.contains("already exists"));

    let rule = RenameRule {
        prefix: "new-{n}-".into(),
        start: 10,
        step: 10,
        ..Default::default()
    };
    let plan = plan_batch_rename(
        &idx,
        &[node("p/a1.txt"), node("p/a2.txt"), node("q/a1.txt")],
        &rule,
        &Protected::default(),
    )
    .unwrap();
    assert_eq!(plan.kind, Kind::BatchRename);
    assert_eq!(plan.items.len(), 3);
    let report = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(report.count(Status::Done), 3, "{:?}", report.results);
    assert!(
        r.join("p/new-10-a1.txt").exists()
            && r.join("p/new-20-a2.txt").exists()
            && r.join("q/new-30-a1.txt").exists()
    );
    assert!(r.join("p/taken.txt").exists());
}

#[test]
fn renaming_to_a_name_another_item_is_giving_up_is_allowed_but_two_into_one_is_not() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("x/one.txt"), 1);
    put(&r.join("x/two.txt"), 1);
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let node = |p: &str| idx.resolve_path(&r.join(p)).unwrap();
    let collapse = RenameRule {
        find: "one".into(),
        replace: "same".into(),
        ..Default::default()
    };
    let both = RenameRule {
        find: "t".into(),
        replace: "T".into(),
        ..Default::default()
    };
    // Two different sources mapping to one name.
    let rule = RenameRule {
        find: "".into(),
        prefix: "".into(),
        suffix: "".into(),
        ..Default::default()
    };
    let none = plan_batch_rename(&idx, &[node("x/one.txt")], &rule, &Protected::default()).unwrap();
    assert!(none.items.is_empty(), "an unchanged name isn't a rename");
    let ok =
        plan_batch_rename(&idx, &[node("x/one.txt")], &collapse, &Protected::default()).unwrap();
    assert_eq!(ok.items[0].new_name.as_deref(), Some("same.txt"));
    let _ = both;
}

#[test]
fn zip_puts_files_and_folders_in_one_archive_without_following_links_or_replacing() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("src/doc.txt"), 1000);
    put(&r.join("src/sub/deep.bin"), 4000);
    put(&r.join("loose.txt"), 10);
    std::fs::create_dir_all(r.join("out")).unwrap();
    std::fs::write(r.join("out/Pack.zip"), b"mine").unwrap();
    let outside = tempfile::tempdir().unwrap();
    put(&outside.path().join("secret.txt"), 77);
    let linked = junction::create(outside.path(), r.join("src/link")).is_ok();
    let s = scan(r);
    let idx = s.index.read().unwrap();
    let node = |p: &str| idx.resolve_path(&r.join(p)).unwrap();
    let plan = ops::plan(
        &idx,
        Kind::Zip,
        &[node("src"), node("loose.txt")],
        Some(&r.join("out")),
        Some("Pack"),
        &Protected::default(),
    );
    assert!(plan.refused.is_empty(), "{:?}", plan.refused);
    let report = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(report.count(Status::Done), 2, "{:?}", report.results);
    let to = report.results[0].to.clone().unwrap();
    assert!(
        to.ends_with("Pack (2).zip"),
        "the existing Pack.zip must stay: {to}"
    );
    assert_eq!(std::fs::read(r.join("out/Pack.zip")).unwrap(), b"mine");

    let mut z = zip::ZipArchive::new(std::fs::File::open(&to).unwrap()).unwrap();
    let mut entries: Vec<String> = (0..z.len())
        .map(|i| z.by_index(i).unwrap().name().to_string())
        .collect();
    entries.sort();
    assert!(
        entries.contains(&"src/doc.txt".to_string())
            && entries.contains(&"src/sub/deep.bin".to_string())
            && entries.contains(&"loose.txt".to_string()),
        "{entries:?}"
    );
    if linked {
        assert!(
            !entries.iter().any(|e| e.contains("secret")),
            "a link must not be followed into the archive"
        );
    }
    let mut content = Vec::new();
    z.by_name("src/sub/deep.bin")
        .unwrap()
        .read_to_end(&mut content)
        .unwrap();
    assert_eq!(content.len(), 4000);
    assert!(
        r.join("src/doc.txt").exists(),
        "zipping leaves the originals"
    );
}

#[test]
fn zip_of_nothing_valid_leaves_no_file() {
    let d = tempfile::tempdir().unwrap();
    put(&d.path().join("a.txt"), 5);
    std::fs::create_dir_all(d.path().join("out")).unwrap();
    let s = scan(d.path());
    let idx = s.index.read().unwrap();
    let a = idx.resolve_path(&d.path().join("a.txt")).unwrap();
    let plan = ops::plan(
        &idx,
        Kind::Zip,
        &[a],
        Some(&d.path().join("out")),
        Some("x"),
        &Protected::default(),
    );
    std::fs::remove_file(d.path().join("a.txt")).unwrap();
    let report = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(report.count(Status::Done), 0);
    assert!(!d.path().join("out/x.zip").exists());
}
