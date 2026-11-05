//! Path comparison that matches how Windows treats names: case doesn't matter, and `\\?\` and
//! slash style don't change what a path means. Nothing here touches the disk.

use std::path::Path;

pub fn norm(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.trim_end_matches('\\').to_lowercase()
}

pub fn is_same(a: &Path, b: &Path) -> bool {
    norm(a) == norm(b)
}

/// True when `child` is `parent` or anywhere below it.
pub fn is_inside(child: &Path, parent: &Path) -> bool {
    let (c, p) = (norm(child), norm(parent));
    !p.is_empty() && (c == p || c.starts_with(&format!("{p}\\")))
}

/// True for `C:\` and other roots with nothing above them.
pub fn is_drive_root(p: &Path) -> bool {
    let n = norm(p);
    n.len() <= 2 && n.ends_with(':') || p.parent().is_none()
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
    }

    #[test]
    fn roots_and_traversal() {
        assert!(is_drive_root(Path::new(r"C:\")));
        assert!(is_drive_root(Path::new("D:")));
        assert!(!is_drive_root(Path::new(r"C:\Users")));
        assert!(has_traversal(Path::new(r"C:\a\..\b")));
        assert!(has_traversal(Path::new("relative\\x")));
        assert!(!has_traversal(&PathBuf::from(r"C:\a\b")));
    }
}
