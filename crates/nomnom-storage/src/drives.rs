//! The drives Windows knows about, with their capacity and what kind they are.

use serde::Serialize;
use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageDeviceSeekPenaltyProperty, DEVICE_SEEK_PENALTY_DESCRIPTOR,
    IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY,
};
use windows::Win32::System::IO::DeviceIoControl;

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
    /// `C:\`
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

/// Every lettered drive that is present. Drives that can't be read (an empty card reader) are
/// listed with zero capacity rather than hidden.
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

fn cluster_size(rootw: &[u16]) -> u32 {
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceW;
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

/// The drive that holds `path`, if it is on a lettered drive.
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

pub fn root_path(d: &Drive) -> PathBuf {
    PathBuf::from(&d.root)
}
