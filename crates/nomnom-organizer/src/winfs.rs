//! Platform file facts the organizer relies on. Everything here looks at the entry itself and
//! never follows a link (`FILE_FLAG_OPEN_REPARSE_POINT` on Windows, `lstat` / `O_NOFOLLOW` on Unix).

use std::fs::OpenOptions;
use std::io;
use std::path::Path;

pub const ATTR_HIDDEN: u32 = 0x2;
pub const ATTR_SYSTEM: u32 = 0x4;
pub const ATTR_DIRECTORY: u32 = 0x10;
pub const ATTR_REPARSE_POINT: u32 = 0x400;
pub const ATTR_OFFLINE: u32 = 0x1000;
pub const ATTR_UNPINNED: u32 = 0x10_0000;
pub const ATTR_RECALL_ON_OPEN: u32 = 0x4_0000;
pub const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;

/// A file's identity on its volume or mount. It survives a rename on the same filesystem.
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
    /// Last write time in 100 ns ticks since 1601 (Windows FILETIME epoch, used across platforms).
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

#[cfg(windows)]
pub fn info(path: &Path) -> io::Result<FileInfo> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT,
    };
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

#[cfg(unix)]
pub fn info(path: &Path) -> io::Result<FileInfo> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    const FILETIME_UNIX_EPOCH: i64 = 116_444_736_000_000_000;

    let md = std::fs::symlink_metadata(path)?;
    let ft = md.file_type();
    let mut attrs = 0u32;
    if ft.is_dir() {
        attrs |= ATTR_DIRECTORY;
    }
    if ft.is_symlink() {
        attrs |= ATTR_REPARSE_POINT;
    } else if ft.is_fifo() || ft.is_socket() || ft.is_block_device() || ft.is_char_device() {
        attrs |= ATTR_SYSTEM;
    }
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.') && n != "." && n != "..")
    {
        attrs |= ATTR_HIDDEN;
    }
    let dev = md.dev();
    let volume = (dev as u32) ^ ((dev >> 32) as u32);
    let mtime = md
        .mtime()
        .saturating_mul(10_000_000)
        .saturating_add(md.mtime_nsec() / 100)
        .saturating_add(FILETIME_UNIX_EPOCH);
    Ok(FileInfo {
        attrs,
        size: if ft.is_dir() { 0 } else { md.size() },
        mtime,
        id: FileId {
            volume,
            index: md.ino(),
        },
    })
}

/// True when something else has the file open exclusively (Windows) or holds an advisory lock on
/// it (Unix), or when it cannot be opened for reading.
#[cfg(windows)]
pub fn is_locked(path: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .is_err()
}

#[cfg(unix)]
pub fn is_locked(path: &Path) -> bool {
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::io::AsRawFd;
    let Ok(file) = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    else {
        return true;
    };
    let fd = file.as_raw_fd();
    // SAFETY: `fd` is an open file descriptor owned by `file`.
    let rc = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        unsafe {
            let _ = libc::flock(fd, libc::LOCK_UN);
        }
        false
    } else {
        let code = io::Error::last_os_error().raw_os_error();
        matches!(code, Some(c) if c == libc::EWOULDBLOCK || c == libc::EAGAIN)
    }
}

/// First `n` bytes of a file, without following a link.
#[cfg(windows)]
pub fn read_head(path: &Path, n: usize) -> io::Result<Vec<u8>> {
    use std::io::Read;
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
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

#[cfg(unix)]
pub fn read_head(path: &Path, n: usize) -> io::Result<Vec<u8>> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !f.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
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
    fn a_link_is_seen_as_a_link_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let link = dir.path().join("link");
        #[cfg(windows)]
        junction::create(&target, &link).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(info(&link).unwrap().is_reparse());
        assert!(!info(&target).unwrap().is_reparse());
    }

    #[test]
    fn locked_files_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("busy.bin");
        std::fs::write(&f, "x").unwrap();
        assert!(!is_locked(&f));
        #[cfg(windows)]
        let _holder = {
            use std::os::windows::fs::OpenOptionsExt;
            OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&f)
                .unwrap()
        };
        #[cfg(unix)]
        let _holder = {
            use std::os::unix::io::AsRawFd;
            let file = OpenOptions::new().read(true).write(true).open(&f).unwrap();
            unsafe {
                assert_eq!(
                    libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB),
                    0
                );
            }
            file
        };
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
