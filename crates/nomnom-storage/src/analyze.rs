//! Questions asked of a finished (or partial) index: a folder's children, the biggest files
//! anywhere, the biggest folders, and totals by kind.

use crate::index::*;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    Name,
    Size,
    Alloc,
    Files,
    Dirs,
    Modified,
    Percent,
}

#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Filter {
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    /// Lower-case extensions without the dot.
    pub extensions: Vec<String>,
    pub category: Option<String>,
    /// Unix seconds.
    pub modified_after: Option<i64>,
    pub modified_before: Option<i64>,
    pub name_contains: Option<String>,
}

pub fn extension_of(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => name[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// Broad kind of a file, from its extension. The same table colours the treemap.
pub fn category_of(ext: &str) -> &'static str {
    match ext {
        "mp4" | "m4v" | "mkv" | "mov" | "webm" | "avi" | "wmv" | "flv" | "mpg" | "mpeg" | "3gp"
        | "ts" | "vob" => "video",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff" | "ico" | "heic"
        | "avif" | "raw" | "cr2" | "nef" | "psd" | "svg" => "image",
        "mp3" | "m4a" | "aac" | "wav" | "flac" | "ogg" | "oga" | "opus" | "wma" | "aiff"
        | "aif" | "mid" => "audio",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
        | "rtf" | "txt" | "md" | "csv" | "epub" | "json" | "xml" | "html" | "htm" => "document",
        "exe" | "dll" | "msi" | "sys" | "bat" | "cmd" | "ps1" | "jar" | "com" | "scr" | "ocx"
        | "drv" => "executable",
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" | "zst" | "cab" | "iso" | "img"
        | "vhd" | "vhdx" | "wim" | "dmg" => "archive",
        "tmp" | "temp" | "log" | "bak" | "old" | "dmp" | "etl" | "cache" | "part"
        | "crdownload" => "temp",
        _ => "other",
    }
}

pub const CATEGORIES: [&str; 8] = [
    "video",
    "image",
    "audio",
    "document",
    "executable",
    "archive",
    "temp",
    "other",
];

fn matches(idx: &Index, id: NodeId, f: &Filter) -> bool {
    let n = idx.node(id).unwrap();
    if let Some(m) = f.min_size {
        if n.size < m {
            return false;
        }
    }
    if let Some(m) = f.max_size {
        if n.size > m {
            return false;
        }
    }
    if f.modified_after.is_some() || f.modified_before.is_some() {
        let Some(t) = filetime_to_unix(n.mtime) else {
            return false;
        };
        if f.modified_after.is_some_and(|a| t < a) || f.modified_before.is_some_and(|b| t > b) {
            return false;
        }
    }
    if !f.extensions.is_empty() || f.category.is_some() {
        let ext = extension_of(idx.name(id));
        if !f.extensions.is_empty() && !f.extensions.contains(&ext) {
            return false;
        }
        if f.category
            .as_deref()
            .is_some_and(|c| category_of(&ext) != c)
        {
            return false;
        }
    }
    if let Some(s) = &f.name_contains {
        if !idx.name(id).to_lowercase().contains(&s.to_lowercase()) {
            return false;
        }
    }
    true
}

#[derive(Serialize)]
pub struct Page {
    pub total: usize,
    pub items: Vec<NodeView>,
}

/// One page of a folder's children, sorted. `text` narrows by name.
pub fn children_page(
    idx: &Index,
    id: NodeId,
    sort: SortKey,
    desc: bool,
    offset: usize,
    limit: usize,
    text: Option<&str>,
) -> Page {
    let needle = text.filter(|t| !t.is_empty()).map(str::to_lowercase);
    let mut ids: Vec<NodeId> = idx
        .children(id)
        .filter(|c| {
            needle
                .as_ref()
                .is_none_or(|n| idx.name(*c).to_lowercase().contains(n))
        })
        .collect();
    let key = |c: &NodeId| -> (i128, String) {
        let n = idx.node(*c).unwrap();
        match sort {
            SortKey::Name => (0, idx.name(*c).to_lowercase()),
            SortKey::Size | SortKey::Percent => (n.size as i128, String::new()),
            SortKey::Alloc => (n.alloc as i128, String::new()),
            SortKey::Files => (n.files as i128, String::new()),
            SortKey::Dirs => (n.dirs as i128, String::new()),
            SortKey::Modified => (n.mtime as i128, String::new()),
        }
    };
    ids.sort_by(|a, b| {
        let ord = key(a).cmp(&key(b));
        // Ties (and equal sizes) fall back to the name so the order is stable between calls.
        let ord = ord.then_with(|| {
            idx.name(*a)
                .to_lowercase()
                .cmp(&idx.name(*b).to_lowercase())
        });
        if desc {
            ord.reverse()
        } else {
            ord
        }
    });
    let total = ids.len();
    let items = ids
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|c| idx.view(c))
        .collect();
    Page { total, items }
}

