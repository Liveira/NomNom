//! Renaming many files at once: a pattern such as `{date}_{name}` or `Holiday {n:3}`, a preview of
//! every new name before anything changes, and an apply step that cannot overwrite a file and can
//! be undone. Only the base name changes; each file keeps its own extension.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// What the caller knows about one file. The dates are already in the person's time zone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileInfo {
    pub path: PathBuf,
    /// The name without the extension.
    pub stem: String,
    /// The extension without the dot ("" if there is none).
    pub ext: String,
    /// The folder's name.
    pub parent: String,
    /// `YYYY-MM-DD` of the last change.
    pub date: String,
    /// `HHMMSS` of the last change.
    pub time: String,
    /// Seconds since 1970 of the last change (to put files in order).
    pub modified: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Case {
    #[default]
    Keep,
    Lower,
    Upper,
    /// Each Word Starts With A Capital.
    Title,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Spaces {
    #[default]
    Keep,
    Underscore,
    Dash,
    Dot,
    Remove,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    /// As the files were given.
    #[default]
    Given,
    Name,
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Options {
    /// The new base name. `{name}` is the old one, `{n}` or `{n:3}` a counter, `{date}`, `{time}`,
    /// `{year}`, `{month}`, `{day}`, `{today}` and `{parent}` fill in the rest.
    pub template: String,
    /// Text to find in the old name, and what to put there instead.
    pub find: String,
    pub replace: String,
    pub case: Case,
    pub spaces: Spaces,
    pub order: Order,
    /// The first counter value and how much it grows.
    pub start: u32,
    pub step: u32,
    /// The extension in lower case.
    pub lower_ext: bool,
    /// Today's date as `YYYY-MM-DD`, for `{today}`.
    pub today: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            template: "{name}".into(),
            find: String::new(),
            replace: String::new(),
            case: Case::Keep,
            spaces: Spaces::Keep,
            order: Order::Given,
            start: 1,
            step: 1,
            lower_ext: false,
            today: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Will be renamed.
    Ok,
    /// The name stays as it is.
    Same,
    /// Two files would get the same name, or the name is already taken.
    Conflict,
    /// The result is not a usable name.
    Invalid,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Item {
    pub from: PathBuf,
    pub from_name: String,
    pub to_name: String,
    pub status: Status,
    /// A short reason, when the status is not Ok.
    pub note: String,
}

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];
const MAX_NAME: usize = 200;

/// Makes `name` safe as a file name on every system. Returns `None` if nothing usable is left.
pub fn clean(name: &str) -> Option<String> {
    let mut s: String = name
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    s = s.trim().trim_end_matches('.').trim_end().to_string();
    if s.chars().count() > MAX_NAME {
        s = s.chars().take(MAX_NAME).collect();
    }
    if s.is_empty() || s == "." || s == ".." {
        return None;
    }
    if RESERVED.contains(&s.to_ascii_uppercase().as_str()) {
        s.push('_');
    }
    Some(s)
}

fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut new_word = true;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if new_word {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            new_word = false;
        } else {
            out.push(c);
            new_word = true;
        }
    }
    out
}

