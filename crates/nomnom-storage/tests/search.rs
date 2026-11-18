use nomnom_storage::analyze;
use nomnom_storage::search::{compile, run, DateField, Query, TextMode};
use nomnom_storage::{ScanOptions, Session};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

const NOW: i64 = 1_800_000_000;

fn at(days_ago: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(NOW as u64 - days_ago * 86_400)
}

fn write(path: &Path, bytes: usize, days_ago: u64) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![1u8; bytes]).unwrap();
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(at(days_ago))
        .unwrap();
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

fn tree() -> (tempfile::TempDir, Arc<Session>) {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    write(&r.join("Movies/holiday.mp4"), 5000, 400);
    write(&r.join("Movies/new.mp4"), 4000, 3);
    write(&r.join("Movies/small.mp4"), 100, 500);
    write(&r.join("Docs/Report Final.pdf"), 800, 40);
    write(&r.join("Docs/notes.txt"), 50, 10);
    write(&r.join("Docs/archive/old.zip"), 2000, 1500);
    write(&r.join("IMG_0001.JPG"), 300, 100);
    let s = scan(r);
    (d, s)
}

fn names(s: &Session, q: &Query) -> Vec<String> {
    let c = compile(q, NOW).unwrap();
    let idx = s.index.read().unwrap();
    run(&idx, 0, &c, 100, &AtomicBool::new(false))
        .items
        .into_iter()
        .map(|v| v.name)
        .collect()
}

fn name(text: &str, mode: TextMode) -> Query {
    Query::Name {
        text: text.into(),
        mode,
    }
}

#[test]
fn names_match_by_text_wildcard_and_regex_without_regard_to_case() {
    let (_d, s) = tree();
    assert_eq!(
        names(&s, &name("REPORT", TextMode::Contains)),
        ["Report Final.pdf"]
    );
    let mut w = names(&s, &name("*.mp4", TextMode::Wildcard));
    w.sort();
    assert_eq!(w, ["holiday.mp4", "new.mp4", "small.mp4"]);
    assert_eq!(
        names(&s, &name("img_00??.jpg", TextMode::Wildcard)),
        ["IMG_0001.JPG"]
    );
    assert_eq!(
        names(&s, &name(r"^notes\.(txt|md)$", TextMode::Regex)),
        ["notes.txt"]
    );
    assert!(
        names(&s, &name("*.mp", TextMode::Wildcard)).is_empty(),
        "a wildcard must match the whole name"
    );
}

#[test]
fn the_example_from_the_request_big_old_videos() {
    let (_d, s) = tree();
    let q = Query::And {
        items: vec![
            Query::Ext {
                values: vec!["mp4".into()],
            },
            Query::Size {
                min: Some(1000),
                max: None,
            },
            Query::Age {
                field: DateField::Modified,
                older_than_days: Some(180),
                newer_than_days: None,
            },
        ],
    };
    assert_eq!(names(&s, &q), ["holiday.mp4"]);
}

#[test]
fn or_and_not_combine_conditions() {
    let (_d, s) = tree();
    let q = Query::Or {
        items: vec![
            Query::Ext {
                values: vec!["pdf".into()],
            },
            Query::Ext {
                values: vec![".ZIP".into()],
            },
        ],
    };
    let mut got = names(&s, &q);
    got.sort();
    assert_eq!(got, ["Report Final.pdf", "old.zip"]);
    let not_video = Query::And {
        items: vec![
            Query::Item {
                what: "file".into(),
            },
            Query::Not {
                item: Box::new(Query::Ext {
                    values: vec!["mp4".into()],
                }),
            },
            Query::Size {
                min: Some(1000),
                max: None,
            },
        ],
    };
    assert_eq!(names(&s, &not_video), ["old.zip"]);
}

