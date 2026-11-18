//! Path comparison across platforms: strips `\\?\`, normalizes separators and letter case for
//! collision-free comparison (safe on case-insensitive NTFS/exFAT/vfat mounts on both Windows and
//! Linux), and detects drive/filesystem roots and parent traversal (`..`). Nothing touches disk.

use std::path::Path;

pub fn norm(p: &Path) -> String {
    let raw = p.to_string_lossy();
    let raw = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
    if raw.starts_with('/') {
        let s = raw.replace('\\', "/");
        if s.bytes().all(|b| b == b'/') {
            return "/".to_string();
        }
        return s.trim_end_matches('/').to_lowercase();
    }
    let s = raw.replace('/', "\\");
    s.trim_end_matches('\\').to_lowercase()
}

pub fn is_same(a: &Path, b: &Path) -> bool {
    norm(a) == norm(b)
}

/// True when `child` is `parent` or anywhere below it.
pub fn is_inside(child: &Path, parent: &Path) -> bool {
    let (c, p) = (norm(child), norm(parent));
    if p.is_empty() {
        return false;
    }
    if p == "/" {
        return c.starts_with('/');
    }
    let sep = if p.starts_with('/') { '/' } else { '\\' };
    c == p || c.starts_with(&format!("{p}{sep}"))
}

/// True for `C:\`, `/`, and other roots with nothing above them.
pub fn is_drive_root(p: &Path) -> bool {
    let n = norm(p);
    n == "/" || (n.len() <= 2 && n.ends_with(':')) || p.parent().is_none()
}

/// A path that tries to climb out of where it should be (`..`) or isn't absolute.
pub fn has_traversal(p: &Path) -> bool {
    !p.is_absolute()
        || p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn case_slashes_and_prefixes_do_not_matter() {
        assert!(is_same(
            Path::new(r"C:\Users\Me\Downloads"),
            Path::new("c:/users/me/downloads/")
        ));
        assert!(is_same(
            Path::new(r"\\?\C:\Users\Me"),
            Path::new(r"C:\USERS\ME")
        ));
        assert!(is_same(
            Path::new("/home/me/Downloads/"),
            Path::new("/home/me/downloads")
        ));
    }

    #[test]
    fn inside_means_a_whole_folder_boundary() {
        assert!(is_inside(Path::new(r"C:\a\b\c.txt"), Path::new(r"C:\a")));
        assert!(is_inside(Path::new(r"C:\a"), Path::new(r"C:\A")));
        assert!(
            !is_inside(Path::new(r"C:\ab\c"), Path::new(r"C:\a")),
            "a sibling that starts with the same letters"
        );
        assert!(!is_inside(Path::new(r"C:\a"), Path::new(r"C:\a\b")));
        assert!(is_inside(Path::new("/usr/bin/ls"), Path::new("/usr")));
        assert!(is_inside(Path::new("/usr/bin/ls"), Path::new("/")));
        assert!(!is_inside(Path::new("/usr_local/bin"), Path::new("/usr")));
    }

    #[test]
    fn roots_and_traversal() {
        assert!(is_drive_root(Path::new("/")));
        #[cfg(windows)]
        {
            assert!(is_drive_root(Path::new(r"C:\")));
            assert!(is_drive_root(Path::new("D:")));
            assert!(!is_drive_root(Path::new(r"C:\Users")));
            assert!(has_traversal(Path::new(r"C:\a\..\b")));
            assert!(!has_traversal(&PathBuf::from(r"C:\a\b")));
        }
        #[cfg(not(windows))]
        {
            assert!(!is_drive_root(Path::new("/home")));
            assert!(has_traversal(Path::new("/a/../b")));
            assert!(!has_traversal(&PathBuf::from("/a/b")));
        }
        assert!(has_traversal(Path::new("relative/x")));
    }
}