/// Visits every node below `scope` (inclusive), without recursion.
fn walk(idx: &Index, scope: NodeId, mut f: impl FnMut(NodeId)) {
    let mut stack = vec![scope];
    while let Some(id) = stack.pop() {
        f(id);
        stack.extend(idx.children(id));
    }
}

/// The `n` biggest files under `scope` that pass the filter, biggest first.
pub fn top_files(idx: &Index, scope: NodeId, n: usize, filter: &Filter) -> Vec<NodeView> {
    let mut heap: BinaryHeap<Reverse<(u64, NodeId)>> = BinaryHeap::new();
    walk(idx, scope, |id| {
        let node = idx.node(id).unwrap();
        if node.is_dir() || !matches(idx, id, filter) {
            return;
        }
        if heap.len() < n {
            heap.push(Reverse((node.size, id)));
        } else if let Some(Reverse((smallest, _))) = heap.peek() {
            if node.size > *smallest {
                heap.pop();
                heap.push(Reverse((node.size, id)));
            }
        }
    });
    let mut v: Vec<(u64, NodeId)> = heap.into_iter().map(|Reverse(x)| x).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|(_, id)| idx.view(id)).collect()
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FolderMode {
    /// Everything below the folder. A parent always outweighs its children, so these overlap.
    Inclusive,
    /// Only the files directly inside the folder. Nothing is counted twice.
    Exclusive,
}

/// The `n` biggest folders under `scope`.
pub fn top_folders(
    idx: &Index,
    scope: NodeId,
    n: usize,
    mode: FolderMode,
    min_size: u64,
) -> Vec<NodeView> {
    let mut heap: BinaryHeap<Reverse<(u64, NodeId)>> = BinaryHeap::new();
    walk(idx, scope, |id| {
        let node = idx.node(id).unwrap();
        if !node.is_dir() || node.is_link() {
            return;
        }
        let value = match mode {
            FolderMode::Inclusive => node.size,
            FolderMode::Exclusive => node.own_size,
        };
        if value < min_size || value == 0 {
            return;
        }
        if heap.len() < n {
            heap.push(Reverse((value, id)));
        } else if let Some(Reverse((smallest, _))) = heap.peek() {
            if value > *smallest {
                heap.pop();
                heap.push(Reverse((value, id)));
            }
        }
    });
    let mut v: Vec<(u64, NodeId)> = heap.into_iter().map(|Reverse(x)| x).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter()
        .map(|(value, id)| {
            let mut view = idx.view(id);
            if mode == FolderMode::Exclusive {
                // Show what was ranked, so the number on screen is the number used for the order.
                view.size = value;
            }
            view
        })
        .collect()
}

#[derive(Serialize, Debug, Clone)]
pub struct Group {
    pub key: String,
    pub size: u64,
    pub alloc: u64,
    pub files: u64,
}

/// Totals by extension under `scope`, biggest first.
pub fn by_extension(idx: &Index, scope: NodeId, limit: usize) -> Vec<Group> {
    let mut map: std::collections::HashMap<String, Group> = std::collections::HashMap::new();
    walk(idx, scope, |id| {
        let n = idx.node(id).unwrap();
        if n.is_dir() {
            return;
        }
        let ext = extension_of(idx.name(id));
        let g = map.entry(ext.clone()).or_insert(Group {
            key: ext,
            size: 0,
            alloc: 0,
            files: 0,
        });
        g.size += n.size;
        g.alloc += n.alloc;
        g.files += 1;
    });
    let mut v: Vec<Group> = map.into_values().collect();
    v.sort_by(|a, b| b.size.cmp(&a.size).then(a.key.cmp(&b.key)));
    v.truncate(limit);
    v
}

