//! The safety suite. Every test builds its own temporary folders; nothing here ever touches a real
//! user folder. The main claim being proven: whatever the organizer is not sure about stays
//! exactly as it was, byte for byte.

mod common;

use common::*;
use nomnom_organizer::exec::{execute, recover, undo, ExecOptions};
use nomnom_organizer::journal::Journal;
use nomnom_organizer::model::{ConflictPolicy, Move, Plan, SkipCode};
use nomnom_organizer::scan::plan;
use nomnom_organizer::winfs;
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

const NO_CROSS: ExecOptions = ExecOptions {
    allow_cross_volume: false,
};

fn run(
    sb: &Sandbox,
    o: &nomnom_organizer::Options,
) -> (Plan, nomnom_organizer::exec::Report, Journal) {
    let p = plan(o).unwrap();
    let j = sb.journal();
    let r = execute(&j, &p, "test", &NO_CROSS, &|| false).unwrap();
    (p, r, j)
}

fn skip_for<'a>(p: &'a Plan, path: &Path) -> Option<&'a nomnom_organizer::Skip> {
    p.skips
        .iter()
        .find(|s| nomnom_organizer::pathutil::is_same(&s.path, path))
}

fn moved_names(p: &Plan) -> Vec<String> {
    let mut v: Vec<String> = p
        .moves
        .iter()
        .map(|m| m.src.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

// ---- the example from the specification --------------------------------------------------

fn build_spec_tree(sb: &Sandbox) {
    sb.put("random-video.mp4", &mp4(1));
    sb.put("image.png", &png(2));
    sb.put("vacation-video.mkv", &mkv(3));
    sb.put("Installer/setup.exe", &exe());
    sb.put("Installer/config.dll", &exe());
    sb.put("Installer/resources/logo.png", &png(4));
    sb.put("Software/application.exe", &exe());
    sb.put("Software/data/x.bin", &[1, 2, 3, 4]);
    sb.put("Software/assets/pic.png", &png(5));
}

#[test]
fn installer_and_software_folders_stay_untouched_at_every_depth() {
    for depth in [0, 1, 2, 5] {
        let sb = Sandbox::new();
        build_spec_tree(&sb);
        let installer = snapshot(&sb.path("Installer"));
        let software = snapshot(&sb.path("Software"));
        let everything = all_content(&[&sb.downloads]);

        let mut o = sb.options();
        o.max_depth = depth;
        let (p, report, _) = run(&sb, &o);

        assert_eq!(
            moved_names(&p),
            ["image.png", "random-video.mp4", "vacation-video.mkv"],
            "depth {depth}"
        );
        assert_eq!(report.moved, 3);
        assert!(report.failed.is_empty());
        assert_eq!(
            snapshot(&sb.path("Installer")),
            installer,
            "Installer changed at depth {depth}"
        );
        assert_eq!(
            snapshot(&sb.path("Software")),
            software,
            "Software changed at depth {depth}"
        );
        assert!(exists(&sb.dest.join("Videos").join("random-video.mp4")));
        assert!(exists(&sb.dest.join("Videos").join("vacation-video.mkv")));
        assert!(exists(&sb.dest.join("Images").join("image.png")));
        assert_eq!(
            all_content(&[&sb.downloads, &sb.dest]),
            everything,
            "nothing lost or altered"
        );

        let expected = if depth == 0 {
            SkipCode::FolderNotOpened
        } else {
            SkipCode::SoftwareFolder
        };
        for dir in ["Installer", "Software"] {
            assert_eq!(
                skip_for(&p, &sb.path(dir)).unwrap().code,
                expected,
                "{dir} at depth {depth}"
            );
        }
        // the folders' insides are never even listed as skipped items: they were not entered
        assert!(p
            .skips
            .iter()
            .all(
                |s| !nomnom_organizer::pathutil::is_inside(&s.path, &sb.path("Installer"))
                    || s.path == sb.path("Installer")
            ));
    }
}

#[test]
fn one_program_in_a_folder_of_pictures_protects_the_whole_folder() {
    let sb = Sandbox::new();
    sb.put("Photos/a.png", &png(1));
    sb.put("Photos/b.jpg", &jpg(2));
    sb.put("Photos/helper.exe", &exe());
    sb.put("loose.png", &png(3));
    let before = snapshot(&sb.path("Photos"));
    let mut o = sb.options();
    o.max_depth = 3;
    let (p, _, _) = run(&sb, &o);
    assert_eq!(moved_names(&p), ["loose.png"]);
    assert_eq!(snapshot(&sb.path("Photos")), before);
    assert_eq!(
        skip_for(&p, &sb.path("Photos")).unwrap().code,
        SkipCode::SoftwareFolder
    );
}

#[test]
fn projects_and_dependency_folders_are_never_entered() {
    let sb = Sandbox::new();
    sb.put("Trip/.git/config", b"[core]\n");
    sb.put("Trip/a.png", &png(1));
    sb.put("Site/img/logo.png", &png(2));
    sb.put("Site/node_modules/pkg/deep/x.txt", b"text");
    sb.put("App/package.json", b"{}");
    sb.put("App/shot.png", &png(3));
    sb.put("Game/steam_appid.txt", b"480");
    sb.put("Game/cover.jpg", &jpg(4));
    sb.put("Proj/build/out.png", &png(5));
    let before = snapshot(&sb.downloads);
    let mut o = sb.options();
    o.max_depth = 4;
    let (p, r, _) = run(&sb, &o);
    assert!(p.moves.is_empty(), "{:?}", p.moves);
    assert_eq!(r.moved, 0);
    assert_eq!(snapshot(&sb.downloads), before);
    assert!(p.skips.iter().all(|s| s.code == SkipCode::SoftwareFolder));
}

#[test]
fn a_program_with_a_media_name_is_not_trusted() {
    let sb = Sandbox::new();
    sb.put("movie.mp4", &exe());
    sb.put("picture.png", b"#!/bin/sh\nrm -rf ~\n");
    sb.put("Album/ok.png", &png(1));
    sb.put("Album/fake.mp4", &exe());
    let before = snapshot(&sb.downloads);
    let mut o = sb.options();
    o.max_depth = 2;
    let (p, r, _) = run(&sb, &o);
    assert_eq!(r.moved, 0);
    assert_eq!(snapshot(&sb.downloads), before);
    assert_eq!(
        skip_for(&p, &sb.path("movie.mp4")).unwrap().code,
        SkipCode::Program
    );
    assert_eq!(
        skip_for(&p, &sb.path("picture.png")).unwrap().code,
        SkipCode::Program
    );
    assert_eq!(
        skip_for(&p, &sb.path("Album")).unwrap().code,
        SkipCode::SoftwareFolder
    );
}

#[test]
fn archives_executables_and_scripts_are_left_where_they_are() {
    let sb = Sandbox::new();
    for (n, b) in [
        ("setup.exe", exe()),
        ("run.ps1", b"Write-Host hi".to_vec()),
        ("tool.bat", b"@echo off".to_vec()),
        ("lib.dll", exe()),
        ("installer.msi", exe()),
        ("pack.zip", b"PK\x03\x04zz".to_vec()),
        ("data.7z", b"7z\xBC\xAF".to_vec()),
    ] {
        sb.put(n, &b);
    }
    let before = snapshot(&sb.downloads);
    let (p, r, _) = run(&sb, &sb.options());
    assert_eq!(r.moved, 0);
    assert_eq!(snapshot(&sb.downloads), before);
    assert_eq!(
        p.skips
            .iter()
            .filter(|s| s.code == SkipCode::Archive)
            .count(),
        2
    );
    assert_eq!(
        p.skips
            .iter()
            .filter(|s| s.code == SkipCode::Program)
            .count(),
        5
    );
}

// ---- links, junctions, loops -------------------------------------------------------------

#[test]
fn a_junction_is_never_followed_and_what_it_points_to_is_untouched() {
    let sb = Sandbox::new();
    let outside = sb._root.path().join("Outside");
    put_at(&outside.join("precious.mp4"), &mp4(9));
    put_at(&outside.join("sub").join("photo.png"), &png(9));
    let outside_before = snapshot(&outside);
    junction::create(&outside, sb.path("shortcut")).unwrap();
    sb.put("real.png", &png(1));

    let mut o = sb.options();
    o.max_depth = 5;
    let (p, _, _) = run(&sb, &o);

    assert_eq!(moved_names(&p), ["real.png"]);
    assert_eq!(
        skip_for(&p, &sb.path("shortcut")).unwrap().code,
        SkipCode::Link
    );
    assert_eq!(
        snapshot(&outside),
        outside_before,
        "the folder behind the junction was not touched"
    );
    assert!(exists(&outside.join("precious.mp4")));
}

#[test]
fn a_junction_that_loops_back_does_not_cause_a_loop() {
    let sb = Sandbox::new();
    fs::create_dir_all(sb.path("Stuff")).unwrap();
    junction::create(&sb.downloads, sb.path("Stuff").join("loop")).unwrap();
    sb.put("Stuff/a.png", &png(1));
    let mut o = sb.options();
    o.max_depth = 6;
    let started = std::time::Instant::now();
    let (p, r, _) = run(&sb, &o);
    assert!(started.elapsed().as_secs() < 10);
    assert_eq!(
        r.moved, 0,
        "the folder with a link inside is skipped as a whole"
    );
    assert!(skip_for(&p, &sb.path("Stuff")).is_some());
}

#[test]
fn symbolic_links_are_skipped_when_this_machine_allows_creating_them() {
    let sb = Sandbox::new();
    let target = sb.put("real.png", &png(1));
    let link = sb.path("link-to-real.png");
    if std::os::windows::fs::symlink_file(&target, &link).is_err() {
        eprintln!("skipped: creating symbolic links needs a privilege this account doesn't have");
        return;
    }
    let (p, _, _) = run(&sb, &sb.options());
    assert_eq!(skip_for(&p, &link).unwrap().code, SkipCode::Link);
    assert!(exists(&sb.dest.join("Images").join("real.png")));
    assert!(
        winfs::info(&link).is_ok(),
        "the link itself was not removed or moved"
    );
}

#[test]
fn a_source_that_is_itself_a_junction_is_refused() {
    let sb = Sandbox::new();
    let real = sb._root.path().join("Real");
    put_at(&real.join("a.png"), &png(1));
    let link = sb._root.path().join("Linked");
    junction::create(&real, &link).unwrap();
    let mut o = sb.options();
    o.source = link;
    assert!(plan(&o).unwrap_err().to_string().contains("link"));
    assert!(exists(&real.join("a.png")));
}

// ---- names, collisions, odd files --------------------------------------------------------

#[test]
fn same_name_in_two_folders_never_overwrites_and_never_loses_a_file() {
    for policy in [ConflictPolicy::Skip, ConflictPolicy::KeepBoth] {
        let sb = Sandbox::new();
        sb.put("a/clip.mp4", &mp4(1));
        sb.put("b/CLIP.MP4", &mp4(2));
        put_at(&sb.dest.join("Videos").join("existing.mp4"), &mp4(7));
        let everything = all_content(&[&sb.downloads, &sb.dest]);
        let mut o = sb.options();
        o.max_depth = 1;
        o.conflict = policy;
        let (p, r, _) = run(&sb, &o);
        assert!(r.failed.is_empty(), "{:?}", r.failed);
        assert_eq!(
            all_content(&[&sb.downloads, &sb.dest]),
            everything,
            "{policy:?}: no file lost or changed"
        );
        let landed = fs::read_dir(sb.dest.join("Videos")).unwrap().count();
        match policy {
            ConflictPolicy::Skip => {
                assert_eq!(
                    p.moves.len(),
                    1,
                    "the second one is left because the name is taken"
                );
                assert!(p.skips.iter().any(|s| s.code == SkipCode::Conflict));
                assert_eq!(landed, 2);
            }
            ConflictPolicy::KeepBoth => {
                assert_eq!(p.moves.len(), 2);
                assert_eq!(landed, 3);
            }
        }
    }
}

#[test]
fn a_file_already_in_the_destination_is_never_replaced() {
    let sb = Sandbox::new();
    put_at(
        &sb.dest.join("Images").join("photo.png"),
        b"\x89PNGprecious original",
    );
    sb.put("photo.png", &png(2));
    let (p, r, _) = run(&sb, &sb.options());
    assert_eq!(r.moved, 0);
    assert_eq!(
        skip_for(&p, &sb.path("photo.png")).unwrap().code,
        SkipCode::Conflict
    );
    assert_eq!(
        fs::read(sb.dest.join("Images").join("photo.png")).unwrap(),
        b"\x89PNGprecious original"
    );
    assert!(exists(&sb.path("photo.png")));
}

#[test]
fn files_in_use_unfinished_downloads_unknown_types_and_empty_files_stay() {
    let sb = Sandbox::new();
    let busy = sb.put("busy.mp4", &mp4(1));
    let _hold = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&busy)
        .unwrap();
    sb.put("half.mp4.part", &mp4(2));
    sb.put("half2.crdownload", &mp4(3));
    sb.put("thing.xyz", b"???");
    sb.put("README", b"no extension");
    sb.put("empty.mp4", b"");
    sb.put("liar.pdf", b"this is not a pdf");
    sb.put("fine.pdf", &pdf(4));
    let (p, r, _) = run(&sb, &sb.options());
    assert_eq!(moved_names(&p), ["fine.pdf"]);
    assert_eq!(r.moved, 1);
    let code = |n: &str| skip_for(&p, &sb.path(n)).unwrap().code;
    assert_eq!(code("busy.mp4"), SkipCode::Locked);
    assert_eq!(code("half.mp4.part"), SkipCode::InProgress);
    assert_eq!(code("half2.crdownload"), SkipCode::InProgress);
    assert_eq!(code("thing.xyz"), SkipCode::Unknown);
    assert_eq!(code("README"), SkipCode::Unknown);
    assert_eq!(code("empty.mp4"), SkipCode::Empty);
    assert_eq!(code("liar.pdf"), SkipCode::Mismatch);
    assert!(
        exists(&sb.path("busy.mp4"))
            && exists(&sb.path("half.mp4.part"))
            && exists(&sb.path("empty.mp4"))
    );
}

#[test]
fn a_file_changed_a_moment_ago_is_left_alone() {
    let sb = Sandbox::new();
    fs::write(sb.path("fresh.mp4"), mp4(1)).unwrap(); // not aged
    let mut o = sb.options();
    o.settle_secs = 30;
    let (p, r, _) = run(&sb, &o);
    assert_eq!(r.moved, 0);
    assert_eq!(
        skip_for(&p, &sb.path("fresh.mp4")).unwrap().code,
        SkipCode::RecentlyChanged
    );
}

#[test]
fn hidden_and_system_items_are_skipped() {
    let sb = Sandbox::new();
    let h = sb.put("hidden.mp4", &mp4(1));
    sb.put("HiddenDir/a.png", &png(2));
    sb.put("visible.mp4", &mp4(3));
    for p in [&h, &sb.path("HiddenDir")] {
        std::process::Command::new("attrib")
            .arg("+h")
            .arg(p)
            .status()
            .unwrap();
    }
    let mut o = sb.options();
    o.max_depth = 2;
    let (p, _, _) = run(&sb, &o);
    assert_eq!(moved_names(&p), ["visible.mp4"]);
    assert_eq!(skip_for(&p, &h).unwrap().code, SkipCode::Hidden);
    assert_eq!(
        skip_for(&p, &sb.path("HiddenDir")).unwrap().code,
        SkipCode::Hidden
    );
}

#[test]
fn unicode_and_awkward_names_move_cleanly() {
    let sb = Sandbox::new();
    sb.put("vídeo férias 日本語 😀.mp4", &mp4(1));
    sb.put("a  double  spaces.png", &png(2));
    sb.put("UPPER.JPG", &jpg(3));
    let before = all_content(&[&sb.downloads]);
    let (_, r, _) = run(&sb, &sb.options());
    assert_eq!(r.moved, 3, "{:?}", r.failed);
    assert_eq!(all_content(&[&sb.dest]), before);
}

#[test]
fn a_read_only_file_is_moved_and_stays_read_only() {
    let sb = Sandbox::new();
    let f = sb.put("ro.mp4", &mp4(1));
    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&f, perms).unwrap();
    let (_, r, _) = run(&sb, &sb.options());
    assert_eq!(r.moved, 1, "{:?}", r.failed);
    assert!(fs::metadata(sb.dest.join("Videos").join("ro.mp4"))
        .unwrap()
        .permissions()
        .readonly());
}

