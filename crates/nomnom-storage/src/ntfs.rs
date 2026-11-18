//! Filesystem-level helpers: file owner lookup (cached SID on Windows, cached `/etc/passwd` +
//! `getpwuid_r` on Linux), alternate data streams / extended attributes (`xattr`), in-place
//! compression (`FSCTL_SET_COMPRESSION` on NTFS, `FS_IOC_SETFLAGS` `FS_COMPR_FL` on Linux where
//! supported), atomic hardlink replacement, privilege checks, and NTFS MFT metadata.

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::Mutex;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::scan::long_os_path;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, BOOL, HANDLE};
    use windows::Win32::Security::{
        AdjustTokenPrivileges, GetFileSecurityW, GetLengthSid, GetSecurityDescriptorOwner,
        GetTokenInformation, IsValidSid, LookupAccountSidW, LookupPrivilegeValueW,
        LUID_AND_ATTRIBUTES, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
        SE_BACKUP_NAME, SE_PRIVILEGE_ENABLED, SID_NAME_USE, TOKEN_ADJUST_PRIVILEGES,
        TOKEN_ELEVATION, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, CreateHardLinkW, FindClose, FindFirstStreamW, FindNextStreamW,
        FindStreamInfoStandard, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        WIN32_FIND_STREAM_DATA,
    };
    use windows::Win32::System::Ioctl::{
        FSCTL_GET_NTFS_VOLUME_DATA, FSCTL_SET_COMPRESSION, NTFS_VOLUME_DATA_BUFFER,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::Win32::System::IO::DeviceIoControl;

    const COMPRESSION_FORMAT_NONE_U16: u16 = 0;
    const COMPRESSION_FORMAT_DEFAULT_U16: u16 = 1;

    fn wide_long(path: &Path) -> Vec<u16> {
        long_os_path(path)
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    #[derive(Default)]
    pub struct OwnerCache {
        by_sid: Mutex<HashMap<Vec<u8>, Option<String>>>,
    }

    impl OwnerCache {
        pub fn owner_of(&self, path: &Path) -> Option<String> {
            let w = wide_long(path);
            let mut stack_buf = [0u8; 256];
            let mut needed = 0u32;
            // SAFETY: `w` is NUL-terminated; `stack_buf` is valid writable memory of 256 bytes.
            let ok = unsafe {
                GetFileSecurityW(
                    PCWSTR(w.as_ptr()),
                    OWNER_SECURITY_INFORMATION.0,
                    PSECURITY_DESCRIPTOR(stack_buf.as_mut_ptr() as *mut _),
                    stack_buf.len() as u32,
                    &mut needed,
                )
            };
            let heap_buf: Vec<u8>;
            let sd_bytes: &[u8] = if ok.as_bool() {
                &stack_buf[..]
            } else if needed > stack_buf.len() as u32 && needed <= 65_536 {
                let mut v = vec![0u8; needed as usize];
                let mut needed2 = 0u32;
                // SAFETY: `v` has `needed` writable bytes.
                let ok2 = unsafe {
                    GetFileSecurityW(
                        PCWSTR(w.as_ptr()),
                        OWNER_SECURITY_INFORMATION.0,
                        PSECURITY_DESCRIPTOR(v.as_mut_ptr() as *mut _),
                        v.len() as u32,
                        &mut needed2,
                    )
                };
                if !ok2.as_bool() {
                    return None;
                }
                heap_buf = v;
                &heap_buf[..]
            } else {
                return None;
            };

            let mut sid = PSID::default();
            let mut defaulted = BOOL::default();
            // SAFETY: `sd_bytes` holds a self-relative security descriptor returned by GetFileSecurityW.
            if unsafe {
                GetSecurityDescriptorOwner(
                    PSECURITY_DESCRIPTOR(sd_bytes.as_ptr() as *mut _),
                    &mut sid,
                    &mut defaulted,
                )
            }
            .is_err()
                || sid.is_invalid()
            {
                return None;
            }
            // SAFETY: checked `sid` came from a valid security descriptor.
            if !unsafe { IsValidSid(sid) }.as_bool() {
                return None;
            }
            let len = unsafe { GetLengthSid(sid) } as usize;
            if len == 0 || len > 256 {
                return None;
            }
            // SAFETY: `sid.0` points to `len` bytes inside `sd_bytes`.
            let raw = unsafe { std::slice::from_raw_parts(sid.0 as *const u8, len) };
            if let Some(cached) = self.by_sid.lock().unwrap().get(raw).cloned() {
                return cached;
            }
            let resolved = lookup_sid(sid);
            self.by_sid
                .lock()
                .unwrap()
                .insert(raw.to_vec(), resolved.clone());
            resolved
        }
    }

    fn lookup_sid(sid: PSID) -> Option<String> {
        let mut name_buf = [0u16; 256];
        let mut dom_buf = [0u16; 256];
        let mut name_len = name_buf.len() as u32;
        let mut dom_len = dom_buf.len() as u32;
        let mut kind = SID_NAME_USE::default();
        // SAFETY: buffers and lengths match what LookupAccountSidW expects.
        let ok = unsafe {
            LookupAccountSidW(
                PCWSTR::null(),
                sid,
                windows::core::PWSTR(name_buf.as_mut_ptr()),
                &mut name_len,
                windows::core::PWSTR(dom_buf.as_mut_ptr()),
                &mut dom_len,
                &mut kind,
            )
        };
        if ok.is_err() {
            return None;
        }
        let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
        let dom = String::from_utf16_lossy(&dom_buf[..dom_len as usize]);
        if name.is_empty() {
            None
        } else if dom.is_empty()
            || dom.eq_ignore_ascii_case("BUILTIN")
            || dom.eq_ignore_ascii_case("NT AUTHORITY")
        {
            Some(name)
        } else {
            Some(format!(r"{dom}\{name}"))
        }
    }

    pub fn alternate_streams(path: &Path) -> (u16, u64) {
        let w = wide_long(path);
        let mut data = WIN32_FIND_STREAM_DATA::default();
        // SAFETY: `w` is NUL-terminated; `data` is a plain POD output buffer.
        let Ok(handle) = (unsafe {
            FindFirstStreamW(
                PCWSTR(w.as_ptr()),
                FindStreamInfoStandard,
                &mut data as *mut _ as *mut _,
                0,
            )
        }) else {
            return (0, 0);
        };
        let mut count = 0u16;
        let mut bytes = 0u64;
        loop {
            let end = data
                .cStreamName
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(data.cStreamName.len());
            let sname = String::from_utf16_lossy(&data.cStreamName[..end]);
            let is_default = sname.eq_ignore_ascii_case("::$DATA")
                || sname.to_ascii_uppercase().ends_with(":$INDEX_ALLOCATION");
            if !is_default && !sname.is_empty() {
                count = count.saturating_add(1);
                if data.StreamSize > 0 {
                    bytes = bytes.saturating_add(data.StreamSize as u64);
                }
            }
            // SAFETY: `handle` is valid until FindClose.
            if unsafe { FindNextStreamW(handle, &mut data as *mut _ as *mut _) }.is_err() {
                break;
            }
        }
        // SAFETY: `handle` came from FindFirstStreamW.
        unsafe {
            let _ = FindClose(handle);
        }
        (count, bytes)
    }

    pub fn set_ntfs_compression(path: &Path, compress: bool) -> io::Result<()> {
        let file = std::fs::OpenOptions::new()
            .access_mode(0x1 | 0x2)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
            .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
            .open(long_os_path(path))?;
        let format: u16 = if compress {
            COMPRESSION_FORMAT_DEFAULT_U16
        } else {
            COMPRESSION_FORMAT_NONE_U16
        };
        let mut returned = 0u32;
        // SAFETY: handle is valid for the duration of `file`; `format` is a valid u16 input buffer.
        unsafe {
            DeviceIoControl(
                HANDLE(file.as_raw_handle()),
                FSCTL_SET_COMPRESSION,
                Some(&format as *const u16 as *const _),
                std::mem::size_of::<u16>() as u32,
                None,
                0,
                Some(&mut returned),
                None,
            )
        }
        .map_err(|e| io::Error::from_raw_os_error(e.code().0 & 0xFFFF))
    }

    pub fn replace_with_hardlink(keep: &Path, target: &Path) -> io::Result<()> {
        let parent = target
            .parent()
            .ok_or_else(|| io::Error::other("Target file has no parent folder."))?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let backup = parent.join(format!(".nomnom-hl-{}-{stamp}.bak", std::process::id()));
        let long_target = long_os_path(target);
        let long_backup = long_os_path(&backup);
        nomnom_core::fs::move_no_replace(&long_target, &long_backup)?;

        let w_target = wide_long(target);
        let w_keep = wide_long(keep);
        // SAFETY: both paths are NUL-terminated UTF-16 strings; security attributes is None.
        let linked =
            unsafe { CreateHardLinkW(PCWSTR(w_target.as_ptr()), PCWSTR(w_keep.as_ptr()), None) };
        if let Err(e) = linked {
            let code = e.code().0 & 0xFFFF;
            let _ = nomnom_core::fs::move_no_replace(&long_backup, &long_target);
            return Err(io::Error::from_raw_os_error(code));
        }
        let _ = std::fs::remove_file(&long_backup);
        Ok(())
    }

    pub fn is_elevated() -> bool {
        let mut token = HANDLE::default();
        // SAFETY: current process pseudo-handle is always valid.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION::default();
        let mut ret = 0u32;
        // SAFETY: `elev` matches `TokenElevation` output size.
        let ok = unsafe {
            GetTokenInformation(
                token,
                windows::Win32::Security::TokenElevation,
                Some(&mut elev as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut ret,
            )
        };
        unsafe {
            let _ = CloseHandle(token);
        }
        ok.is_ok() && elev.TokenIsElevated != 0
    }

    pub fn enable_backup_privilege() -> bool {
        let mut token = HANDLE::default();
        if unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token,
            )
        }
        .is_err()
        {
            return false;
        }
        let mut luid = Default::default();
        let looked = unsafe { LookupPrivilegeValueW(PCWSTR::null(), SE_BACKUP_NAME, &mut luid) };
        if looked.is_err() {
            unsafe {
                let _ = CloseHandle(token);
            }
            return false;
        }
        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        let adj = unsafe { AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None) };
        let last_err = unsafe { GetLastError() };
        unsafe {
            let _ = CloseHandle(token);
        }
        adj.is_ok() && last_err.0 == 0
    }

    pub fn ntfs_mft_bytes(root: &Path) -> Option<u64> {
        let s = root.to_string_lossy();
        let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
        let letter = s
            .chars()
            .next()
            .filter(|c| c.is_ascii_alphabetic() && s.chars().nth(1) == Some(':'))?;
        let vol: Vec<u16> = OsStr::new(&format!(r"\\.\{letter}:"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                PCWSTR(vol.as_ptr()),
                0x8000_0000,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                Default::default(),
                None,
            )
        }
        .ok()?;
        let mut data = NTFS_VOLUME_DATA_BUFFER::default();
        let mut returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_GET_NTFS_VOLUME_DATA,
                None,
                0,
                Some(&mut data as *mut _ as *mut _),
                std::mem::size_of::<NTFS_VOLUME_DATA_BUFFER>() as u32,
                Some(&mut returned),
                None,
            )
        };
        unsafe {
            let _ = CloseHandle(handle);
        }
        ok.ok()?;
        let mft = data.MftValidDataLength.max(0) as u64;
        let reserved =
            (data.TotalReserved.max(0) as u64).saturating_mul(u64::from(data.BytesPerCluster));
        Some(mft.saturating_add(reserved))
    }
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::os::unix::io::AsRawFd;

    pub struct OwnerCache {
        passwd: HashMap<u32, String>,
        extra: Mutex<HashMap<u32, String>>,
    }

    impl Default for OwnerCache {
        fn default() -> Self {
            let mut passwd = HashMap::new();
            if let Ok(text) = std::fs::read_to_string("/etc/passwd") {
                for line in text.lines() {
                    let mut cols = line.split(':');
                    let (Some(user), Some(_pw), Some(uid_str)) =
                        (cols.next(), cols.next(), cols.next())
                    else {
                        continue;
                    };
                    if !user.is_empty() {
                        if let Ok(uid) = uid_str.parse::<u32>() {
                            passwd.insert(uid, user.to_string());
                        }
                    }
                }
            }
            OwnerCache {
                passwd,
                extra: Mutex::new(HashMap::new()),
            }
        }
    }

    impl OwnerCache {
        pub fn owner_of(&self, path: &Path) -> Option<String> {
            let md = std::fs::symlink_metadata(path).ok()?;
            self.owner_of_uid(md.uid())
        }

        pub fn owner_of_uid(&self, uid: u32) -> Option<String> {
            if let Some(name) = self.passwd.get(&uid) {
                return Some(name.clone());
            }
            if let Some(cached) = self.extra.lock().unwrap().get(&uid).cloned() {
                return Some(cached);
            }
            let resolved = lookup_uid_nss(uid).unwrap_or_else(|| format!("uid:{uid}"));
            self.extra.lock().unwrap().insert(uid, resolved.clone());
            Some(resolved)
        }
    }

    fn lookup_uid_nss(uid: u32) -> Option<String> {
        let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
        let mut buf = vec![0 as libc::c_char; 4096];
        let mut result: *mut libc::passwd = std::ptr::null_mut();
        // SAFETY: thread-safe reentrant passwd lookup using caller-owned `pwd` and `buf`.
        let rc =
            unsafe { libc::getpwuid_r(uid, &mut pwd, buf.as_mut_ptr(), buf.len(), &mut result) };
        if rc != 0 || result.is_null() || pwd.pw_name.is_null() {
            return None;
        }
        // SAFETY: `pwd.pw_name` points into `buf` and is NUL-terminated.
        let name = unsafe { CStr::from_ptr(pwd.pw_name) }.to_string_lossy();
        (!name.is_empty()).then(|| name.into_owned())
    }

    /// Counts extended attributes (`xattr`) on `path` and returns `(count, total_value_bytes)`.
    pub fn alternate_streams(path: &Path) -> (u16, u64) {
        #[cfg(target_os = "linux")]
        {
            let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
                return (0, 0);
            };
            // SAFETY: c_path is NUL-terminated; passing null queries total xattr name list size.
            let needed = unsafe { libc::llistxattr(c_path.as_ptr(), std::ptr::null_mut(), 0) };
            if needed <= 0 || needed > 65_536 {
                return (0, 0);
            }
            let mut names = vec![0 as libc::c_char; needed as usize];
            let got = unsafe { libc::llistxattr(c_path.as_ptr(), names.as_mut_ptr(), names.len()) };
            if got <= 0 {
                return (0, 0);
            }
            let raw_bytes: &[u8] =
                unsafe { std::slice::from_raw_parts(names.as_ptr() as *const u8, got as usize) };
            let mut count = 0u16;
            let mut total_bytes = 0u64;
            for slice in raw_bytes.split(|&b| b == 0).filter(|s| !s.is_empty()) {
                let Ok(attr_name) = CString::new(slice) else {
                    continue;
                };
                count = count.saturating_add(1);
                let val_len = unsafe {
                    libc::lgetxattr(c_path.as_ptr(), attr_name.as_ptr(), std::ptr::null_mut(), 0)
                };
                if val_len > 0 {
                    total_bytes = total_bytes.saturating_add(val_len as u64);
                }
            }
            (count, total_bytes)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = path;
            (0, 0)
        }
    }

    /// Toggles the Linux `FS_COMPR_FL` inode flag (`FS_IOC_GETFLAGS` / `FS_IOC_SETFLAGS`), used by
    /// filesystems with per-inode transparent compression such as Btrfs.
    pub fn set_ntfs_compression(path: &Path, compress: bool) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        {
            const FS_COMPR_FL: libc::c_long = 0x0000_0004;
            const FS_NOCOMP_FL: libc::c_long = 0x0000_0400;
            const IOC_GETFLAGS: libc::c_ulong = (2 << 30)
                | ((std::mem::size_of::<libc::c_long>() as libc::c_ulong) << 16)
                | ((b'f' as libc::c_ulong) << 8)
                | 1;
            const IOC_SETFLAGS: libc::c_ulong = (1 << 30)
                | ((std::mem::size_of::<libc::c_long>() as libc::c_ulong) << 16)
                | ((b'f' as libc::c_ulong) << 8)
                | 2;

            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(path)?;
            let fd = file.as_raw_fd();
            let mut flags: libc::c_long = 0;
            // SAFETY: `fd` is open and `flags` is a valid `c_long` buffer.
            if unsafe { libc::ioctl(fd, IOC_GETFLAGS as _, &mut flags) } != 0 {
                let err = io::Error::last_os_error();
                if matches!(
                    err.raw_os_error(),
                    Some(libc::EOPNOTSUPP | libc::ENOTTY | libc::EINVAL)
                ) {
                    return Err(io::Error::other(
                        "This filesystem doesn't support per-file compression.",
                    ));
                }
                return Err(err);
            }
            if compress {
                flags = (flags | FS_COMPR_FL) & !FS_NOCOMP_FL;
            } else {
                flags &= !FS_COMPR_FL;
            }
            if unsafe { libc::ioctl(fd, IOC_SETFLAGS as _, &flags) } != 0 {
                let err = io::Error::last_os_error();
                if matches!(
                    err.raw_os_error(),
                    Some(libc::EOPNOTSUPP | libc::ENOTTY | libc::EINVAL)
                ) {
                    return Err(io::Error::other(
                        "This filesystem doesn't support per-file compression.",
                    ));
                }
                return Err(err);
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (path, compress);
            Err(io::Error::other(
                "This filesystem doesn't support per-file compression.",
            ))
        }
    }

    pub fn replace_with_hardlink(keep: &Path, target: &Path) -> io::Result<()> {
        let parent = target
            .parent()
            .ok_or_else(|| io::Error::other("Target file has no parent folder."))?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let backup = parent.join(format!(".nomnom-hl-{}-{stamp}.bak", std::process::id()));
        nomnom_core::fs::move_no_replace(target, &backup)?;
        if let Err(e) = std::fs::hard_link(keep, target) {
            let _ = nomnom_core::fs::move_no_replace(&backup, target);
            return Err(e);
        }
        let _ = std::fs::remove_file(&backup);
        Ok(())
    }

    pub fn is_elevated() -> bool {
        // SAFETY: geteuid takes no arguments and cannot fail.
        unsafe { libc::geteuid() == 0 }
    }

    pub fn enable_backup_privilege() -> bool {
        is_elevated()
    }

    pub fn ntfs_mft_bytes(_root: &Path) -> Option<u64> {
        None
    }
}

pub use imp::{
    alternate_streams, enable_backup_privilege, is_elevated, ntfs_mft_bytes, replace_with_hardlink,
    set_ntfs_compression, OwnerCache,
};