/// Totals by broad kind (video, image, ...) under `scope`.
pub fn by_category(idx: &Index, scope: NodeId) -> Vec<Group> {
    let mut v: Vec<Group> = CATEGORIES
        .iter()
        .map(|c| Group {
            key: (*c).to_string(),
            size: 0,
            alloc: 0,
            files: 0,
        })
        .collect();
    walk(idx, scope, |id| {
        let n = idx.node(id).unwrap();
        if n.is_dir() {
            return;
        }
        let cat = category_of(&extension_of(idx.name(id)));
        let g = v.iter_mut().find(|g| g.key == cat).unwrap();
        g.size += n.size;
        g.alloc += n.alloc;
        g.files += 1;
    });
    v.sort_by_key(|g| Reverse(g.size));
    v
}

#[derive(Serialize, Debug, Clone)]
pub struct TreeView {
    pub id: NodeId,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub alloc: u64,
    /// Broad kind for colouring (files only; folders are "folder").
    pub category: &'static str,
    pub children: Vec<TreeView>,
    /// Smaller children that were folded together to keep the picture readable.
    pub hidden_count: u32,
    pub hidden_size: u64,
    pub hidden_alloc: u64,
}

/// A nested view for the treemap: `depth` levels below `node`, at most `max_children` biggest
/// children at each level (the rest are counted in `hidden_*`).
pub fn treemap(idx: &Index, node: NodeId, depth: u32, max_children: usize) -> TreeView {
    let n = idx.node(node).unwrap();
    let is_dir = n.is_dir();
    let mut v = TreeView {
        id: node,
        name: idx.name(node).to_string(),
        is_dir,
        size: n.size,
        alloc: n.alloc,
        category: if is_dir {
            "folder"
        } else {
            category_of(&extension_of(idx.name(node)))
        },
        children: Vec::new(),
        hidden_count: 0,
        hidden_size: 0,
        hidden_alloc: 0,
    };
    if !is_dir || depth == 0 || n.is_link() {
        return v;
    }
    let mut kids: Vec<NodeId> = idx
        .children(node)
        .filter(|c| idx.node(*c).unwrap().size > 0)
        .collect();
    kids.sort_by_key(|c| Reverse(idx.node(*c).unwrap().size));
    for (i, c) in kids.into_iter().enumerate() {
        if i < max_children {
            v.children.push(treemap(idx, c, depth - 1, max_children));
        } else {
            let cn = idx.node(c).unwrap();
            v.hidden_count += 1;
            v.hidden_size += cn.size;
            v.hidden_alloc += cn.alloc;
        }
    }
    v
}

/// Totals of files by how long ago they were last changed (or created, or opened), under `scope`.
/// `now` is Unix seconds. Files with no date go in "No date".
pub fn by_age(idx: &Index, scope: NodeId, field: crate::search::DateField, now: i64) -> Vec<Group> {
    const DAY: i64 = 86_400;
    let buckets: [(&str, i64); 6] = [
        ("Last 7 days", 7 * DAY),
        ("8 to 30 days", 30 * DAY),
        ("31 to 90 days", 90 * DAY),
        ("3 months to 1 year", 365 * DAY),
        ("1 to 3 years", 3 * 365 * DAY),
        ("Over 3 years", i64::MAX),
    ];
    let mut v: Vec<Group> = buckets
        .iter()
        .map(|(k, _)| Group {
            key: (*k).to_string(),
            size: 0,
            alloc: 0,
            files: 0,
        })
        .collect();
    v.push(Group {
        key: "No date".into(),
        size: 0,
        alloc: 0,
        files: 0,
    });
    walk(idx, scope, |id| {
        let n = idx.node(id).unwrap();
        if n.is_dir() {
            return;
        }
        let t = filetime_to_unix(match field {
            crate::search::DateField::Created => n.ctime,
            crate::search::DateField::Modified => n.mtime,
            crate::search::DateField::Accessed => n.atime,
        });
        let slot = match t {
            None => v.len() - 1,
            Some(t) => {
                let age = (now - t).max(0);
                buckets
                    .iter()
                    .position(|(_, limit)| age <= *limit)
                    .unwrap_or(5)
            }
        };
        v[slot].size += n.size;
        v[slot].alloc += n.alloc;
        v[slot].files += 1;
    });
    v
}
