//! The in-memory picture of a scan: a compact arena of nodes. Folder totals are filled in while
//! the scan runs, so a partial result can already be browsed.

use serde::Serialize;
use std::path::PathBuf;

pub type NodeId = u32;
pub const NONE: NodeId = u32::MAX;

pub const F_DIR: u16 = 1;
/// A junction, symlink or mount point. Recorded, never entered.
pub const F_LINK: u16 = 2;
/// A hardlink to a file already counted elsewhere: its allocated space is 0 here.
pub const F_HARDLINK_DUP: u16 = 4;
/// The name had characters that aren't valid Unicode; operations on it are refused.
pub const F_LOSSY_NAME: u16 = 8;
/// Some of this folder couldn't be read (access denied or locked).
pub const F_INCOMPLETE: u16 = 16;
pub const F_SPARSE: u16 = 32;
pub const F_COMPRESSED: u16 = 64;

pub const ATTR_READONLY: u32 = 0x1;
pub const ATTR_HIDDEN: u32 = 0x2;
pub const ATTR_SYSTEM: u32 = 0x4;
pub const ATTR_DIRECTORY: u32 = 0x10;
pub const ATTR_REPARSE: u32 = 0x400;

#[derive(Clone, Debug)]
pub struct Node {
    pub parent: NodeId,
    pub first_child: NodeId,
    pub next_sibling: NodeId,
    name_off: u32,
    name_len: u16,
    pub flags: u16,
    /// Logical size. For a folder: everything below it.
    pub size: u64,
    /// Space actually used on disk. For a folder: everything below it.
    pub alloc: u64,
    /// Only the files directly in this folder (0 for files).
    pub own_size: u64,
    pub mtime: i64,
    pub ctime: i64,
    pub atime: i64,
    pub attrs: u32,
    /// Files below (inclusive). 0 for a file itself.
    pub files: u32,
    /// Folders below (inclusive).
    pub dirs: u32,
}

impl Node {
    pub fn is_dir(&self) -> bool {
        self.flags & F_DIR != 0
    }
    pub fn is_link(&self) -> bool {
        self.flags & F_LINK != 0
    }
}

/// What the scanner hands over for one directory entry.
pub struct NewEntry {
    pub name: String,
    pub lossy: bool,
    pub flags: u16,
    pub size: u64,
    pub alloc: u64,
    pub mtime: i64,
    pub ctime: i64,
    pub atime: i64,
    pub attrs: u32,
}

#[derive(Default)]
pub struct Index {
    nodes: Vec<Node>,
    names: String,
    /// The path the scan started from, as given (no trailing separator except for a drive root).
    pub root_path: PathBuf,
}

