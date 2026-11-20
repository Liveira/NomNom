//! The PATH variable: the list of folders programs are searched in. This reads it into entries,
//! marks the ones that repeat, point nowhere or are empty, and puts a list back together. Reading and
//! writing the system's value is the Windows crate's job.

use serde::Serialize;

pub const MAX_LEN: usize = 32_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Issue {
    /// The same folder appears earlier in the list.
    Duplicate,
    /// The folder is not there.
    Missing,
    /// An empty piece (two semicolons in a row, or one at the end).
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// As written, so `%USERPROFILE%\bin` stays that way when saved.
    pub raw: String,
    /// With the `%NAME%` parts filled in.
    pub expanded: String,
    pub issue: Option<Issue>,
}

/// The pieces of a PATH value, in order. Empty pieces are kept so they can be shown and removed.
pub fn split(path: &str) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }
    path.split(';').map(|p| p.trim().to_string()).collect()
}

pub fn join(entries: &[String]) -> String {
    entries.join(";")
}

fn key(expanded: &str) -> String {
    expanded
        .trim()
        .trim_matches('"')
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

/// Reads a PATH value. `expand` fills in `%NAME%`; `exists` says whether a folder is there.
pub fn analyze(
    raw: &str,
    expand: &dyn Fn(&str) -> String,
    exists: &dyn Fn(&str) -> bool,
) -> Vec<Entry> {
    let mut seen: Vec<String> = Vec::new();
    split(raw)
        .into_iter()
        .map(|piece| {
            let expanded = expand(&piece);
            let issue = if piece.is_empty() {
                Some(Issue::Empty)
            } else {
                let k = key(&expanded);
                if seen.contains(&k) {
                    Some(Issue::Duplicate)
                } else {
                    seen.push(k);
                    // A name that still has a `%` in it could not be filled in; it is not judged missing.
                    (!expanded.contains('%') && !exists(expanded.trim().trim_matches('"')))
                        .then_some(Issue::Missing)
                }
            };
            Entry {
                raw: piece,
                expanded,
                issue,
            }
        })
        .collect()
}

/// The list without its empty, repeated and missing entries, in the same order.
pub fn cleaned(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter(|e| e.issue.is_none())
        .map(|e| e.raw.clone())
        .collect()
}

/// Checks a list before it is written: no empty entries, no `;` or line breaks inside one, and a value
/// that fits what Windows allows.
pub fn validate(entries: &[String]) -> Result<(), String> {
    for e in entries {
        if e.trim().is_empty() {
            return Err("There is an empty entry. Remove it first.".into());
        }
        if e.contains([';', '\n', '\r', '\0']) {
            return Err(format!("\"{e}\" can't be an entry."));
        }
    }
    if join(entries).len() > MAX_LEN {
        return Err("That list is too long for Windows to keep.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(s: &str) -> String {
        s.replace("%USERPROFILE%", "C:\\Users\\a")
    }
    fn exists(p: &str) -> bool {
        !p.contains("gone")
    }

    #[test]
    fn a_path_is_read_in_order_with_its_problems_marked() {
        let raw = "C:\\Tools;%USERPROFILE%\\bin;C:\\tools\\;C:\\gone;;C:\\Users\\a\\bin";
        let e = analyze(raw, &expand, &exists);
        let issues: Vec<_> = e.iter().map(|x| x.issue).collect();
        assert_eq!(
            issues,
            vec![
                None,
                None,
                Some(Issue::Duplicate),
                Some(Issue::Missing),
                Some(Issue::Empty),
                Some(Issue::Duplicate)
            ]
        );
        assert_eq!(e[1].raw, "%USERPROFILE%\\bin");
        assert_eq!(e[1].expanded, "C:\\Users\\a\\bin");
    }

    #[test]
    fn cleaning_keeps_the_first_of_each_and_the_order() {
        let e = analyze("C:\\b;C:\\a;C:\\B;C:\\gone;", &expand, &exists);
        assert_eq!(cleaned(&e), vec!["C:\\b", "C:\\a"]);
    }

    #[test]
    fn a_name_that_cannot_be_filled_in_is_not_called_missing() {
        let e = analyze("%UNKNOWN%\\bin", &|s| s.to_string(), &|_| false);
        assert_eq!(e[0].issue, None);
    }

    #[test]
    fn an_empty_path_has_no_entries() {
        assert!(split("").is_empty());
        assert!(analyze("", &expand, &exists).is_empty());
    }

    #[test]
    fn a_list_is_joined_back_the_way_it_was() {
        let raw = "C:\\a;%USERPROFILE%\\bin;D:\\c";
        assert_eq!(join(&split(raw)), raw);
    }

    #[test]
    fn bad_lists_are_refused() {
        assert!(validate(&["C:\\a".into(), "C:\\b".into()]).is_ok());
        assert!(validate(&["C:\\a".into(), " ".into()]).is_err());
        assert!(validate(&["C:\\a;C:\\b".into()]).is_err());
        assert!(validate(&["C:\\a\nC:\\b".into()]).is_err());
        assert!(validate(&[format!("C:\\{}", "x".repeat(MAX_LEN))]).is_err());
    }
}
