//! Comparing two folders: which files are only on one side, which are the same, and which differ (and
//! which side is newer). It also decides which paths are safe to act on. Reading the folders and
//! copying files is the app's job; nothing here touches the disk, and nothing here deletes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One file, named by its path inside the folder with `/` between the parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub rel: String,
    pub size: u64,
    /// Seconds since 1970.
    pub modified: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    OnlyLeft,
    OnlyRight,
    Same,
    LeftNewer,
    RightNewer,
    /// Different content with the same time: nobody can say which is newer.
    Different,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Row {
    pub rel: String,
    pub status: Status,
    pub left: Option<(u64, i64)>,
    pub right: Option<(u64, i64)>,
}

/// Two times within this many seconds count as the same moment (file systems round differently).
pub const SAME_TIME_SLACK: i64 = 2;

fn key(rel: &str) -> String {
    rel.to_lowercase()
}

/// Compares by size and time. A same-size pair with different times is reported as newer/older; the
/// app may then read both and turn it into `Same` when the content is identical.
pub fn compare(left: &[Entry], right: &[Entry]) -> Vec<Row> {
    let mut map: BTreeMap<String, (Option<&Entry>, Option<&Entry>)> = BTreeMap::new();
    for e in left {
        map.entry(key(&e.rel)).or_default().0 = Some(e);
    }
    for e in right {
        map.entry(key(&e.rel)).or_default().1 = Some(e);
    }
    map.into_values()
        .map(|(l, r)| match (l, r) {
            (Some(l), None) => Row {
                rel: l.rel.clone(),
                status: Status::OnlyLeft,
                left: Some((l.size, l.modified)),
                right: None,
            },
            (None, Some(r)) => Row {
                rel: r.rel.clone(),
                status: Status::OnlyRight,
                left: None,
                right: Some((r.size, r.modified)),
            },
            (Some(l), Some(r)) => {
                let dt = l.modified - r.modified;
                let status = if l.size == r.size && dt.abs() <= SAME_TIME_SLACK {
                    Status::Same
                } else if dt > SAME_TIME_SLACK {
                    Status::LeftNewer
                } else if dt < -SAME_TIME_SLACK {
                    Status::RightNewer
                } else {
                    Status::Different
                };
                Row {
                    rel: l.rel.clone(),
                    status,
                    left: Some((l.size, l.modified)),
                    right: Some((r.size, r.modified)),
                }
            }
            (None, None) => unreachable!("a key always has a side"),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Copy the left file to the right side.
    CopyRight,
    /// Copy the right file to the left side.
    CopyLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// The right folder becomes a copy of the left one (nothing is deleted from it).
    ToRight,
    ToLeft,
    /// Each side gets what the other has, and the newer of two different files wins.
    Both,
}

/// What a sync in this direction would do for a row, or `None` to leave it alone.
pub fn suggest(row: &Row, dir: Direction) -> Option<Action> {
    use Action::*;
    use Direction::*;
    match (row.status, dir) {
        (Status::Same, _) | (Status::Different, _) => None,
        (Status::OnlyLeft, ToRight | Both) => Some(CopyRight),
        (Status::OnlyRight, ToLeft | Both) => Some(CopyLeft),
        (Status::LeftNewer, ToRight | Both) => Some(CopyRight),
        (Status::RightNewer, ToLeft | Both) => Some(CopyLeft),
        _ => None,
    }
}

/// Does copying for this row replace a file that is already there?
pub fn replaces(row: &Row, action: Action) -> bool {
    match action {
        Action::CopyRight => row.right.is_some(),
        Action::CopyLeft => row.left.is_some(),
    }
}

/// Is this a path inside a folder and nothing more? No `..`, no drive or root, no empty parts, no
/// characters Windows refuses, and no reserved device names.
pub fn safe_rel(rel: &str) -> bool {
    if rel.is_empty()
        || rel.len() > 1000
        || rel.starts_with('/')
        || rel.contains('\\')
        || rel.contains(':')
        || rel.contains('\0')
    {
        return false;
    }
    rel.split('/').all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && !part.ends_with('.')
            && !part.ends_with(' ')
            && !part.contains(['<', '>', '"', '|', '?', '*'])
            && !is_reserved(part)
    })
}

