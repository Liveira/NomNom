//! Tests for TreeSize-style Disk Analyzer additions: NTFS owners, size brackets, `[Files]` split,
//! exclude patterns, alternate data streams, NTFS compression, hardlink deduplication, new search
//! conditions, extended reports, and full-scan save/load round-trips.

use nomnom_storage::analyze::{self, SortKey};
use nomnom_storage::dupes;
use nomnom_storage::elevate::{load_scan_file, save_scan_file};
#[cfg(windows)]
use nomnom_storage::index::F_HAS_ADS;
use nomnom_storage::ops::{self, Kind, RunOptions, Status};
use nomnom_storage::protect::Protected;
use nomnom_storage::report::{self, Format};
#[cfg(windows)]
use nomnom_storage::scan::compressed_size;
use nomnom_storage::search::{compile, run, Query, TextMode};
use nomnom_storage::{ScanOptions, Session};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn write_bytes(path: &Path, data: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, data).unwrap();
}

fn scan_with(root: &Path, opts: ScanOptions) -> Arc<Session> {
    let s = Session::start(1, root.to_path_buf(), opts);
    let t = Instant::now();
    while !s.is_finished() {
        assert!(t.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(10));
    }
    s
}

#[test]
fn owners_size_buckets_own_files_and_excludes_work() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    write_bytes(&r.join("empty.txt"), &[]);
    write_bytes(&r.join("tiny.txt"), &[1u8; 500]);
    write_bytes(&r.join("med.bin"), &[2u8; 100_000]);
    write_bytes(&r.join("node_modules/pkg/index.js"), &[3u8; 50_000]);
    write_bytes(&r.join("ignore.tmp"), &[4u8; 9999]);
    std::fs::create_dir_all(r.join("sub_empty")).unwrap();

    let s = scan_with(
        r,
        ScanOptions {
            cluster: 4096,
            collect_owners: true,
            exclude: vec!["node_modules".into(), "*.tmp".into()],
            ..ScanOptions::default()
        },
    );
    let idx = s.index.read().unwrap();
    let root = idx.node(0).unwrap();
    assert_eq!(root.files, 3, "excluded folder and *.tmp were skipped");
    assert_eq!(root.own_files, 3);
    assert_eq!(root.own_size, 100_500);
    assert!(root.own_alloc >= 100_500);

    // Subfolders vs loose files split cleanly for TreeGrid `[Files]`.
    let dirs_only =
        analyze::children_filtered(&idx, 0, SortKey::Name, false, 0, 100, None, Some("dirs"));
    let files_only =
        analyze::children_filtered(&idx, 0, SortKey::Size, true, 0, 100, None, Some("files"));
    assert_eq!(dirs_only.total, 1);
    assert_eq!(dirs_only.items[0].name, "sub_empty");
    assert_eq!(files_only.total, 3);
    assert_eq!(files_only.items[0].name, "med.bin");

    // Owners are populated on NTFS.
    let owners = analyze::by_owner(&idx, 0, 20);
    assert_eq!(owners.iter().map(|g| g.files).sum::<u64>(), 3);
    assert!(!owners[0].key.is_empty());

    // Size buckets classify empty, tiny, and medium files.
    let buckets = analyze::by_size_bucket(&idx, 0);
    let map: std::collections::HashMap<_, _> = buckets
        .iter()
        .map(|g| (g.key.as_str(), (g.files, g.size)))
        .collect();
    assert_eq!(map["Empty (0 B)"], (1, 0));
    assert_eq!(map["Tiny (< 4 KB)"], (1, 500));
    assert_eq!(map["Medium (64 KB – 1 MB)"], (1, 100_000));

    // EmptyFolder, PathLen and Owner search conditions match accurately.
    let q_empty = compile(&Query::EmptyFolder, 1_800_000_000).unwrap();
    let found_empty = run(&idx, 0, &q_empty, 50, &AtomicBool::new(false));
    assert_eq!(found_empty.total, 1);
    assert_eq!(found_empty.items[0].name, "sub_empty");

    let min_len = idx.path_len(files_only.items[0].id);
    let q_len = compile(
        &Query::PathLen {
            min: Some(min_len),
            max: None,
        },
        1_800_000_000,
    )
    .unwrap();
    assert!(run(&idx, 0, &q_len, 50, &AtomicBool::new(false)).total >= 1);

    if let Some(owner_name) = files_only.items[0].owner.as_deref() {
        let q_owner = compile(
            &Query::Owner {
                text: owner_name.to_string(),
                mode: TextMode::Contains,
            },
            1_800_000_000,
        )
        .unwrap();
        assert_eq!(run(&idx, 0, &q_owner, 50, &AtomicBool::new(false)).total, 4);
    }
}

