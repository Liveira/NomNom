//! Windows file facts the organizer relies on. Everything here looks at the entry itself and
//! never follows a link: files are opened with FILE_FLAG_OPEN_REPARSE_POINT.

use std::fs::OpenOptions;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT,
};

pub const ATTR_HIDDEN: u32 = 0x2;
pub const ATTR_SYSTEM: u32 = 0x4;
pub const ATTR_DIRECTORY: u32 = 0x10;
pub const ATTR_REPARSE_POINT: u32 = 0x400;
pub const ATTR_OFFLINE: u32 = 0x1000;
pub const ATTR_UNPINNED: u32 = 0x10_0000;
pub const ATTR_RECALL_ON_OPEN: u32 = 0x4_0000;
pub const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;

/// A file's identity on its volume. It survives a rename on the same volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId {
    pub volume: u32,
    pub index: u64,
}

impl FileId {
    pub fn to_text(self) -> String {
        format!("{:x}:{:x}", self.volume, self.index)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileInfo {
    pub attrs: u32,
    pub size: u64,
    /// Last write time in 100 ns ticks since 1601.
    pub mtime: i64,
    pub id: FileId,
}

impl FileInfo {
    pub fn is_dir(&self) -> bool {
        self.attrs & ATTR_DIRECTORY != 0
    }
    pub fn is_reparse(&self) -> bool {
        self.attrs & ATTR_REPARSE_POINT != 0
    }
    pub fn is_hidden_or_system(&self) -> bool {
        self.attrs & (ATTR_HIDDEN | ATTR_SYSTEM) != 0
    }
    /// Cloud placeholders and other files that may not really be on this disk.
    pub fn is_cloud_or_offline(&self) -> bool {
        self.attrs
            & (ATTR_OFFLINE | ATTR_UNPINNED | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS)
            != 0
    }
}

pub fn info(path: &Path) -> io::Result<FileInfo> {
    let file = OpenOptions::new()
        .access_mode(0)
        .share_mode(7)
        .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
        .open(path)?;
    let mut raw = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the handle is valid for the lifetime of `file`, and `raw` is a plain output struct.
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut raw) }
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(FileInfo {
        attrs: raw.dwFileAttributes,
        size: (u64::from(raw.nFileSizeHigh) << 32) | u64::from(raw.nFileSizeLow),
        mtime: (i64::from(raw.ftLastWriteTime.dwHighDateTime) << 32)
            | i64::from(raw.ftLastWriteTime.dwLowDateTime),
        id: FileId {
            volume: raw.dwVolumeSerialNumber,
            index: (u64::from(raw.nFileIndexHigh) << 32) | u64::from(raw.nFileIndexLow),
        },
    })
}

/// True when something else has the file open, or it can't be opened exclusively for any reason.
pub fn is_locked(path: &Path) -> bool {
    OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .is_err()
}

/// First `n` bytes of a file, without following a link.
pub fn read_head(path: &Path, n: usize) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let mut f = OpenOptions::new()
        .read(true)
        .share_mode(7)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    let mut buf = vec![0u8; n];
    let mut got = 0;
    while got < n {
        match f.read(&mut buf[got..])? {
            0 => break,
            k => got += k,
        }
    }
    buf.truncate(got);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_size_attributes_and_stable_identity() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "hello").unwrap();
        let a = info(&f).unwrap();
        assert_eq!(a.size, 5);
        assert!(!a.is_dir() && !a.is_reparse());
        assert!(info(dir.path()).unwrap().is_dir());

        let g = dir.path().join("renamed.txt");
        std::fs::rename(&f, &g).unwrap();
        assert_eq!(
            info(&g).unwrap().id,
            a.id,
            "a rename keeps the file's identity"
        );
    }

    #[test]
    fn a_junction_is_seen_as_a_link_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let link = dir.path().join("link");
        junction::create(&target, &link).unwrap();
        assert!(info(&link).unwrap().is_reparse());
        assert!(!info(&target).unwrap().is_reparse());
    }

    #[test]
    fn locked_files_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("busy.bin");
        std::fs::write(&f, "x").unwrap();
        assert!(!is_locked(&f));
        let _holder = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&f)
            .unwrap();
        assert!(is_locked(&f));
    }

    #[test]
    fn head_reads_at_most_what_exists() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("h.bin");
        std::fs::write(&f, b"abcdef").unwrap();
        assert_eq!(read_head(&f, 3).unwrap(), b"abc");
        assert_eq!(read_head(&f, 100).unwrap(), b"abcdef");
    }
}
