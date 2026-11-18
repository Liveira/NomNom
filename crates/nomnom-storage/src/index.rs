//! The in-memory picture of a scan: a compact arena of nodes. Folder totals are filled in while
//! the scan runs, so a partial result can already be browsed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub type NodeId = u32;
pub const NONE: NodeId = u32::MAX;
pub const NO_OWNER: u16 = u16::MAX;

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
/// Has NTFS alternate data streams beyond the default stream.
pub const F_HAS_ADS: u16 = 128;

pub const ATTR_READONLY: u32 = 0x1;
pub const ATTR_HIDDEN: u32 = 0x2;
pub const ATTR_SYSTEM: u32 = 0x4;
pub const ATTR_DIRECTORY: u32 = 0x10;
pub const ATTR_ARCHIVE: u32 = 0x20;
pub const ATTR_REPARSE: u32 = 0x400;
pub const ATTR_OFFLINE_FLAG: u32 = 0x1000;
pub const ATTR_ENCRYPTED: u32 = 0x4000;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Space on disk of only the files directly in this folder (0 for files).
    pub own_alloc: u64,
    /// Count of files directly in this folder (0 for files).
    pub own_files: u32,
    pub mtime: i64,
    pub ctime: i64,
    pub atime: i64,
    pub attrs: u32,
    /// Files below (inclusive). 0 for a file itself.
    pub files: u32,
    /// Folders below (inclusive).
    pub dirs: u32,
    /// Index into `Index::owners`, or `NO_OWNER`.
    pub owner_id: u16,
    /// Alternate data streams count (excluding `::$DATA`).
    pub ads_count: u16,
    /// Extra bytes stored in alternate data streams.
    pub ads_size: u64,
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
    pub owner: Option<String>,
    pub ads_count: u16,
    pub ads_size: u64,
}

#[derive(Serialize, Deserialize)]
pub struct IndexArchive {
    pub version: u32,
    pub root_path: String,
    pub names: String,
    pub owners: Vec<String>,
    pub nodes: Vec<Node>,
}

#[derive(Default)]
pub struct Index {
    nodes: Vec<Node>,
    names: String,
    owners: Vec<String>,
    owner_map: HashMap<String, u16>,
    /// The path the scan started from, as given (no trailing separator except for a drive root).
    pub root_path: PathBuf,
    root_utf16_len: u32,
}

fn path_code_units(p: &std::path::Path) -> u32 {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        p.as_os_str().encode_wide().count() as u32
    }
    #[cfg(not(windows))]
    {
        p.to_string_lossy().chars().count() as u32
    }
}