#[test]
#[cfg(windows)]
fn alternate_streams_and_ntfs_compression_work() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    let p = r.join("compressible.txt");
    write_bytes(
        &p,
        &" repetitive text line for NTFS LZNT1 compression!\n"
            .repeat(4000)
            .into_bytes(),
    );
    // Attach an NTFS alternate data stream.
    let stream_path = format!("{}:Zone.Identifier", p.display());
    if std::fs::write(&stream_path, b"[ZoneTransfer]\r\nZoneId=3\r\n").is_err() {
        eprintln!("SKIPPED: filesystem does not support NTFS alternate streams");
        return;
    }

    let s = scan_with(
        r,
        ScanOptions {
            collect_streams: true,
            ..ScanOptions::default()
        },
    );
    let fid = {
        let idx = s.index.read().unwrap();
        let fid = idx.resolve_path(&p).unwrap();
        let v = idx.view(fid);
        assert_eq!(v.ads_count, 1);
        assert!(v.ads_size >= 20);
        assert!(v.flags & F_HAS_ADS != 0);
        let q = compile(
            &Query::Attr {
                name: "streams".into(),
                set: true,
            },
            1_800_000_000,
        )
        .unwrap();
        assert_eq!(run(&idx, 0, &q, 10, &AtomicBool::new(false)).total, 1);
        fid
    };

    // Compress in place via `ops` and verify on-disk space shrinks while logical content stays intact.
    let before_bytes = std::fs::read(&p).unwrap();
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Compress,
        &[fid],
        None,
        None,
        &Protected::default(),
    );
    assert_eq!(plan.items.len(), 1);
    let rep = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(rep.count(Status::Done), 1, "{:?}", rep.results);
    assert_eq!(std::fs::read(&p).unwrap(), before_bytes);
    let comp_sz = compressed_size(&p).unwrap();
    assert!(
        comp_sz < before_bytes.len() as u64,
        "expected compressed size {comp_sz} < {}",
        before_bytes.len()
    );

    // Decompress back in place.
    let s2 = scan_with(r, ScanOptions::default());
    let fid2 = s2.index.read().unwrap().resolve_path(&p).unwrap();
    let decomp_plan = ops::plan(
        &s2.index.read().unwrap(),
        Kind::Decompress,
        &[fid2],
        None,
        None,
        &Protected::default(),
    );
    let rep2 = ops::run(
        &decomp_plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(rep2.count(Status::Done), 1);
    assert_eq!(std::fs::read(&p).unwrap(), before_bytes);
}