#[test]
fn path_dates_and_kind_conditions_work() {
    let (_d, s) = tree();
    let in_archive = Query::And {
        items: vec![
            Query::Path {
                text: "docs\\archive".into(),
                mode: TextMode::Contains,
            },
            Query::Item {
                what: "file".into(),
            },
        ],
    };
    assert_eq!(names(&s, &in_archive), ["old.zip"]);
    assert_eq!(
        names(
            &s,
            &Query::Path {
                text: "docs\\archive".into(),
                mode: TextMode::Contains
            }
        )
        .len(),
        2,
        "the folder itself matches too"
    );
    let recent = Query::Date {
        field: DateField::Modified,
        after: Some(NOW - 30 * 86_400),
        before: None,
    };
    let mut got = names(
        &s,
        &Query::And {
            items: vec![
                recent,
                Query::Item {
                    what: "file".into(),
                },
            ],
        },
    );
    got.sort();
    assert_eq!(got, ["new.mp4", "notes.txt"]);
    assert_eq!(
        names(
            &s,
            &Query::Kind {
                category: "archive".into()
            }
        ),
        ["old.zip"]
    );
    assert_eq!(
        names(
            &s,
            &Query::Item {
                what: "folder".into()
            }
        )
        .len(),
        3
    );
}

#[test]
fn attributes_can_be_required_or_excluded() {
    let d = tempfile::tempdir().unwrap();
    #[cfg(windows)]
    let secret_name = {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::File::options()
            .write(true)
            .create_new(true)
            .attributes(0x2)
            .open(d.path().join("secret.txt"))
            .unwrap();
        "secret.txt"
    };
    #[cfg(not(windows))]
    let secret_name = {
        std::fs::write(d.path().join(".secret.txt"), b"x").unwrap();
        ".secret.txt"
    };
    std::fs::write(d.path().join("plain.txt"), b"x").unwrap();
    let s = scan(d.path());
    assert_eq!(
        names(
            &s,
            &Query::Attr {
                name: "hidden".into(),
                set: true
            }
        ),
        [secret_name]
    );
    assert_eq!(
        names(
            &s,
            &Query::Attr {
                name: "hidden".into(),
                set: false
            }
        ),
        ["plain.txt"]
    );
}

#[test]
fn bad_queries_are_explained_and_results_are_ranked_and_capped() {
    let (_d, s) = tree();
    assert!(compile(&name("(unclosed", TextMode::Regex), NOW)
        .err()
        .unwrap()
        .contains("isn't valid"));
    assert!(compile(
        &Query::Attr {
            name: "sticky".into(),
            set: true
        },
        NOW
    )
    .is_err());
    assert!(compile(
        &Query::Item {
            what: "device".into()
        },
        NOW
    )
    .is_err());
    let c = compile(
        &Query::Item {
            what: "file".into(),
        },
        NOW,
    )
    .unwrap();
    let idx = s.index.read().unwrap();
    let found = run(&idx, 0, &c, 3, &AtomicBool::new(false));
    assert_eq!(found.total, 7);
    assert!(found.truncated);
    assert_eq!(
        found.items.iter().map(|v| v.size).collect::<Vec<_>>(),
        [5000, 4000, 2000]
    );
}

#[test]
fn a_deeply_nested_query_is_refused() {
    let mut q = Query::Item {
        what: "file".into(),
    };
    for _ in 0..40 {
        q = Query::Not { item: Box::new(q) };
    }
    assert!(compile(&q, NOW).err().unwrap().contains("nested"));
}

#[test]
fn files_are_grouped_by_age() {
    let (_d, s) = tree();
    let idx = s.index.read().unwrap();
    let g = analyze::by_age(&idx, 0, DateField::Modified, NOW);
    let by: std::collections::HashMap<_, _> = g
        .iter()
        .map(|x| (x.key.as_str(), (x.files, x.size)))
        .collect();
    assert_eq!(by["Last 7 days"], (1, 4000));
    assert_eq!(by["8 to 30 days"], (1, 50));
    assert_eq!(by["31 to 90 days"], (1, 800));
    assert_eq!(by["3 months to 1 year"], (1, 300));
    assert_eq!(by["1 to 3 years"], (2, 5100)); // 400 and 500 days
    assert_eq!(by["Over 3 years"], (1, 2000));
    assert_eq!(g.iter().map(|x| x.files).sum::<u64>(), 7);
}