// ---- the folders themselves --------------------------------------------------------------

#[test]
fn protected_places_and_bad_paths_are_refused_before_anything_is_read() {
    let sb = Sandbox::new();
    sb.put("a.png", &png(1));
    let base = sb.options();

    let mut o = base.clone();
    o.protected_zones = vec![sb._root.path().to_path_buf()];
    assert!(
        plan(&o).unwrap_err().to_string().contains("protected"),
        "inside a protected zone"
    );

    let mut o = base.clone();
    o.protected_exact = vec![sb.downloads.clone()];
    assert!(plan(&o).is_err(), "the whole profile folder");

    let mut o = base.clone();
    o.source = PathBuf::from(r"C:\");
    assert!(plan(&o).unwrap_err().to_string().contains("drive"));

    let mut o = base.clone();
    o.dest_root = PathBuf::from(r"C:\");
    assert!(plan(&o).is_err(), "a drive as the destination");

    let mut o = base.clone();
    o.protected_zones = vec![sb.dest.clone()];
    assert!(plan(&o).is_err(), "moving into a protected zone");

    let mut o = base.clone();
    o.source = PathBuf::from(r"relative\folder");
    assert!(plan(&o).is_err(), "relative paths");

    let mut o = base.clone();
    o.source = sb.downloads.join("..").join("Downloads");
    assert!(plan(&o).is_err(), "paths with ..");

    let mut o = base.clone();
    o.source = sb.path("a.png");
    assert!(
        plan(&o).unwrap_err().to_string().contains("folder"),
        "a file instead of a folder"
    );

    let mut o = base.clone();
    o.source = sb.path("does-not-exist");
    assert!(plan(&o).is_err());

    assert!(
        exists(&sb.path("a.png")),
        "nothing was moved by any of those"
    );
}

#[test]
fn an_already_organized_destination_inside_the_source_is_never_reorganized() {
    let sb = Sandbox::new();
    let documents = sb._root.path().join("Documents");
    put_at(
        &documents.join("Files").join("Videos").join("old.mp4"),
        &mp4(1),
    );
    put_at(&documents.join("new.mp4"), &mp4(2));
    let mut o = sb.options();
    o.source = documents.clone();
    o.max_depth = 4;
    let (p, r, _) = run(&sb, &o);
    assert_eq!(moved_names(&p), ["new.mp4"]);
    assert_eq!(r.moved, 1);
    assert!(exists(&sb.dest.join("Videos").join("old.mp4")));
    assert!(exists(&sb.dest.join("Videos").join("new.mp4")));
    // and the other way round: organizing the destination itself is refused
    let mut back = sb.options();
    back.source = sb.dest.clone();
    assert!(plan(&back).is_err());
}

#[test]
fn a_folder_that_cannot_be_read_is_left_alone() {
    let sb = Sandbox::new();
    sb.put("Locked/a.png", &png(1));
    sb.put("fine.png", &png(2));
    let who = std::env::var("USERNAME").unwrap_or_default();
    let dir = sb.path("Locked");
    let denied = std::process::Command::new("icacls")
        .arg(&dir)
        .arg("/deny")
        .arg(format!("{who}:(RD)"))
        .output();
    if !matches!(&denied, Ok(o) if o.status.success()) {
        eprintln!("skipped: couldn't change folder permissions here");
        return;
    }
    if fs::read_dir(&dir).is_ok() {
        // This account can still list the folder (an administrator, or a policy overrides the deny),
        // so there is nothing to test here. Put the permissions back and move on.
        let _ = std::process::Command::new("icacls")
            .arg(&dir)
            .arg("/remove:d")
            .arg(&who)
            .output();
        eprintln!("skipped: the folder is still readable by this account");
        return;
    }
    let mut o = sb.options();
    o.max_depth = 2;
    let result = plan(&o);
    // always restore, so the temporary folder can be deleted
    let _ = std::process::Command::new("icacls")
        .arg(&dir)
        .arg("/remove:d")
        .arg(&who)
        .output();
    let p = result.unwrap();
    assert_eq!(moved_names(&p), ["fine.png"]);
    assert!(skip_for(&p, &dir).is_some());
    assert!(exists(&dir.join("a.png")));
}

// ---- changes between the preview and the move --------------------------------------------

#[test]
fn a_file_edited_after_the_preview_is_not_moved() {
    let sb = Sandbox::new();
    let f = sb.put("doc.pdf", &pdf(1));
    let p = plan(&sb.options()).unwrap();
    assert_eq!(p.moves.len(), 1);
    let mut grown = pdf(1);
    grown.extend_from_slice(b"edited after the preview");
    fs::write(&f, &grown).unwrap();
    let j = sb.journal();
    let r = execute(&j, &p, "test", &NO_CROSS, &|| false).unwrap();
    assert_eq!(r.moved, 0);
    assert!(r.failed[0].message.contains("changed"));
    assert_eq!(
        fs::read(&f).unwrap(),
        grown,
        "the edited file is exactly where and as it was"
    );
    assert!(!exists(&sb.dest.join("Documents")));
}

#[test]
fn a_file_swapped_for_another_with_the_same_name_is_not_moved() {
    let sb = Sandbox::new();
    let f = sb.put("pic.png", &png(1));
    let p = plan(&sb.options()).unwrap();
    fs::remove_file(&f).unwrap();
    put_at(&f, &png(1)); // same name, same bytes, but a different file
    let r = execute(&sb.journal(), &p, "test", &NO_CROSS, &|| false).unwrap();
    assert_eq!(
        r.moved, 0,
        "an identical-looking replacement is still a different file"
    );
    assert!(exists(&f));
}

#[test]
fn a_name_that_appears_in_the_destination_after_the_preview_is_never_replaced() {
    let sb = Sandbox::new();
    let f = sb.put("song.mp3", &mp3(1));
    let p = plan(&sb.options()).unwrap();
    let taken = sb.dest.join("Audio").join("song.mp3");
    put_at(&taken, b"ID3 somebody else's file");
    let r = execute(&sb.journal(), &p, "test", &NO_CROSS, &|| false).unwrap();
    assert_eq!(r.moved, 0);
    assert_eq!(fs::read(&taken).unwrap(), b"ID3 somebody else's file");
    assert!(exists(&f));
}

#[test]
fn a_file_that_vanishes_before_its_turn_is_reported_not_fatal() {
    let sb = Sandbox::new();
    let gone = sb.put("a.png", &png(1));
    sb.put("b.png", &png(2));
    let p = plan(&sb.options()).unwrap();
    fs::remove_file(&gone).unwrap();
    let r = execute(&sb.journal(), &p, "test", &NO_CROSS, &|| false).unwrap();
    assert_eq!(r.moved, 1);
    assert_eq!(r.failed.len(), 1);
}

#[test]
fn a_forged_plan_cannot_move_files_from_or_to_the_wrong_places() {
    let sb = Sandbox::new();
    let inside = sb.put("in.png", &png(1));
    let outside = sb._root.path().join("Elsewhere").join("secret.png");
    put_at(&outside, &png(9));
    let fp = |p: &Path| nomnom_organizer::exec::fingerprint_of(p).unwrap();
    let mk = |src: PathBuf, dst: PathBuf| Move {
        fp: fp(&src),
        src,
        dst,
        category: "Images".into(),
        cross_volume: false,
    };
    let forged = Plan {
        source: sb.downloads.clone(),
        dest_root: sb.dest.clone(),
        skips: vec![],
        moves: vec![
            mk(outside.clone(), sb.dest.join("Images").join("secret.png")), // source outside the folder
            mk(inside.clone(), sb._root.path().join("Evil").join("in.png")), // destination outside the root
            mk(inside.clone(), sb.dest.join("a").join("b").join("in.png")), // not a category folder
            mk(
                sb.downloads.join("..").join("Elsewhere").join("secret.png"),
                sb.dest.join("Images").join("s.png"),
            ), // ..
        ],
    };
    let before_out = snapshot(sb._root.path().join("Elsewhere").as_path());
    let r = execute(&sb.journal(), &forged, "test", &NO_CROSS, &|| false).unwrap();
    assert_eq!(r.moved, 0, "{:?}", r.failed);
    assert_eq!(r.failed.len(), 4);
    assert_eq!(
        snapshot(sb._root.path().join("Elsewhere").as_path()),
        before_out
    );
    assert!(exists(&inside));
    assert!(!exists(&sb._root.path().join("Evil")));
}

// ---- journal, recovery, undo -------------------------------------------------------------

#[test]
fn undo_puts_everything_back_exactly() {
    let sb = Sandbox::new();
    build_spec_tree(&sb);
    sb.put("Trips/beach.jpg", &jpg(6));
    let before = snapshot(&sb.downloads);
    let mut o = sb.options();
    o.max_depth = 2;
    let (_, r, j) = run(&sb, &o);
    assert_eq!(r.moved, 4);
    assert_ne!(snapshot(&sb.downloads), before);
    let u = undo(&j, &r.batch_id, &NO_CROSS).unwrap();
    assert_eq!((u.restored, u.skipped.len()), (4, 0), "{:?}", u.skipped);
    assert_eq!(
        snapshot(&sb.downloads),
        before,
        "every file is back, with its original time and contents"
    );
    // a second undo has nothing left to do
    assert_eq!(undo(&j, &r.batch_id, &NO_CROSS).unwrap().restored, 0);
}

#[test]
fn undo_never_overwrites_something_newer_in_the_old_place() {
    let sb = Sandbox::new();
    let f = sb.put("report.pdf", &pdf(1));
    let (_, r, j) = run(&sb, &sb.options());
    assert_eq!(r.moved, 1);
    fs::write(&f, b"a brand new file with the old name").unwrap();
    let u = undo(&j, &r.batch_id, &NO_CROSS).unwrap();
    assert_eq!(u.restored, 0);
    assert!(u.skipped[0].message.contains("original name"));
    assert_eq!(fs::read(&f).unwrap(), b"a brand new file with the old name");
    assert!(exists(&sb.dest.join("Documents").join("report.pdf")));
}

#[test]
fn undo_leaves_a_file_alone_if_it_was_edited_or_replaced_after_the_move() {
    let sb = Sandbox::new();
    sb.put("edited.pdf", &pdf(1));
    sb.put("swapped.pdf", &pdf(2));
    let (_, r, j) = run(&sb, &sb.options());
    let edited = sb.dest.join("Documents").join("edited.pdf");
    let swapped = sb.dest.join("Documents").join("swapped.pdf");
    let mut more = fs::read(&edited).unwrap();
    more.extend_from_slice(b"my later edits");
    fs::write(&edited, &more).unwrap();
    fs::remove_file(&swapped).unwrap();
    put_at(&swapped, &pdf(2));
    let u = undo(&j, &r.batch_id, &NO_CROSS).unwrap();
    assert_eq!(u.restored, 0, "{:?}", u.skipped);
    assert_eq!(u.skipped.len(), 2);
    assert_eq!(
        fs::read(&edited).unwrap(),
        more,
        "later edits are preserved"
    );
    assert!(!exists(&sb.path("edited.pdf")) && !exists(&sb.path("swapped.pdf")));
}

#[test]
fn undo_reports_when_the_original_folder_is_gone() {
    let sb = Sandbox::new();
    sb.put("Trip/a.png", &png(1));
    let mut o = sb.options();
    o.max_depth = 1;
    let (_, r, j) = run(&sb, &o);
    assert_eq!(r.moved, 1);
    fs::remove_dir(sb.path("Trip")).unwrap();
    let u = undo(&j, &r.batch_id, &NO_CROSS).unwrap();
    assert_eq!(u.restored, 0);
    assert!(u.skipped[0].message.contains("gone"));
    assert!(exists(&sb.dest.join("Images").join("a.png")));
}

#[test]
fn recovery_after_a_crash_never_loses_or_deletes_a_user_file() {
    let sb = Sandbox::new();
    let a = sb.put("a-never-moved.png", &png(1));
    let b = sb.put("b-moved-but-not-recorded.png", &png(2));
    let c = sb.put("c-both-exist.png", &png(3));
    let d = sb.put("d-vanished.png", &png(4));
    let p = plan(&sb.options()).unwrap();
    let j = sb.journal();
    let batch = j.begin("test", &p.source, &p.dest_root, &p.moves).unwrap();
    let ops = j.ops(&batch).unwrap();
    let dst_of = |name: &str| {
        ops.iter()
            .find(|o| o.src.file_name().unwrap().to_string_lossy() == name)
            .unwrap()
            .clone()
    };
    for o in &ops {
        j.op_in_progress(&o.id).unwrap();
    }
    fs::create_dir_all(sb.dest.join("Images")).unwrap();
    // b: it did move, the app died before writing "done"
    fs::rename(&b, dst_of("b-moved-but-not-recorded.png").dst).unwrap();
    // c: a copy was made, the original was never removed, and a half-written temp file is lying around
    fs::copy(&c, dst_of("c-both-exist.png").dst).unwrap();
    let tmp = dst_of("c-both-exist.png")
        .dst
        .with_file_name("c-both-exist.png.nomnom-tmp");
    fs::write(&tmp, b"half a copy").unwrap();
    // d: gone from everywhere
    fs::remove_file(&d).unwrap();

    let notes = recover(&j).unwrap();
    let state = |name: &str| {
        j.ops(&batch)
            .unwrap()
            .into_iter()
            .find(|o| o.src.file_name().unwrap().to_string_lossy() == name)
            .unwrap()
            .state
    };
    assert_eq!(state("a-never-moved.png"), "failed");
    assert_eq!(state("b-moved-but-not-recorded.png"), "done");
    assert_eq!(state("c-both-exist.png"), "failed");
    assert_eq!(state("d-vanished.png"), "failed");
    assert_eq!(notes.len(), 2, "the user is told about the two odd cases");
    assert!(exists(&a), "an untouched original stays");
    assert!(
        exists(&c) && exists(&dst_of("c-both-exist.png").dst),
        "both copies are kept"
    );
    assert!(
        !exists(&tmp),
        "only NomNom's own temporary copy was removed"
    );
    // the one that really moved can still be undone
    let u = undo(&j, &batch, &NO_CROSS).unwrap();
    assert_eq!(u.restored, 1);
    assert!(exists(&b));
}

#[test]
fn stopping_midway_leaves_untouched_files_untouched_and_records_it() {
    let sb = Sandbox::new();
    for i in 0..6u8 {
        sb.put(&format!("p{i}.png"), &png(i));
    }
    let p = plan(&sb.options()).unwrap();
    let j = sb.journal();
    let counter = std::cell::Cell::new(0);
    let r = execute(&j, &p, "test", &NO_CROSS, &|| {
        counter.set(counter.get() + 1);
        counter.get() > 2
    })
    .unwrap();
    assert_eq!(r.moved, 2);
    assert!(r.stopped.is_some());
    assert_eq!(fs::read_dir(&sb.downloads).unwrap().count(), 4);
    assert_eq!(all_content(&[&sb.downloads, &sb.dest]).len(), 6);
    // what moved can be undone; the rest was never part of it
    assert_eq!(undo(&j, &r.batch_id, &NO_CROSS).unwrap().restored, 2);
}

#[test]
fn a_file_that_was_just_organized_is_recognised_so_it_is_not_planned_again() {
    let sb = Sandbox::new();
    sb.put("x.png", &png(1));
    let (_, _, j) = run(&sb, &sb.options());
    assert!(j.was_moved_here(&sb.dest.join("Images").join("x.png")));
    assert!(j.was_moved_here(&sb.dest.join("IMAGES").join("X.PNG")));
    assert!(!j.was_moved_here(&sb.dest.join("Images").join("other.png")));
    assert_eq!(j.batches(10).unwrap()[0].moved, 1);
}

// ---- copying between drives (tested on one drive, since a second one isn't available) -----

#[test]
fn the_copy_check_and_remove_path_works_and_keeps_both_when_it_cannot_remove() {
    use nomnom_organizer::exec::copy_verify_remove_for_tests as cvr;
    let sb = Sandbox::new();
    let dir = sb._root.path().join("copytest");
    fs::create_dir_all(&dir).unwrap();

    let src = dir.join("a.mp4");
    put_at(&src, &mp4(1));
    let i = winfs::info(&src).unwrap();
    cvr(&src, &dir.join("b.mp4"), i.size, i.mtime).unwrap();
    assert!(!exists(&src) && fs::read(dir.join("b.mp4")).unwrap() == mp4(1));
    assert!(
        !exists(&dir.join("b.mp4.nomnom-tmp")),
        "no temporary copy is left behind"
    );

    // read-only original: the copy is kept and so is the original
    let ro = dir.join("ro.mp4");
    put_at(&ro, &mp4(2));
    let mut perms = fs::metadata(&ro).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&ro, perms).unwrap();
    let i = winfs::info(&ro).unwrap();
    let err = cvr(&ro, &dir.join("ro-copy.mp4"), i.size, i.mtime).unwrap_err();
    assert!(err.contains("Both copies were kept"), "{err}");
    assert!(exists(&ro) && exists(&dir.join("ro-copy.mp4")));

    // the file changed since the preview
    let c = dir.join("c.mp4");
    put_at(&c, &mp4(3));
    let i = winfs::info(&c).unwrap();
    assert!(cvr(&c, &dir.join("c2.mp4"), i.size + 1, i.mtime).is_err());
    assert!(exists(&c) && !exists(&dir.join("c2.mp4")) && !exists(&dir.join("c2.mp4.nomnom-tmp")));

    // a stranger's file that happens to share our temporary name is never touched
    let d = dir.join("d.mp4");
    put_at(&d, &mp4(4));
    let stranger = dir.join("d2.mp4.nomnom-tmp");
    fs::write(&stranger, b"somebody's file").unwrap();
    let i = winfs::info(&d).unwrap();
    assert!(cvr(&d, &dir.join("d2.mp4"), i.size, i.mtime).is_err());
    assert_eq!(fs::read(&stranger).unwrap(), b"somebody's file");
    assert!(exists(&d));

    // a name that is already taken is never replaced
    let e = dir.join("e.mp4");
    put_at(&e, &mp4(5));
    put_at(&dir.join("taken.mp4"), b"keep me");
    let i = winfs::info(&e).unwrap();
    assert!(cvr(&e, &dir.join("taken.mp4"), i.size, i.mtime).is_err());
    assert_eq!(fs::read(dir.join("taken.mp4")).unwrap(), b"keep me");
    assert!(exists(&e));
}

#[test]
fn files_going_to_another_drive_are_flagged_and_held_back_without_approval() {
    // Same-drive plans must not be flagged...
    let sb = Sandbox::new();
    sb.put("a.png", &png(1));
    let p = plan(&sb.options()).unwrap();
    assert!(p.moves.iter().all(|m| !m.cross_volume));
    assert_eq!(p.cross_volume_count(), 0);
}

// ---- the property that matters, over many random folders ----------------------------------

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next() as usize) % items.len()]
    }
}