#[test]
fn deduplicating_with_hardlinks_keeps_both_paths_and_shares_storage() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    let data = vec![0xABu8; 250_000];
    write_bytes(&r.join("a/clip.bin"), &data);
    write_bytes(&r.join("b/clip_copy.bin"), &data);

    let s = scan_with(
        r,
        ScanOptions {
            hardlink_min: 1024,
            ..ScanOptions::default()
        },
    );
    let job = dupes::start(s.clone(), 0, 1024, 2);
    while !job.is_finished() {
        std::thread::sleep(Duration::from_millis(10));
    }
    let g = &job.groups(0, 10)[0];
    let (keep, target) = (g.copies[0].node, g.copies[1].node);
    let plan = ops::plan_duplicates(
        &s.index.read().unwrap(),
        Kind::Hardlink,
        &[(keep, vec![target])],
        &Protected::default(),
    );
    assert_eq!(plan.items.len(), 1, "{:?}", plan.refused);
    let rep = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(rep.count(Status::Done), 1, "{:?}", rep.results);

    // Both files still exist and read identically, and a fresh scan sees one shared hardlink!
    assert_eq!(std::fs::read(r.join("a/clip.bin")).unwrap(), data);
    assert_eq!(std::fs::read(r.join("b/clip_copy.bin")).unwrap(), data);
    let after = scan_with(
        r,
        ScanOptions {
            hardlink_min: 1024,
            cluster: 4096,
            ..ScanOptions::default()
        },
    );
    let root = after.index.read().unwrap().node(0).unwrap().clone();
    assert_eq!(root.size, 500_000);
    assert!(
        root.alloc < 300_000,
        "alloc={} should count data once",
        root.alloc
    );
}

#[test]
fn scan_file_round_trips_and_extended_reports_render() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path().join("data");
    write_bytes(&r.join("video/demo.mp4"), &[8u8; 12_345]);
    write_bytes(&r.join("readme.md"), b"hello");
    let s = scan_with(&r, ScanOptions::default());

    let out_file = d.path().join("saved.nomnom-scan.json");
    let written = save_scan_file(&s, &out_file).unwrap();
    assert!(written > 50);

    let loaded = load_scan_file(99, &out_file).unwrap();
    let idx = loaded.index.read().unwrap();
    assert_eq!(idx.node(0).unwrap().size, 12_350);
    assert_eq!(idx.node(0).unwrap().files, 2);

    let ext_table = report::tree_table_ext(&idx, 0, 4);
    let picked = report::select_columns(
        &ext_table,
        &[
            "name".into(),
            "own_size".into(),
            "owner".into(),
            "path_len".into(),
            "attributes".into(),
        ],
    );
    assert_eq!(picked.columns.len(), 5);
    assert!(!report::render(&picked, Format::Csv).unwrap().is_empty());
}

#[test]
#[cfg(unix)]
fn linux_freedesktop_trash_moves_file_and_writes_trashinfo() {
    let d = tempfile::tempdir().unwrap();
    let xdg_data = d.path().join("xdg-data");
    std::fs::create_dir_all(&xdg_data).unwrap();
    std::env::set_var("XDG_DATA_HOME", &xdg_data);

    let data_root = d.path().join("work");
    write_bytes(
        &data_root.join("notes with spaces.txt"),
        b"keep safe in trash",
    );
    let s = scan_with(&data_root, ScanOptions::default());
    let fid = s
        .index
        .read()
        .unwrap()
        .resolve_path(&data_root.join("notes with spaces.txt"))
        .unwrap();
    let plan = ops::plan(
        &s.index.read().unwrap(),
        Kind::Recycle,
        &[fid],
        None,
        None,
        &Protected::default(),
    );
    assert_eq!(plan.items.len(), 1, "{:?}", plan.refused);
    let rep = ops::run(
        &plan,
        RunOptions::default(),
        &AtomicBool::new(false),
        &|_| {},
    );
    assert_eq!(rep.count(Status::Done), 1, "{:?}", rep.results);
    assert!(!data_root.join("notes with spaces.txt").exists());
    assert_eq!(
        std::fs::read(xdg_data.join("Trash/files/notes with spaces.txt")).unwrap(),
        b"keep safe in trash"
    );
    let info = std::fs::read_to_string(xdg_data.join("Trash/info/notes with spaces.txt.trashinfo"))
        .unwrap();
    assert!(info.starts_with("[Trash Info]\nPath="), "{info}");
    assert!(info.contains("notes%20with%20spaces.txt"), "{info}");
    assert!(info.contains("DeletionDate="), "{info}");
}
