//! Advanced search over a scan. A query is a small tree: AND, OR and NOT over conditions on the
//! name, path, extension, size, dates, attributes, owner, path length, empty folders and kind.
//! Matching never touches the disk.

use crate::analyze::{category_of, extension_of};
use crate::index::*;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextMode {
    #[default]
    Contains,
    /// `*` and `?`, matched against the whole text.
    Wildcard,
    Regex,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DateField {
    Created,
    Modified,
    Accessed,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Query {
    And {
        items: Vec<Query>,
    },
    Or {
        items: Vec<Query>,
    },
    Not {
        item: Box<Query>,
    },
    Name {
        text: String,
        #[serde(default)]
        mode: TextMode,
    },
    Path {
        text: String,
        #[serde(default)]
        mode: TextMode,
    },
    Owner {
        text: String,
        #[serde(default)]
        mode: TextMode,
    },
    PathLen {
        min: Option<u32>,
        max: Option<u32>,
    },
    EmptyFolder,
    Ext {
        values: Vec<String>,
    },
    Size {
        min: Option<u64>,
        max: Option<u64>,
    },
    /// Unix seconds.
    Date {
        field: DateField,
        after: Option<i64>,
        before: Option<i64>,
    },
    /// Older or newer than a number of days, counted back from now.
    Age {
        field: DateField,
        older_than_days: Option<u32>,
        newer_than_days: Option<u32>,
    },
    /// "hidden", "system", "readonly", "archive", "compressed", "encrypted", "offline", "sparse", "link", "hardlink", "streams".
    Attr {
        name: String,
        set: bool,
    },
    Kind {
        category: String,
    },
    /// "file" or "folder".
    Item {
        what: String,
    },
}

enum C {
    And(Vec<C>),
    Or(Vec<C>),
    Not(Box<C>),
    Name(Matcher),
    Path(Matcher),
    Owner(Matcher),
    PathLen(Option<u32>, Option<u32>),
    EmptyFolder,
    Ext(Vec<String>),
    Size(Option<u64>, Option<u64>),
    Date(DateField, Option<i64>, Option<i64>),
    Attr(u32, u16, bool),
    Kind(String),
    Folder(bool),
}

enum Matcher {
    Contains(String),
    Re(Regex),
}

impl Matcher {
    fn is(&self, s: &str) -> bool {
        match self {
            Matcher::Contains(n) => s.to_lowercase().contains(n),
            Matcher::Re(r) => r.is_match(s),
        }
    }
}

pub struct Compiled {
    root: C,
    needs_path: bool,
}

fn wildcard_to_regex(p: &str) -> String {
    let mut out = String::from("^");
    for ch in p.chars() {
        match ch {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            c => out.push_str(&regex::escape(&c.to_string())),
        }
    }
    out.push('$');
    out
}

fn matcher(text: &str, mode: TextMode) -> Result<Matcher, String> {
    match mode {
        TextMode::Contains => Ok(Matcher::Contains(text.to_lowercase())),
        TextMode::Wildcard => RegexBuilder::new(&wildcard_to_regex(text))
            .case_insensitive(true)
            .build()
            .map(Matcher::Re)
            .map_err(|e| e.to_string()),
        TextMode::Regex => RegexBuilder::new(text)
            .case_insensitive(true)
            // A pattern can't be allowed to grow without bound.
            .size_limit(1 << 20)
            .build()
            .map(Matcher::Re)
            .map_err(|e| format!("That pattern isn't valid: {e}")),
    }
}

/// Checks a query and prepares it (patterns are built once). `now` is Unix seconds.
pub fn compile(q: &Query, now: i64) -> Result<Compiled, String> {
    fn go(q: &Query, needs_path: &mut bool, depth: usize, now: i64) -> Result<C, String> {
        if depth > 32 {
            return Err("That search is nested too deeply.".into());
        }
        Ok(match q {
            Query::And { items } => C::And(
                items
                    .iter()
                    .map(|i| go(i, needs_path, depth + 1, now))
                    .collect::<Result<_, _>>()?,
            ),
            Query::Or { items } => C::Or(
                items
                    .iter()
                    .map(|i| go(i, needs_path, depth + 1, now))
                    .collect::<Result<_, _>>()?,
            ),
            Query::Not { item } => C::Not(Box::new(go(item, needs_path, depth + 1, now)?)),
            Query::Name { text, mode } => C::Name(matcher(text, *mode)?),
            Query::Path { text, mode } => {
                *needs_path = true;
                C::Path(matcher(text, *mode)?)
            }
            Query::Owner { text, mode } => C::Owner(matcher(text, *mode)?),
            Query::PathLen { min, max } => C::PathLen(*min, *max),
            Query::EmptyFolder => C::EmptyFolder,
            Query::Ext { values } => C::Ext(
                values
                    .iter()
                    .map(|v| v.trim_start_matches('.').to_ascii_lowercase())
                    .collect(),
            ),
            Query::Size { min, max } => C::Size(*min, *max),
            Query::Date {
                field,
                after,
                before,
            } => C::Date(*field, *after, *before),
            Query::Age {
                field,
                older_than_days,
                newer_than_days,
            } => {
                let day = 86_400i64;
                let before = older_than_days.map(|d| now - i64::from(d) * day);
                let after = newer_than_days.map(|d| now - i64::from(d) * day);
                C::Date(*field, after, before)
            }
            Query::Attr { name, set } => {
                let (mask, flag) = match name.as_str() {
                    "hidden" => (ATTR_HIDDEN, 0),
                    "system" => (ATTR_SYSTEM, 0),
                    "readonly" => (ATTR_READONLY, 0),
                    "archive" => (ATTR_ARCHIVE, 0),
                    "encrypted" => (ATTR_ENCRYPTED, 0),
                    "offline" => (ATTR_OFFLINE_FLAG, 0),
                    "compressed" => (0x800, F_COMPRESSED),
                    "sparse" => (0x200, F_SPARSE),
                    "link" => (0, F_LINK),
                    "hardlink" => (0, F_HARDLINK_DUP),
                    "streams" => (0, F_HAS_ADS),
                    other => return Err(format!("Unknown attribute \"{other}\".")),
                };
                C::Attr(mask, flag, *set)
            }
            Query::Kind { category } => C::Kind(category.clone()),
            Query::Item { what } => match what.as_str() {
                "file" => C::Folder(false),
                "folder" => C::Folder(true),
                other => return Err(format!("Unknown item type \"{other}\".")),
            },
        })
    }
    let mut needs_path = false;
    let root = go(q, &mut needs_path, 0, now)?;
    Ok(Compiled { root, needs_path })
}

fn eval(c: &C, idx: &Index, id: NodeId, path: &str) -> bool {
    let n = idx.node(id).unwrap();
    match c {
        C::And(v) => v.iter().all(|x| eval(x, idx, id, path)),
        C::Or(v) => v.iter().any(|x| eval(x, idx, id, path)),
        C::Not(x) => !eval(x, idx, id, path),
        C::Name(m) => m.is(idx.name(id)),
        C::Path(m) => m.is(path),
        C::Owner(m) => idx.owner_of(id).is_some_and(|o| m.is(o)),
        C::PathLen(min, max) => {
            let len = idx.path_len(id);
            min.is_none_or(|m| len >= m) && max.is_none_or(|m| len <= m)
        }
        C::EmptyFolder => n.is_dir() && !n.is_link() && n.files == 0 && n.dirs == 0,
        C::Ext(v) => !n.is_dir() && v.contains(&extension_of(idx.name(id))),
        C::Size(min, max) => min.is_none_or(|m| n.size >= m) && max.is_none_or(|m| n.size <= m),
        C::Date(f, after, before) => {
            let t = filetime_to_unix(match f {
                DateField::Created => n.ctime,
                DateField::Modified => n.mtime,
                DateField::Accessed => n.atime,
            });
            match t {
                None => false,
                Some(t) => after.is_none_or(|a| t >= a) && before.is_none_or(|b| t <= b),
            }
        }
        C::Attr(mask, flag, set) => {
            let has = (*mask != 0 && n.attrs & mask != 0) || (*flag != 0 && n.flags & flag != 0);
            has == *set
        }
        C::Kind(cat) => !n.is_dir() && category_of(&extension_of(idx.name(id))) == cat,
        C::Folder(want) => n.is_dir() == *want,
    }
}

#[derive(Serialize)]
pub struct Found {
    pub total: usize,
    pub truncated: bool,
    pub items: Vec<NodeView>,
}

/// Runs a compiled query below `scope`. Results are biggest first and capped at `limit`.
pub fn run(idx: &Index, scope: NodeId, q: &Compiled, limit: usize, cancel: &AtomicBool) -> Found {
    let mut hits: Vec<(u64, NodeId)> = Vec::new();
    let mut stack = vec![scope];
    let mut count = 0usize;
    while let Some(id) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        stack.extend(idx.children(id));
        if id == scope && scope == 0 {
            continue; // the scan root itself isn't a result
        }
        let path = if q.needs_path {
            idx.path_of(id).to_string_lossy().to_string()
        } else {
            String::new()
        };
        if eval(&q.root, idx, id, &path) {
            count += 1;
            hits.push((idx.node(id).unwrap().size, id));
            if hits.len() >= limit.max(1) * 2 {
                hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                hits.truncate(limit);
            }
        }
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    hits.truncate(limit);
    Found {
        total: count,
        truncated: count > limit,
        items: hits.into_iter().map(|(_, id)| idx.view(id)).collect(),
    }
}