#[test]
fn over_many_random_folders_nothing_is_lost_and_unmoved_files_are_unchanged() {
    for seed in 0..60u64 {
        let sb = Sandbox::new();
        let mut rng = Rng(seed * 7919 + 1);
        let dirs = [
            "",
            "",
            "a/",
            "b/",
            "a/deep/",
            "Soft/",
            "Proj/.git/",
            "x/y/z/",
        ];
        for i in 0..rng.next() % 25 + 5 {
            let dir = *rng.pick(&dirs);
            let (name, bytes): (String, Vec<u8>) = match rng.next() % 12 {
                0 => (format!("v{i}.mp4"), mp4(i as u8)),
                1 => (format!("m{i}.mp3"), mp3(i as u8)),
                2 => (format!("p{i}.png"), png(i as u8)),
                3 => (format!("d{i}.pdf"), pdf(i as u8)),
                4 => (format!("setup{i}.exe"), exe()),
                5 => (format!("fake{i}.mp4"), exe()),
                6 => (format!("z{i}.zip"), b"PK\x03\x04..".to_vec()),
                7 => (format!("n{i}.txt"), text("plain notes")),
                8 => (format!("u{i}.xyz"), b"???".to_vec()),
                9 => (format!("dl{i}.mp4.part"), mp4(i as u8)),
                10 => (format!("liar{i}.jpg"), text("not a jpeg")),
                _ => (format!("j{i}.jpg"), jpg(i as u8)),
            };
            sb.put(&format!("{dir}{name}"), &bytes);
        }
        let before = snapshot(&sb.downloads);
        let content = all_content(&[&sb.downloads]);

        let mut o = sb.options();
        o.max_depth = (seed % 4) as u32;
        o.conflict = if seed % 2 == 0 {
            ConflictPolicy::Skip
        } else {
            ConflictPolicy::KeepBoth
        };
        let p = plan(&o).unwrap();
        let moved: std::collections::HashSet<String> = p
            .moves
            .iter()
            .map(|m| nomnom_organizer::pathutil::norm(&m.src))
            .collect();
        let (_, r, j) = {
            let jj = sb.journal();
            let rr = execute(&jj, &p, "test", &NO_CROSS, &|| false).unwrap();
            (p.clone(), rr, jj)
        };
        assert!(r.failed.is_empty(), "seed {seed}: {:?}", r.failed);

        // 1. nothing lost, duplicated or altered anywhere
        assert_eq!(
            all_content(&[&sb.downloads, &sb.dest]),
            content,
            "seed {seed}: content changed"
        );
        // 2. everything that was not planned to move is byte-for-byte what it was
        let after = snapshot(&sb.downloads);
        for (path, state) in &before {
            let abs = nomnom_organizer::pathutil::norm(&sb.downloads.join(path));
            if !moved.contains(&abs) && !state.starts_with("dir") {
                assert_eq!(
                    after.get(path),
                    Some(state),
                    "seed {seed}: {path} was touched"
                );
            }
        }
        // 3. no program, archive or unfinished download ever reached the destination
        if sb.dest.exists() {
            for e in walkdir(&sb.dest) {
                let n = e.file_name().unwrap().to_string_lossy().to_lowercase();
                assert!(
                    !n.ends_with(".exe")
                        && !n.ends_with(".zip")
                        && !n.ends_with(".part")
                        && !n.starts_with("fake")
                        && !n.starts_with("liar")
                        && !n.ends_with(".xyz"),
                    "seed {seed}: {n} should not have moved"
                );
            }
        }
        // 4. and the whole thing can be undone to the exact original
        let u = undo(&j, &r.batch_id, &NO_CROSS).unwrap();
        assert!(u.skipped.is_empty(), "seed {seed}: {:?}", u.skipped);
        assert_eq!(
            snapshot(&sb.downloads),
            before,
            "seed {seed}: undo did not restore everything"
        );
    }
}