impl Index {
    pub fn new(root_path: PathBuf, root_name: &str) -> Index {
        let mut idx = Index {
            nodes: Vec::new(),
            names: String::new(),
            root_path,
        };
        idx.push(NONE, root_name, F_DIR, 0, 0, 0, 0, 0, ATTR_DIRECTORY);
        idx
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn root(&self) -> NodeId {
        0
    }
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn name(&self, id: NodeId) -> &str {
        let n = &self.nodes[id as usize];
        &self.names[n.name_off as usize..n.name_off as usize + n.name_len as usize]
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        parent: NodeId,
        name: &str,
        flags: u16,
        size: u64,
        alloc: u64,
        mtime: i64,
        ctime: i64,
        atime: i64,
        attrs: u32,
    ) -> NodeId {
        let name = if name.len() > u16::MAX as usize {
            // Names this long can't exist on Windows (255 characters); guard anyway.
            &name[..name.floor_char_boundary(u16::MAX as usize)]
        } else {
            name
        };
        let off = self.names.len() as u32;
        self.names.push_str(name);
        let id = self.nodes.len() as NodeId;
        self.nodes.push(Node {
            parent,
            first_child: NONE,
            next_sibling: NONE,
            name_off: off,
            name_len: name.len() as u16,
            flags,
            size,
            alloc,
            own_size: 0,
            mtime,
            ctime,
            atime,
            attrs,
            files: 0,
            dirs: 0,
        });
        id
    }

    /// Adds everything found in one directory and rolls the totals up to every ancestor once.
    /// Returns the ids of the new subfolders that still have to be scanned.
    pub fn add_children(
        &mut self,
        parent: NodeId,
        entries: Vec<NewEntry>,
        incomplete: bool,
    ) -> Vec<NodeId> {
        let mut to_scan = Vec::new();
        let (mut own_size, mut own_alloc, mut files, mut dirs) = (0u64, 0u64, 0u32, 0u32);
        for e in entries {
            let mut flags = e.flags;
            if e.lossy {
                flags |= F_LOSSY_NAME;
            }
            let is_dir = flags & F_DIR != 0;
            let id = self.push(
                parent, &e.name, flags, e.size, e.alloc, e.mtime, e.ctime, e.atime, e.attrs,
            );
            let next = self.nodes[parent as usize].first_child;
            self.nodes[id as usize].next_sibling = next;
            self.nodes[parent as usize].first_child = id;
            if is_dir {
                dirs += 1;
                if flags & F_LINK == 0 {
                    to_scan.push(id);
                } else {
                    // A link counts as a folder entry but contributes no size.
                    self.nodes[id as usize].size = 0;
                    self.nodes[id as usize].alloc = 0;
                }
            } else {
                files += 1;
                own_size += e.size;
                own_alloc += e.alloc;
            }
        }
        if incomplete {
            self.nodes[parent as usize].flags |= F_INCOMPLETE;
        }
        self.nodes[parent as usize].own_size += own_size;
        let mut cur = parent;
        while cur != NONE {
            let n = &mut self.nodes[cur as usize];
            n.size += own_size;
            n.alloc += own_alloc;
            n.files += files;
            n.dirs += dirs;
            cur = n.parent;
        }
        to_scan
    }

    /// Children of `id`, in storage order.
    pub fn children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let mut cur = self.nodes[id as usize].first_child;
        std::iter::from_fn(move || {
            if cur == NONE {
                return None;
            }
            let this = cur;
            cur = self.nodes[cur as usize].next_sibling;
            Some(this)
        })
    }

    /// Full path of a node, built from the root path and the names above it.
    pub fn path_of(&self, id: NodeId) -> PathBuf {
        let mut parts = Vec::new();
        let mut cur = id;
        while cur != NONE && cur != 0 {
            parts.push(self.name(cur));
            cur = self.nodes[cur as usize].parent;
        }
        let mut p = self.root_path.clone();
        for part in parts.iter().rev() {
            p.push(part);
        }
        p
    }

    /// Finds the node for a path inside the scan (names compared without regard to case).
    pub fn resolve_path(&self, path: &std::path::Path) -> Option<NodeId> {
        let norm = |p: &std::path::Path| {
            p.to_string_lossy()
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_lowercase()
        };
        let (p, root) = (norm(path), norm(&self.root_path));
        if p == root {
            return Some(0);
        }
        let rest = p.strip_prefix(&format!("{root}\\"))?;
        let mut cur = 0;
        for part in rest.split('\\') {
            cur = self
                .children(cur)
                .find(|c| self.name(*c).to_lowercase() == part)?;
        }
        Some(cur)
    }

    /// Takes one item (and everything below it) out of the index, correcting every total above it.
    /// Used after a delete or move so the picture stays right without scanning again.
    pub fn remove_node(&mut self, id: NodeId) {
        let Some(n) = self.nodes.get(id as usize) else {
            return;
        };
        let (parent, size, alloc, files, dirs, is_dir, next) = (
            n.parent,
            n.size,
            n.alloc,
            n.files,
            n.dirs,
            n.is_dir(),
            n.next_sibling,
        );
        if parent == NONE {
            return;
        }
        let first = self.nodes[parent as usize].first_child;
        if first == id {
            self.nodes[parent as usize].first_child = next;
        } else {
            let mut cur = first;
            while cur != NONE {
                if self.nodes[cur as usize].next_sibling == id {
                    self.nodes[cur as usize].next_sibling = next;
                    break;
                }
                cur = self.nodes[cur as usize].next_sibling;
            }
        }
        self.nodes[id as usize].parent = NONE;
        if !is_dir {
            let p = &mut self.nodes[parent as usize];
            p.own_size = p.own_size.saturating_sub(size);
        }
        let (df, dd) = if is_dir { (files, dirs + 1) } else { (1, 0) };
        let mut cur = parent;
        while cur != NONE {
            let a = &mut self.nodes[cur as usize];
            a.size = a.size.saturating_sub(size);
            a.alloc = a.alloc.saturating_sub(alloc);
            a.files = a.files.saturating_sub(df);
            a.dirs = a.dirs.saturating_sub(dd);
            cur = a.parent;
        }
    }

