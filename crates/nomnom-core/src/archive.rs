//! Safe handling of the names and sizes inside a ZIP file. A ZIP can hold a name like `../../x` that
//! would be written outside the folder it is opened into, or a tiny file that unpacks to a huge one.
//! This file decides which names are allowed and when an archive is too big to open; reading and
//! writing the files is the app's job.

use std::path::PathBuf;

pub const MAX_ENTRIES: usize = 200_000;
/// The most that is unpacked from one archive, in bytes.
pub const MAX_UNPACKED: u64 = 16 * 1024 * 1024 * 1024;
/// An archive that grows more than this many times is refused (a "zip bomb").
pub const MAX_RATIO: u64 = 1000;

/// The path an entry is unpacked to, inside the destination folder, or `None` when the name could
/// lead anywhere else (a `..`, a drive, a root, a device name, or characters Windows refuses).
pub fn safe_entry(name: &str) -> Option<PathBuf> {
    let n = name.replace('\\', "/");
    let n = n.trim_end_matches('/');
    if n.is_empty() || n.starts_with('/') || n.contains(':') || n.contains('\0') || n.len() > 1000 {
        return None;
    }
    let mut out = PathBuf::new();
    for part in n.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return None;
        }
        if part.ends_with('.')
            || part.ends_with(' ')
            || part.contains(['<', '>', '"', '|', '?', '*'])
        {
            return None;
        }
        let stem = part.split('.').next().unwrap_or(part).to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit());
        if reserved {
            return None;
        }
        out.push(part);
    }
    Some(out)
}

/// Is an archive of this shape fine to open? `Err` says why not, in words for the person.
pub fn check_limits(entries: usize, unpacked: u64, packed: u64) -> Result<(), String> {
    if entries > MAX_ENTRIES {
        return Err("That archive has too many files to open here.".into());
    }
    if unpacked > MAX_UNPACKED {
        return Err("That archive would unpack to more than 16 GB, so it was not opened.".into());
    }
    if unpacked > 100 * 1024 * 1024 && packed > 0 && unpacked / packed > MAX_RATIO {
        return Err("That archive unpacks to far more than it looks like, which is how a harmful archive works, so it was not opened.".into());
    }
    Ok(())
}

/// A folder name for unpacking: the archive's name without `.zip`, made usable as a folder.
pub fn folder_name_for(archive_file_name: &str) -> String {
    let stem = archive_file_name
        .rsplit_once('.')
        .filter(|(s, e)| !s.is_empty() && e.eq_ignore_ascii_case("zip"))
        .map(|(s, _)| s)
        .unwrap_or(archive_file_name);
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.').trim().to_string();
    if cleaned.is_empty() {
        "Unpacked".into()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_names_unpack_where_they_say() {
        assert_eq!(safe_entry("a.txt"), Some(PathBuf::from("a.txt")));
        assert_eq!(
            safe_entry("dir/sub/file.tar.gz"),
            Some(PathBuf::from("dir").join("sub").join("file.tar.gz"))
        );
        assert_eq!(safe_entry("dir/"), Some(PathBuf::from("dir")));
        assert_eq!(
            safe_entry("dir\\file.txt"),
            Some(PathBuf::from("dir").join("file.txt"))
        );
    }

    #[test]
    fn names_that_lead_elsewhere_are_refused() {
        for bad in [
            "../evil.txt",
            "a/../../evil.txt",
            "/etc/passwd",
            "C:/Windows/x.dll",
            "C:\\x",
            "a//b",
            "./x",
            "a/./b",
            "",
            "/",
            "CON",
            "dir/aux.txt",
            "a/b.",
            "a/b ",
            "a?b",
            "a\0b",
            "..\\..\\x",
        ] {
            assert_eq!(safe_entry(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn small_and_ordinary_archives_pass_and_bombs_do_not() {
        assert!(check_limits(10, 5_000, 2_000).is_ok());
        assert!(check_limits(10, 500 * 1024 * 1024, 100 * 1024 * 1024).is_ok());
        assert!(check_limits(MAX_ENTRIES + 1, 1, 1).is_err());
        assert!(check_limits(1, MAX_UNPACKED + 1, MAX_UNPACKED).is_err());
        // 10 GB from 1 MB.
        assert!(check_limits(1, 10 * 1024 * 1024 * 1024, 1024 * 1024).is_err());
        // A small file that compresses well is fine: the ratio only counts for big results.
        assert!(check_limits(1, 50 * 1024 * 1024, 1024).is_ok());
    }

    #[test]
    fn the_unpack_folder_is_named_after_the_archive() {
        assert_eq!(folder_name_for("Photos 2026.zip"), "Photos 2026");
        assert_eq!(folder_name_for("a.ZIP"), "a");
        assert_eq!(folder_name_for("weird:name?.zip"), "weird_name_");
        assert_eq!(folder_name_for(".zip"), ".zip");
        assert_eq!(folder_name_for("..zip"), "Unpacked");
    }
}