/// Fills in the template for one file. Unknown `{words}` are kept as they are, and reported.
fn fill(template: &str, f: &FileInfo, name: &str, counter: u64, today: &str) -> (String, bool) {
    let mut out = String::new();
    let mut unknown = false;
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let token = &after[..close];
        let (word, arg) = match token.split_once(':') {
            Some((w, a)) => (w, Some(a)),
            None => (token, None),
        };
        let date_part = |i: usize| f.date.split('-').nth(i).unwrap_or("").to_string();
        match word {
            "name" => out.push_str(name),
            "ext" => out.push_str(&f.ext),
            "n" => {
                let width = arg
                    .and_then(|a| a.parse::<usize>().ok())
                    .unwrap_or(1)
                    .min(12);
                out.push_str(&format!("{counter:0width$}"));
            }
            "date" => out.push_str(&f.date),
            "time" => out.push_str(&f.time),
            "year" => out.push_str(&date_part(0)),
            "month" => out.push_str(&date_part(1)),
            "day" => out.push_str(&date_part(2)),
            "today" => out.push_str(today),
            "parent" => out.push_str(&f.parent),
            _ => {
                unknown = true;
                out.push('{');
                out.push_str(token);
                out.push('}');
            }
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    (out, unknown)
}

fn apply_style(name: &str, o: &Options) -> String {
    let spaced = match o.spaces {
        Spaces::Keep => name.to_string(),
        Spaces::Underscore => name.replace(' ', "_"),
        Spaces::Dash => name.replace(' ', "-"),
        Spaces::Dot => name.replace(' ', "."),
        Spaces::Remove => name.replace(' ', ""),
    };
    match o.case {
        Case::Keep => spaced,
        Case::Lower => spaced.to_lowercase(),
        Case::Upper => spaced.to_uppercase(),
        Case::Title => title_case(&spaced),
    }
}

/// The new name of every file, in the order they were given, and whether each can be applied.
/// `taken` says whether a path already exists on disk (a file that is itself being renamed doesn't
/// count: it is moving out of the way).
pub fn preview(files: &[FileInfo], o: &Options, taken: &dyn Fn(&Path) -> bool) -> Vec<Item> {
    // The counter follows the chosen order, but the answer comes back in the given order.
    let mut order: Vec<usize> = (0..files.len()).collect();
    match o.order {
        Order::Given => {}
        Order::Name => order.sort_by_key(|&i| files[i].stem.to_lowercase()),
        Order::Modified => {
            order.sort_by_key(|&i| (files[i].modified, files[i].stem.to_lowercase()))
        }
    }
    let mut counter_of = vec![0u64; files.len()];
    for (rank, &i) in order.iter().enumerate() {
        counter_of[i] = u64::from(o.start) + rank as u64 * u64::from(o.step.max(1));
    }

    let sources: HashSet<String> = files.iter().map(|f| key(&f.path)).collect();
    let mut items: Vec<Item> = Vec::with_capacity(files.len());
    for (i, f) in files.iter().enumerate() {
        let name = if o.find.is_empty() {
            f.stem.clone()
        } else {
            f.stem.replace(&o.find, &o.replace)
        };
        let (filled, unknown) = fill(&o.template, f, &name, counter_of[i], &o.today);
        let styled = apply_style(&filled, o);
        let ext = if o.lower_ext {
            f.ext.to_lowercase()
        } else {
            f.ext.clone()
        };
        let from_name = file_name(f);
        let (to_name, status, note) = match clean(&styled) {
            None => (
                from_name.clone(),
                Status::Invalid,
                "That leaves no name.".to_string(),
            ),
            Some(base) => {
                let full = if ext.is_empty() {
                    base
                } else {
                    format!("{base}.{ext}")
                };
                if unknown {
                    (
                        full,
                        Status::Invalid,
                        "The pattern has a word NomNom doesn't know.".to_string(),
                    )
                } else if full == from_name {
                    (full, Status::Same, String::new())
                } else {
                    (full, Status::Ok, String::new())
                }
            }
        };
        items.push(Item {
            from: f.path.clone(),
            from_name,
            to_name,
            status,
            note,
        });
    }

    // Two files with one new name, or a name that something else already has.
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, item) in items.iter().enumerate() {
        if item.status != Status::Ok {
            continue;
        }
        let target = item.from.with_file_name(&item.to_name);
        let k = key(&target);
        *seen.entry(k).or_insert(0) += 1;
        let _ = i;
    }
    for item in items.iter_mut() {
        if item.status != Status::Ok {
            continue;
        }
        let target = item.from.with_file_name(&item.to_name);
        let k = key(&target);
        if seen.get(&k).copied().unwrap_or(0) > 1 {
            item.status = Status::Conflict;
            item.note = "Another file gets the same name.".into();
        } else if !sources.contains(&k) && taken(&target) {
            item.status = Status::Conflict;
            item.note = "A file with this name is already there.".into();
        }
    }
    items
}

fn file_name(f: &FileInfo) -> String {
    if f.ext.is_empty() {
        f.stem.clone()
    } else {
        format!("{}.{}", f.stem, f.ext)
    }
}

/// Names are compared without regard to case, so `A.txt` and `a.txt` are one name.
fn key(p: &Path) -> String {
    p.to_string_lossy().to_lowercase()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Moved {
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub struct Outcome {
    pub done: Vec<Moved>,
    pub failed: Vec<(PathBuf, String)>,
}

/// Renames what `items` says. A file is never overwritten, and names that swap places (a→b and
/// b→a) work: every file first moves to a temporary name, then to its new one. If something fails
/// in the second step the file goes back to its old name.
pub fn apply(items: &[Item]) -> Outcome {
    let mut out = Outcome::default();
    let todo: Vec<&Item> = items.iter().filter(|i| i.status == Status::Ok).collect();
    let mut staged: Vec<(PathBuf, PathBuf, PathBuf)> = Vec::new(); // (original, temp, final)
    for (n, item) in todo.iter().enumerate() {
        let temp = item
            .from
            .with_file_name(format!(".nomnom-rename-{}-{n}.tmp", std::process::id()));
        let target = item.from.with_file_name(&item.to_name);
        match std::fs::rename(&item.from, &temp) {
            Ok(()) => staged.push((item.from.clone(), temp, target)),
            Err(e) => out.failed.push((item.from.clone(), e.to_string())),
        }
    }
    for (original, temp, target) in staged {
        // Never replace something that appeared since the preview.
        let result = if target.exists() && key(&target) != key(&temp) {
            Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "A file with this name is already there.",
            ))
        } else {
            std::fs::rename(&temp, &target)
        };
        match result {
            Ok(()) => out.done.push(Moved {
                from: original,
                to: target,
            }),
            Err(e) => {
                let _ = std::fs::rename(&temp, &original);
                out.failed.push((original, e.to_string()));
            }
        }
    }
    out
}