    /// Gives an item a new name in place.
    pub fn rename_node(&mut self, id: NodeId, new_name: &str) {
        if new_name.len() > u16::MAX as usize || id as usize >= self.nodes.len() {
            return;
        }
        let off = self.names.len() as u32;
        self.names.push_str(new_name);
        let n = &mut self.nodes[id as usize];
        n.name_off = off;
        n.name_len = new_name.len() as u16;
    }

    /// The chain from the root to `id`, root first.
    pub fn ancestors(&self, id: NodeId) -> Vec<NodeId> {
        let mut v = Vec::new();
        let mut cur = id;
        while cur != NONE {
            v.push(cur);
            cur = self.nodes[cur as usize].parent;
        }
        v.reverse();
        v
    }

    /// Replaces everything below `id` with a fresh scan result (used when only one folder is
    /// rescanned). The old children stay in the arena as unreachable garbage until the next full
    /// scan; their totals are subtracted from the ancestors.
    pub fn detach_children(&mut self, id: NodeId) {
        let (size, alloc, files, dirs) = {
            let n = &self.nodes[id as usize];
            (n.size, n.alloc, n.files, n.dirs)
        };
        let mut cur = self.nodes[id as usize].parent;
        while cur != NONE {
            let n = &mut self.nodes[cur as usize];
            n.size = n.size.saturating_sub(size);
            n.alloc = n.alloc.saturating_sub(alloc);
            n.files = n.files.saturating_sub(files);
            n.dirs = n.dirs.saturating_sub(dirs);
            cur = n.parent;
        }
        let n = &mut self.nodes[id as usize];
        n.size = 0;
        n.alloc = 0;
        n.own_size = 0;
        n.files = 0;
        n.dirs = 0;
        n.first_child = NONE;
        n.flags &= !F_INCOMPLETE;
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct NodeView {
    pub id: NodeId,
    pub name: String,
    pub is_dir: bool,
    pub is_link: bool,
    pub size: u64,
    pub alloc: u64,
    pub files: u32,
    pub dirs: u32,
    /// Seconds since 1970, or null when unknown.
    pub mtime: Option<i64>,
    pub ctime: Option<i64>,
    pub atime: Option<i64>,
    pub attrs: u32,
    pub flags: u16,
    /// Share of the parent folder's size, 0 to 100.
    pub percent: f32,
    pub parent: Option<NodeId>,
}

/// Windows FILETIME ticks (100 ns since 1601) to Unix seconds.
pub fn filetime_to_unix(t: i64) -> Option<i64> {
    if t <= 0 {
        None
    } else {
        Some(t / 10_000_000 - 11_644_473_600)
    }
}

impl Index {
    pub fn view(&self, id: NodeId) -> NodeView {
        let n = &self.nodes[id as usize];
        let parent_size = if n.parent == NONE {
            n.size
        } else {
            self.nodes[n.parent as usize].size
        };
        NodeView {
            id,
            name: self.name(id).to_string(),
            is_dir: n.is_dir(),
            is_link: n.is_link(),
            size: n.size,
            alloc: n.alloc,
            files: n.files,
            dirs: n.dirs,
            mtime: filetime_to_unix(n.mtime),
            ctime: filetime_to_unix(n.ctime),
            atime: filetime_to_unix(n.atime),
            attrs: n.attrs,
            flags: n.flags,
            percent: if parent_size == 0 {
                0.0
            } else {
                (n.size as f64 / parent_size as f64 * 100.0) as f32
            },
            parent: (n.parent != NONE).then_some(n.parent),
        }
    }
}