fn is_reserved(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part).to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(rel: &str, size: u64, modified: i64) -> Entry {
        Entry {
            rel: rel.into(),
            size,
            modified,
        }
    }

    #[test]
    fn files_are_sorted_into_the_six_kinds() {
        let left = vec![
            e("a.txt", 1, 100),
            e("same.txt", 5, 200),
            e("new-left.txt", 2, 500),
            e("old-left.txt", 2, 100),
            e("tie.txt", 3, 300),
        ];
        let right = vec![
            e("b.txt", 1, 100),
            e("same.txt", 5, 201),
            e("new-left.txt", 2, 100),
            e("old-left.txt", 2, 500),
            e("tie.txt", 4, 300),
        ];
        let rows = compare(&left, &right);
        let get = |rel: &str| rows.iter().find(|r| r.rel == rel).unwrap().status;
        assert_eq!(get("a.txt"), Status::OnlyLeft);
        assert_eq!(get("b.txt"), Status::OnlyRight);
        assert_eq!(get("same.txt"), Status::Same);
        assert_eq!(get("new-left.txt"), Status::LeftNewer);
        assert_eq!(get("old-left.txt"), Status::RightNewer);
        assert_eq!(get("tie.txt"), Status::Different);
        assert_eq!(rows.len(), 6);
    }

    #[test]
    fn names_that_differ_only_in_case_are_one_file() {
        let rows = compare(
            &[e("Docs/Readme.TXT", 1, 10)],
            &[e("docs/readme.txt", 1, 10)],
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, Status::Same);
    }

    #[test]
    fn a_sync_never_overwrites_with_something_older() {
        let left_newer = Row {
            rel: "x".into(),
            status: Status::LeftNewer,
            left: Some((1, 9)),
            right: Some((1, 1)),
        };
        assert_eq!(
            suggest(&left_newer, Direction::ToRight),
            Some(Action::CopyRight)
        );
        assert_eq!(
            suggest(&left_newer, Direction::Both),
            Some(Action::CopyRight)
        );
        assert_eq!(suggest(&left_newer, Direction::ToLeft), None);
        let right_newer = Row {
            status: Status::RightNewer,
            ..left_newer.clone()
        };
        assert_eq!(suggest(&right_newer, Direction::ToRight), None);
        assert_eq!(
            suggest(&right_newer, Direction::Both),
            Some(Action::CopyLeft)
        );
    }

    #[test]
    fn extra_files_are_copied_over_and_never_removed() {
        let only_right = Row {
            rel: "x".into(),
            status: Status::OnlyRight,
            left: None,
            right: Some((1, 1)),
        };
        assert_eq!(suggest(&only_right, Direction::ToRight), None);
        assert_eq!(
            suggest(&only_right, Direction::ToLeft),
            Some(Action::CopyLeft)
        );
        let same = Row {
            status: Status::Same,
            ..only_right.clone()
        };
        assert_eq!(suggest(&same, Direction::Both), None);
        let tie = Row {
            status: Status::Different,
            ..only_right
        };
        assert_eq!(suggest(&tie, Direction::Both), None);
    }

    #[test]
    fn replacing_is_told_apart_from_adding() {
        let both = Row {
            rel: "x".into(),
            status: Status::LeftNewer,
            left: Some((1, 9)),
            right: Some((1, 1)),
        };
        assert!(replaces(&both, Action::CopyRight));
        let only_left = Row {
            status: Status::OnlyLeft,
            right: None,
            ..both
        };
        assert!(!replaces(&only_left, Action::CopyRight));
    }

    #[test]
    fn only_paths_inside_the_folder_are_safe() {
        for ok in ["a.txt", "dir/sub/file.tar.gz", "Ünï/ça.txt", "a b/c d.txt"] {
            assert!(safe_rel(ok), "{ok}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "a/../../x",
            "a//b",
            "C:/x",
            "a\\b",
            "./x",
            "a/./b",
            "con",
            "dir/NUL.txt",
            "lpt1",
            "a/b.",
            "a/b ",
            "x?y",
            "a|b",
            "a\0b",
        ] {
            assert!(!safe_rel(bad), "{bad}");
        }
    }
}
