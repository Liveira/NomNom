//! Mounted drives and filesystems, with their capacity, filesystem type and media kind (SSD/HDD).

use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriveKind {
    Fixed,
    Removable,
    Network,
    Optical,
    Ram,
    Unknown,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Media {
    Ssd,
    Hdd,
    Unknown,
}

#[derive(Serialize, Clone, Debug)]
pub struct Drive {
    /// `C:\` on Windows, mount point such as `/` or `/home` on Linux.
    pub root: String,
    pub letter: String,
    pub label: String,
    pub filesystem: String,
    pub kind: DriveKind,
    pub media: Media,
    pub total: u64,
    pub free: u64,
    pub used: u64,
    pub cluster: u32,
    pub serial: u32,
}

pub fn root_path(d: &Drive) -> PathBuf {
    PathBuf::from(&d.root)
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetDiskFreeSpaceExW, GetDiskFreeSpaceW, GetDriveTypeW, GetLogicalDrives,
        GetVolumeInformationW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    };
    use windows::Win32::System::Ioctl::{
        PropertyStandardQuery, StorageDeviceSeekPenaltyProperty, DEVICE_SEEK_PENALTY_DESCRIPTOR,
        IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY,
    };
    use windows::Win32::System::IO::DeviceIoControl;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn media_of(letter: char) -> Media {
        let path = wide(&format!(r"\\.\{letter}:"));
        // SAFETY: path is NUL-terminated and outlives the call; access 0 needs no rights.
        let handle = unsafe {
            CreateFileW(
                PCWSTR(path.as_ptr()),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            )
        };
        let Ok(handle) = handle else {
            return Media::Unknown;
        };
        let query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceSeekPenaltyProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut out = DEVICE_SEEK_PENALTY_DESCRIPTOR::default();
        let mut returned = 0u32;
        // SAFETY: buffers are plain structs that live across the call.
        let ok = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                Some(&query as *const _ as *const _),
                std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                Some(&mut out as *mut _ as *mut _),
                std::mem::size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
                Some(&mut returned),
                None,
            )
        };
        // SAFETY: handle came from CreateFileW above.
        unsafe {
            let _ = CloseHandle(handle);
        }
        match ok {
            Ok(()) if out.IncursSeekPenalty.as_bool() => Media::Hdd,
            Ok(()) => Media::Ssd,
            Err(_) => Media::Unknown,
        }
    }

    fn cluster_size(rootw: &[u16]) -> u32 {
        let (mut spc, mut bps, mut _free, mut _total) = (0u32, 0u32, 0u32, 0u32);
        // SAFETY: rootw is NUL-terminated; out pointers are valid locals.
        let ok = unsafe {
            GetDiskFreeSpaceW(
                PCWSTR(rootw.as_ptr()),
                Some(&mut spc),
                Some(&mut bps),
                Some(&mut _free),
                Some(&mut _total),
            )
        };
        if ok.is_ok() && spc > 0 && bps > 0 {
            spc * bps
        } else {
            4096
        }
    }

    pub fn list() -> Vec<Drive> {
        // SAFETY: plain query with no arguments.
        let mask = unsafe { GetLogicalDrives() };
        let mut out = Vec::new();
        for i in 0..26u32 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let letter = (b'A' + i as u8) as char;
            let root = format!("{letter}:\\");
            let rootw = wide(&root);
            // SAFETY: rootw is NUL-terminated.
            let kind = match unsafe { GetDriveTypeW(PCWSTR(rootw.as_ptr())) } {
                2 => DriveKind::Removable,
                3 => DriveKind::Fixed,
                4 => DriveKind::Network,
                5 => DriveKind::Optical,
                6 => DriveKind::Ram,
                _ => DriveKind::Unknown,
            };
            let mut label = [0u16; 261];
            let mut fs = [0u16; 261];
            let mut serial = 0u32;
            // SAFETY: buffers are sized as the API requires.
            let info = unsafe {
                GetVolumeInformationW(
                    PCWSTR(rootw.as_ptr()),
                    Some(&mut label),
                    Some(&mut serial),
                    None,
                    None,
                    Some(&mut fs),
                )
            };
            let (mut free_caller, mut total, mut free) = (0u64, 0u64, 0u64);
            // SAFETY: out pointers are valid locals.
            let space = unsafe {
                GetDiskFreeSpaceExW(
                    PCWSTR(rootw.as_ptr()),
                    Some(&mut free_caller),
                    Some(&mut total),
                    Some(&mut free),
                )
            };
            let to_s = |b: &[u16]| {
                let n = b.iter().position(|c| *c == 0).unwrap_or(b.len());
                String::from_utf16_lossy(&b[..n])
            };
            let readable = info.is_ok() && space.is_ok();
            out.push(Drive {
                root,
                letter: letter.to_string(),
                label: if readable {
                    to_s(&label)
                } else {
                    String::new()
                },
                filesystem: if readable { to_s(&fs) } else { String::new() },
                media: if kind == DriveKind::Fixed {
                    media_of(letter)
                } else {
                    Media::Unknown
                },
                kind,
                total: if readable { total } else { 0 },
                free: if readable { free } else { 0 },
                used: if readable {
                    total.saturating_sub(free)
                } else {
                    0
                },
                cluster: cluster_size(&rootw),
                serial,
            });
        }
        out
    }

    pub fn drive_of(path: &std::path::Path) -> Option<Drive> {
        let s = path.to_string_lossy();
        let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
        let letter = s
            .chars()
            .next()
            .filter(|c| c.is_ascii_alphabetic() && s.chars().nth(1) == Some(':'))?;
        list()
            .into_iter()
            .find(|d| d.letter.eq_ignore_ascii_case(&letter.to_string()))
    }
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    const IGNORED_FS: &[&str] = &[
        "proc",
        "sysfs",
        "devtmpfs",
        "devpts",
        "cgroup",
        "cgroup2",
        "pstore",
        "bpf",
        "tracefs",
        "debugfs",
        "securityfs",
        "hugetlbfs",
        "mqueue",
        "fusectl",
        "configfs",
        "efivarfs",
        "autofs",
        "nsfs",
        "ramfs",
        "rpc_pipefs",
        "binfmt_misc",
        "squashfs",
        "fuse.portal",
        "fuse.gvfsd-fuse",
    ];

    fn unescape_mount_field(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\\' && i + 3 < bytes.len() {
                if let Ok(oct) = std::str::from_utf8(&bytes[i + 1..i + 4]) {
                    if let Ok(val) = u8::from_str_radix(oct, 8) {
                        out.push(val);
                        i += 4;
                        continue;
                    }
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn base_block_device(dev_path: &str) -> Option<String> {
        let real = std::fs::canonicalize(dev_path)
            .ok()
            .unwrap_or_else(|| PathBuf::from(dev_path));
        let name = real.file_name()?.to_str()?;
        if (name.starts_with("nvme") || name.starts_with("mmcblk") || name.starts_with("loop"))
            && name.contains('p')
        {
            if let Some((base, part)) = name.rsplit_once('p') {
                if !base.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
                    return Some(base.to_string());
                }
            }
        }
        if name.starts_with("dm-") || name.starts_with("md") {
            return Some(name.to_string());
        }
        let trimmed = name.trim_end_matches(|c: char| c.is_ascii_digit());
        if !trimmed.is_empty() {
            Some(trimmed.to_string())
        } else {
            Some(name.to_string())
        }
    }

    fn media_and_removable(dev: &str, mount: &str) -> (Media, bool) {
        let Some(block) = base_block_device(dev) else {
            return (Media::Unknown, false);
        };
        let sys = PathBuf::from("/sys/block").join(&block);
        let rotational = std::fs::read_to_string(sys.join("queue/rotational"))
            .ok()
            .map(|s| s.trim().to_string());
        let media = match rotational.as_deref() {
            Some("0") => Media::Ssd,
            Some("1") => Media::Hdd,
            _ => Media::Unknown,
        };
        let sys_rem = std::fs::read_to_string(sys.join("removable"))
            .ok()
            .is_some_and(|s| s.trim() == "1");
        let path_rem = mount.starts_with("/run/media/") || mount.starts_with("/media/");
        (media, sys_rem || path_rem)
    }

    fn read_disk_labels() -> HashMap<PathBuf, String> {
        let mut map = HashMap::new();
        let Ok(rd) = std::fs::read_dir("/dev/disk/by-label") else {
            return map;
        };
        for entry in rd.flatten() {
            let label = unescape_mount_field(&entry.file_name().to_string_lossy());
            if let Ok(target) = std::fs::canonicalize(entry.path()) {
                map.insert(target, label);
            }
        }
        map
    }

    fn is_user_mount(dev: &str, mount: &str, fstype: &str) -> bool {
        if IGNORED_FS.contains(&fstype) {
            return false;
        }
        if mount.starts_with("/proc/")
            || mount.starts_with("/sys/")
            || mount.starts_with("/dev/")
            || mount.starts_with("/run/user/")
            || mount.starts_with("/run/credentials/")
            || mount.starts_with("/var/lib/docker/")
            || mount.starts_with("/snap/")
            || mount == "/proc"
            || mount == "/sys"
            || mount == "/dev"
            || mount == "/run"
            || mount == "/boot/efi"
        {
            return false;
        }
        if fstype == "tmpfs" {
            return mount == "/tmp";
        }
        dev.starts_with('/')
            || fstype.contains("nfs")
            || fstype.contains("cifs")
            || fstype.contains("smb")
            || fstype.contains("sshfs")
            || mount == "/"
    }

    pub fn list() -> Vec<Drive> {
        let labels = read_disk_labels();
        let mounts_text = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
        let mut raw_mounts: Vec<(String, String, String)> = Vec::new();
        for line in mounts_text.lines() {
            let mut parts = line.split_whitespace();
            let (Some(dev), Some(mnt), Some(fstype)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let dev = unescape_mount_field(dev);
            let mnt = unescape_mount_field(mnt);
            if is_user_mount(&dev, &mnt, fstype) {
                raw_mounts.push((dev, mnt, fstype.to_string()));
            }
        }
        if raw_mounts.is_empty() {
            raw_mounts.push(("rootfs".into(), "/".into(), "linux".into()));
        }
        // Shortest mount path first so `/` wins over bind mounts of the same partition, while
        // still keeping explicit `/home` or `/mnt/*` submounts.
        raw_mounts.sort_by(|a, b| a.1.len().cmp(&b.1.len()).then_with(|| a.1.cmp(&b.1)));

        let mut seen_mounts = HashSet::new();
        let mut seen_devs = HashSet::new();
        let mut out = Vec::new();

        for (dev, mount, fstype) in raw_mounts {
            if !seen_mounts.insert(mount.clone()) {
                continue;
            }
            let keep_subvol = mount == "/"
                || mount == "/home"
                || mount.starts_with("/mnt/")
                || mount.starts_with("/run/media/")
                || mount.starts_with("/media/");
            if dev.starts_with("/dev/") && !seen_devs.insert(dev.clone()) && !keep_subvol {
                continue;
            }

            let Ok(c_mount) = CString::new(Path::new(&mount).as_os_str().as_bytes()) else {
                continue;
            };
            let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
            // SAFETY: c_mount is NUL-terminated; st is a valid statvfs buffer.
            let ok = unsafe { libc::statvfs(c_mount.as_ptr(), &mut st) == 0 };
            let unit = if ok && st.f_frsize > 0 {
                st.f_frsize as u64
            } else if ok && st.f_bsize > 0 {
                st.f_bsize as u64
            } else {
                4096
            };
            let total = if ok {
                (st.f_blocks as u64).saturating_mul(unit)
            } else {
                0
            };
            let free = if ok {
                (st.f_bavail as u64).saturating_mul(unit).min(total)
            } else {
                0
            };
            let used = total.saturating_sub(free);
            let cluster = if ok && st.f_bsize > 0 {
                (st.f_bsize as u32).max(512)
            } else {
                4096
            };
            let serial = if ok { st.f_fsid as u32 } else { 0 };

            let kind = if matches!(
                fstype.as_str(),
                "nfs" | "nfs4" | "cifs" | "smb3" | "smbfs" | "fuse.sshfs" | "9p"
            ) {
                DriveKind::Network
            } else if fstype == "tmpfs" {
                DriveKind::Ram
            } else if matches!(fstype.as_str(), "iso9660" | "udf") {
                DriveKind::Optical
            } else {
                let (_, rem) = media_and_removable(&dev, &mount);
                if rem {
                    DriveKind::Removable
                } else {
                    DriveKind::Fixed
                }
            };
            let (media, _) = if kind == DriveKind::Fixed || kind == DriveKind::Removable {
                media_and_removable(&dev, &mount)
            } else {
                (Media::Unknown, false)
            };

            let canon_dev = std::fs::canonicalize(&dev).ok();
            let label = canon_dev
                .as_ref()
                .and_then(|p| labels.get(p).cloned())
                .or_else(|| {
                    dev.strip_prefix("/dev/")
                        .map(str::to_string)
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or_default();

            out.push(Drive {
                letter: mount.clone(),
                root: mount,
                label,
                filesystem: fstype,
                kind,
                media,
                total,
                free,
                used,
                cluster,
                serial,
            });
        }
        out
    }

    pub fn drive_of(path: &std::path::Path) -> Option<Drive> {
        if !path.is_absolute() {
            return None;
        }
        list()
            .into_iter()
            .filter(|d| nomnom_organizer::pathutil::is_inside(path, Path::new(&d.root)))
            .max_by_key(|d| d.root.len())
    }
}

/// Every mounted drive or user filesystem that is present.
pub fn list() -> Vec<Drive> {
    imp::list()
}

/// The drive or mount that holds `path`.
pub fn drive_of(path: &std::path::Path) -> Option<Drive> {
    imp::drive_of(path)
}
