//! Filesystem primitives that never overwrite silently.

use std::io;
use std::path::Path;

#[cfg(windows)]
fn wide(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Moves `src` to `dst`, failing with `ErrorKind::AlreadyExists` if `dst` exists.
/// (`std::fs::rename` replaces an existing destination on Windows, so it must not be used for
/// user files.) Works across volumes by falling back to copy + delete inside the OS call.
#[cfg(windows)]
pub fn move_no_replace(src: &Path, dst: &Path) -> io::Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_COPY_ALLOWED, MOVEFILE_WRITE_THROUGH,
    };
    const ERROR_FILE_EXISTS: i32 = 80;
    const ERROR_ALREADY_EXISTS: i32 = 183;

    let (s, d) = (wide(src), wide(dst));
    // SAFETY: both buffers are NUL-terminated and outlive the call. No REPLACE_EXISTING flag.
    let result = unsafe {
        MoveFileExW(
            PCWSTR(s.as_ptr()),
            PCWSTR(d.as_ptr()),
            MOVEFILE_COPY_ALLOWED | MOVEFILE_WRITE_THROUGH,
        )
    };
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            let code = e.code().0 & 0xFFFF;
            if code == ERROR_FILE_EXISTS || code == ERROR_ALREADY_EXISTS {
                Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} already exists", dst.display()),
                ))
            } else {
                Err(io::Error::from_raw_os_error(code))
            }
        }
    }
}

/// Marks a directory hidden (used for staging folders). Best effort.
#[cfg(windows)]
pub fn hide(path: &Path) {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN};
    let p = wide(path);
    // SAFETY: `p` is NUL-terminated and valid for the call.
    unsafe {
        let _ = SetFileAttributesW(PCWSTR(p.as_ptr()), FILE_ATTRIBUTE_HIDDEN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_to_replace_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.txt"), dir.path().join("B.txt"));
        std::fs::write(&a, "new").unwrap();
        std::fs::write(&b, "precious").unwrap();
        let err = move_no_replace(&a, &b).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "precious");
        assert!(a.exists(), "source must stay when the move is refused");
    }

    #[test]
    fn refuses_case_insensitive_collision() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.txt"), dir.path().join("x.txt"));
        std::fs::write(&a, "1").unwrap();
        std::fs::write(&b, "2").unwrap();
        let upper = dir.path().join("X.TXT");
        assert_eq!(
            move_no_replace(&a, &upper).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
    }

    #[test]
    fn moves_when_free() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.txt"), dir.path().join("b.txt"));
        std::fs::write(&a, "hi").unwrap();
        move_no_replace(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "hi");
    }
}