/// Puts files back to the names they had. Uses the same two steps, so it works for swaps too.
pub fn undo(done: &[Moved]) -> Outcome {
    let items: Vec<Item> = done
        .iter()
        .map(|m| Item {
            from: m.to.clone(),
            from_name: m
                .to
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            to_name: m
                .from
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            status: Status::Ok,
            note: String::new(),
        })
        .collect();
    apply(&items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(dir: &Path, name: &str, date: &str, modified: i64) -> FileInfo {
        let (stem, ext) = match name.rsplit_once('.') {
            Some((s, e)) => (s.to_string(), e.to_string()),
            None => (name.to_string(), String::new()),
        };
        FileInfo {
            path: dir.join(name),
            stem,
            ext,
            parent: "Trip".into(),
            date: date.into(),
            time: "101530".into(),
            modified,
        }
    }
    fn opts(template: &str) -> Options {
        Options {
            template: template.into(),
            today: "2026-11-19".into(),
            ..Options::default()
        }
    }
    fn names(items: &[Item]) -> Vec<String> {
        items.iter().map(|i| i.to_name.clone()).collect()
    }
    const FREE: &dyn Fn(&Path) -> bool = &|_| false;

    #[test]
    fn the_pattern_fills_in_dates_names_and_counters() {
        let d = Path::new("/p");
        let files = vec![
            info(d, "IMG_01.jpg", "2026-03-05", 1),
            info(d, "IMG_02.jpg", "2026-03-06", 2),
        ];
        let got = preview(&files, &opts("{date}_{name}"), FREE);
        assert_eq!(
            names(&got),
            vec!["2026-03-05_IMG_01.jpg", "2026-03-06_IMG_02.jpg"]
        );
        let got = preview(&files, &opts("{parent} {n:3}"), FREE);
        assert_eq!(names(&got), vec!["Trip 001.jpg", "Trip 002.jpg"]);
        let got = preview(&files, &opts("{year}{month}{day}-{today}"), FREE);
        assert_eq!(
            names(&got),
            vec!["20260305-2026-11-19.jpg", "20260306-2026-11-19.jpg"]
        );
    }

    #[test]
    fn the_counter_can_start_and_step_and_follow_a_chosen_order() {
        let d = Path::new("/p");
        let files = vec![
            info(d, "b.txt", "2026-01-01", 30),
            info(d, "a.txt", "2026-01-01", 20),
            info(d, "c.txt", "2026-01-01", 10),
        ];
        let mut o = opts("File {n}");
        o.start = 10;
        o.step = 5;
        assert_eq!(
            names(&preview(&files, &o, FREE)),
            vec!["File 10.txt", "File 15.txt", "File 20.txt"]
        );
        o.order = Order::Name; // a, b, c
        assert_eq!(
            names(&preview(&files, &o, FREE)),
            vec!["File 15.txt", "File 10.txt", "File 20.txt"]
        );
        o.order = Order::Modified; // c, a, b
        assert_eq!(
            names(&preview(&files, &o, FREE)),
            vec!["File 20.txt", "File 15.txt", "File 10.txt"]
        );
    }

    #[test]
    fn find_and_replace_case_and_spaces_work_on_the_name() {
        let d = Path::new("/p");
        let files = vec![info(d, "my holiday photo.JPG", "2026-01-01", 1)];
        let mut o = opts("{name}");
        o.find = "holiday".into();
        o.replace = "trip".into();
        o.spaces = Spaces::Dash;
        o.case = Case::Title;
        o.lower_ext = true;
        assert_eq!(names(&preview(&files, &o, FREE)), vec!["My-Trip-Photo.jpg"]);
    }

    #[test]
    fn unsafe_names_are_made_safe_or_refused() {
        assert_eq!(clean("a/b:c?").as_deref(), Some("a_b_c_"));
        assert_eq!(clean("  name. ").as_deref(), Some("name"));
        assert_eq!(clean("con").as_deref(), Some("con_"));
        assert_eq!(clean("..."), None);
        assert_eq!(clean(""), None);
        let d = Path::new("/p");
        let got = preview(&[info(d, "a.txt", "2026-01-01", 1)], &opts("{nope}"), FREE);
        assert_eq!(got[0].status, Status::Invalid);
        let got = preview(&[info(d, "a.txt", "2026-01-01", 1)], &opts("..."), FREE);
        assert_eq!(got[0].status, Status::Invalid);
    }

    #[test]
    fn two_files_with_one_new_name_or_a_taken_name_are_conflicts() {
        let d = Path::new("/p");
        let files = vec![
            info(d, "a.txt", "2026-01-01", 1),
            info(d, "b.txt", "2026-01-01", 2),
        ];
        let got = preview(&files, &opts("same"), FREE);
        assert!(got.iter().all(|i| i.status == Status::Conflict));
        let taken: &dyn Fn(&Path) -> bool = &|p| p.ends_with("x.txt");
        let got = preview(&files[..1], &opts("x"), taken);
        assert_eq!(got[0].status, Status::Conflict);
        let got = preview(&files[..1], &opts("{name}"), FREE);
        assert_eq!(got[0].status, Status::Same);
    }

    #[test]
    fn case_only_changes_are_not_conflicts_with_themselves() {
        let d = Path::new("/p");
        let files = vec![info(d, "photo.txt", "2026-01-01", 1)];
        let mut o = opts("{name}");
        o.case = Case::Upper;
        let always: &dyn Fn(&Path) -> bool = &|_| true;
        // The file is its own source, so the "taken" check does not apply to it.
        let got = preview(&files, &o, always);
        assert_eq!(got[0].to_name, "PHOTO.txt");
        assert_eq!(got[0].status, Status::Ok);
    }

    fn make(dir: &Path, name: &str, body: &str) -> FileInfo {
        std::fs::write(dir.join(name), body).unwrap();
        info(dir, name, "2026-01-01", 1)
    }

    #[test]
    fn applying_renames_files_and_undo_puts_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![
            make(dir.path(), "a.txt", "A"),
            make(dir.path(), "b.txt", "B"),
        ];
        let items = preview(&files, &opts("Doc {n}"), &|p| p.exists());
        let out = apply(&items);
        assert_eq!(out.done.len(), 2);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("Doc 1.txt")).unwrap(),
            "A"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("Doc 2.txt")).unwrap(),
            "B"
        );
        let back = undo(&out.done);
        assert!(back.failed.is_empty());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "A"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("b.txt")).unwrap(),
            "B"
        );
    }

    #[test]
    fn names_that_swap_places_work_and_nothing_is_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let a = make(dir.path(), "a.txt", "A");
        let b = make(dir.path(), "b.txt", "B");
        let items = vec![
            Item {
                from: a.path.clone(),
                from_name: "a.txt".into(),
                to_name: "b.txt".into(),
                status: Status::Ok,
                note: String::new(),
            },
            Item {
                from: b.path.clone(),
                from_name: "b.txt".into(),
                to_name: "a.txt".into(),
                status: Status::Ok,
                note: String::new(),
            },
        ];
        let out = apply(&items);
        assert_eq!(out.done.len(), 2);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "B"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("b.txt")).unwrap(),
            "A"
        );

        // Something that appeared since the preview is never replaced.
        let c = make(dir.path(), "c.txt", "C");
        std::fs::write(dir.path().join("d.txt"), "D").unwrap();
        let clash = vec![Item {
            from: c.path.clone(),
            from_name: "c.txt".into(),
            to_name: "d.txt".into(),
            status: Status::Ok,
            note: String::new(),
        }];
        let out = apply(&clash);
        assert_eq!(out.failed.len(), 1);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("c.txt")).unwrap(),
            "C",
            "back where it was"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("d.txt")).unwrap(),
            "D"
        );
    }

    #[test]
    fn items_that_are_not_ok_are_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let f = make(dir.path(), "a.txt", "A");
        let mut items = preview(&[f], &opts("{nope}"), FREE);
        assert_eq!(items[0].status, Status::Invalid);
        assert!(apply(&items).done.is_empty());
        items[0].status = Status::Conflict;
        assert!(apply(&items).done.is_empty());
        assert!(dir.path().join("a.txt").exists());
    }
}