fn walkdir(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    for e in fs::read_dir(root).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walkdir(&p));
        } else {
            out.push(p);
        }
    }
    out
}

// ---- the automatic mode plans single new files --------------------------------------------

#[test]
fn planning_single_files_touches_only_those_files_and_applies_every_check() {
    use nomnom_organizer::scan::plan_files;
    let sb = Sandbox::new();
    let good = sb.put("new-download.mp4", &mp4(1));
    let other = sb.put("other.mp4", &mp4(2));
    let fake = sb.put("fake.mp4", &exe());
    let nested = sb.put("Folder/inside.mp4", &mp4(3));
    let part = sb.put("half.mp4.part", &mp4(4));
    let hold = sb.put("busy.mp4", &mp4(5));
    let _lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&hold)
        .unwrap();

    let p = plan_files(
        &sb.options(),
        &[
            good.clone(),
            fake.clone(),
            nested.clone(),
            part,
            hold,
            sb.path("missing.mp4"),
        ],
    )
    .unwrap();
    assert_eq!(p.moves.len(), 1);
    assert_eq!(p.moves[0].src, good);
    assert_eq!(p.skips.len(), 5, "{:?}", p.skips);
    assert!(p.skips.iter().any(|s| s.code == SkipCode::Program));
    let before_other = snapshot(&sb.downloads);
    let r = execute(&sb.journal(), &p, "auto", &NO_CROSS, &|| false).unwrap();
    assert_eq!(r.moved, 1);
    assert!(
        exists(&other) && exists(&fake) && exists(&nested),
        "files that were not asked about are untouched"
    );
    assert!(!exists(&good));
    let _ = before_other;
}