impl Index {
    pub fn new(root_path: PathBuf, root_name: &str) -> Index {
        let root_utf16_len = path_code_units(&root_path);
        let mut idx = Index {
            nodes: Vec::new(),
            names: String::new(),
            owners: Vec::new(),
            owner_map: HashMap::new(),
            root_path,
            root_utf16_len,
        };
        idx.push(
            NONE,
            root_name,
            F_DIR,
            0,
            0,
            0,
            0,
            0,
            ATTR_DIRECTORY,
            NO_OWNER,
            0,
            0,
        );
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

    pub fn owner_of(&self, id: NodeId) -> Option<&str> {
        let oid = self.nodes.get(id as usize)?.owner_id;
        if oid == NO_OWNER {
            None
        } else {
            self.owners.get(oid as usize).map(String::as_str)
        }
    }

    pub fn intern_owner(&mut self, owner: &str) -> u16 {
        if owner.is_empty() {
            return NO_OWNER;
        }
        if let Some(&id) = self.owner_map.get(owner) {
            return id;
        }
        if self.owners.len() >= (NO_OWNER - 1) as usize {
            return NO_OWNER;
        }
        let id = self.owners.len() as u16;
        let s = owner.to_string();
        self.owners.push(s.clone());
        self.owner_map.insert(s, id);
        id
    }

    pub fn set_owner(&mut self, id: NodeId, owner: Option<&str>) {
        if let Some(o) = owner {
            let oid = self.intern_owner(o);
            if let Some(n) = self.nodes.get_mut(id as usize) {
                n.owner_id = oid;
            }
        }
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
        owner_id: u16,
        ads_count: u16,
        ads_size: u64,
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
            own_alloc: 0,
            own_files: 0,
            mtime,
            ctime,
            atime,
            attrs,
            files: 0,
            dirs: 0,
            owner_id,
            ads_count,
            ads_size,
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
            if e.ads_count > 0 {
                flags |= F_HAS_ADS;
            }
            let owner_id = e
                .owner
                .as_deref()
                .map(|o| self.intern_owner(o))
                .unwrap_or(NO_OWNER);
            let is_dir = flags & F_DIR != 0;
            let id = self.push(
                parent,
                &e.name,
                flags,
                e.size,
                e.alloc,
                e.mtime,
                e.ctime,
                e.atime,
                e.attrs,
                owner_id,
                e.ads_count,
                e.ads_size,
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
        self.nodes[parent as usize].own_alloc += own_alloc;
        self.nodes[parent as usize].own_files += files;
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

    /// Character length of the full path (UTF-16 code units, matching Windows `MAX_PATH = 260`),
    /// computed without allocating a `PathBuf`.
    pub fn path_len(&self, id: NodeId) -> u32 {
        if id == 0 {
            return self.root_utf16_len;
        }
        let root_str = self.root_path.to_string_lossy();
        let base_has_sep = root_str.ends_with('\\') || root_str.ends_with('/');
        let mut total = self.root_utf16_len;
        let mut cur = id;
        let mut depth = 0u32;
        while cur != NONE && cur != 0 {
            total += self.name(cur).encode_utf16().count() as u32;
            depth += 1;
            cur = self.nodes[cur as usize].parent;
        }
        if depth > 0 {
            total += if base_has_sep { depth - 1 } else { depth };
        }
        total
    }

    /// Finds the node for a path inside the scan (names compared without regard to case).
    pub fn resolve_path(&self, path: &std::path::Path) -> Option<NodeId> {
        let norm = |p: &std::path::Path| {
            let s = p.to_string_lossy().replace('/', "\\");
            if s == "\\" {
                "\\".to_string()
            } else {
                s.trim_end_matches('\\').to_lowercase()
            }
        };
        let (p, root) = (norm(path), norm(&self.root_path));
        if p == root {
            return Some(0);
        }
        let prefix = if root == "\\" {
            "\\".to_string()
        } else {
            format!("{root}\\")
        };
        let rest = p.strip_prefix(&prefix)?;
        let mut cur = 0;
        for part in rest.split('\\').filter(|s| !s.is_empty()) {
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
            p.own_alloc = p.own_alloc.saturating_sub(alloc);
            p.own_files = p.own_files.saturating_sub(1);
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

    /// Updates a single file's on-disk (`alloc`) space and flags (e.g. after NTFS compression or
    /// hardlink deduplication), adjusting `own_alloc` on its parent and `alloc` on all ancestors.
    pub fn update_file_alloc(
        &mut self,
        id: NodeId,
        new_alloc: u64,
        set_flags: u16,
        clear_flags: u16,
        set_attrs: u32,
        clear_attrs: u32,
    ) {
        let Some(n) = self.nodes.get_mut(id as usize) else {
            return;
        };
        if n.is_dir() {
            n.flags = (n.flags | set_flags) & !clear_flags;
            n.attrs = (n.attrs | set_attrs) & !clear_attrs;
            return;
        }
        let old_alloc = n.alloc;
        let parent = n.parent;
        n.alloc = new_alloc;
        n.flags = (n.flags | set_flags) & !clear_flags;
        n.attrs = (n.attrs | set_attrs) & !clear_attrs;
        if parent != NONE {
            let p = &mut self.nodes[parent as usize];
            if new_alloc >= old_alloc {
                p.own_alloc = p.own_alloc.saturating_add(new_alloc - old_alloc);
            } else {
                p.own_alloc = p.own_alloc.saturating_sub(old_alloc - new_alloc);
            }
        }
        let mut cur = parent;
        while cur != NONE {
            let a = &mut self.nodes[cur as usize];
            if new_alloc >= old_alloc {
                a.alloc = a.alloc.saturating_add(new_alloc - old_alloc);
            } else {
                a.alloc = a.alloc.saturating_sub(old_alloc - new_alloc);
            }
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
    /// rescanned).
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
        n.own_alloc = 0;
        n.own_files = 0;
        n.files = 0;
        n.dirs = 0;
        n.first_child = NONE;
        n.flags &= !F_INCOMPLETE;
    }

    pub fn to_archive(&self) -> IndexArchive {
        IndexArchive {
            version: 1,
            root_path: self.root_path.to_string_lossy().to_string(),
            names: self.names.clone(),
            owners: self.owners.clone(),
            nodes: self.nodes.clone(),
        }
    }

    pub fn from_archive(a: IndexArchive) -> Result<Index, String> {
        if a.version != 1 {
            return Err(format!("Unsupported scan file version ({}).", a.version));
        }
        if a.nodes.is_empty() {
            return Err("Scan file contains no root folder.".into());
        }
        let len = a.nodes.len() as u32;
        let names_len = a.names.len();
        for (i, n) in a.nodes.iter().enumerate() {
            let start = n.name_off as usize;
            let end = start + n.name_len as usize;
            if end > names_len || !a.names.is_char_boundary(start) || !a.names.is_char_boundary(end)
            {
                return Err(format!("Scan file has an invalid name slice at node {i}."));
            }
            if i == 0 && n.parent != NONE {
                return Err("Root node in scan file has a parent.".into());
            }
            if n.parent != NONE && n.parent >= len {
                return Err("Scan file has an out-of-bounds parent link.".into());
            }
            if n.first_child != NONE && n.first_child >= len {
                return Err("Scan file has an out-of-bounds child link.".into());
            }
            if n.next_sibling != NONE && n.next_sibling >= len {
                return Err("Scan file has an out-of-bounds sibling link.".into());
            }
            if n.owner_id != NO_OWNER && (n.owner_id as usize) >= a.owners.len() {
                return Err("Scan file has an out-of-bounds owner reference.".into());
            }
        }
        let root_path = PathBuf::from(&a.root_path);
        let root_utf16_len = path_code_units(&root_path);
        let mut owner_map = HashMap::with_capacity(a.owners.len());
        for (i, o) in a.owners.iter().enumerate() {
            owner_map.insert(o.clone(), i as u16);
        }
        Ok(Index {
            nodes: a.nodes,
            names: a.names,
            owners: a.owners,
            owner_map,
            root_path,
            root_utf16_len,
        })
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
    pub own_size: u64,
    pub own_alloc: u64,
    pub own_files: u32,
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
    pub owner: Option<String>,
    pub ads_count: u16,
    pub ads_size: u64,
    pub path_len: u32,
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
            own_size: n.own_size,
            own_alloc: n.own_alloc,
            own_files: n.own_files,
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
            owner: self.owner_of(id).map(str::to_string),
            ads_count: n.ads_count,
            ads_size: n.ads_size,
            path_len: self.path_len(id),
        }
    }
}
