//! Places NomNom will not delete from, or will only delete from after a stronger confirmation.

use nomnom_organizer::pathutil::{is_drive_root, is_inside, is_same};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    /// Allowed, but the person must be told and (for permanent deletes) confirm more strongly.
    Warn(String),
    Refuse(String),
}

#[derive(Debug, Clone, Default)]
pub struct Protected {
    /// Folders whose contents are sensitive (Program Files, AppData, `~/.config`, `~/.local`, NomNom's data).
    pub careful: Vec<PathBuf>,
    /// Folders that may not be touched at all, neither the folder nor anything in it.
    pub forbidden: Vec<PathBuf>,
    /// Folders that may not be deleted or moved themselves, though their contents can be managed
    /// (the user profile / `$HOME`, `C:\Users` / `/home`, the careful folders' own roots).
    pub exact: Vec<PathBuf>,
}

impl Protected {
    /// From the zones NomNom already knows (`Paths::protected_folders`) plus OS system roots.
    pub fn from_zones(zones: Vec<PathBuf>, exact: Vec<PathBuf>) -> Protected {
        let mut forbidden = Vec::new();
        let mut exact_all = exact;
        #[cfg(windows)]
        {
            for var in ["SystemRoot", "windir"] {
                if let Some(w) = std::env::var_os(var) {
                    forbidden.push(PathBuf::from(w));
                }
            }
            exact_all.extend(zones.iter().cloned());
            if let Some(home) = std::env::var_os("SystemDrive") {
                exact_all.push(PathBuf::from(format!("{}\\Users", home.to_string_lossy())));
            }
        }
        #[cfg(not(windows))]
        {
            for sys in [
                "/proc", "/sys", "/dev", "/boot", "/usr", "/bin", "/sbin", "/lib", "/lib64",
                "/etc", "/run",
            ] {
                forbidden.push(PathBuf::from(sys));
            }
            exact_all.extend(zones.iter().cloned());
            for root_exact in ["/", "/home", "/root", "/var", "/opt"] {
                exact_all.push(PathBuf::from(root_exact));
            }
        }
        Protected {
            careful: zones,
            forbidden,
            exact: exact_all,
        }
    }

    pub fn check(&self, path: &Path) -> Verdict {
        if !path.is_absolute() || nomnom_organizer::pathutil::has_traversal(path) {
            return Verdict::Refuse("The path isn't a complete, plain path.".into());
        }
        if is_drive_root(path) {
            return Verdict::Refuse("A whole drive can't be deleted or moved from here.".into());
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_ascii_lowercase();
            if matches!(
                lower.as_str(),
                "$recycle.bin"
                    | "system volume information"
                    | "pagefile.sys"
                    | "hiberfil.sys"
                    | "swapfile.sys"
                    | "$windows.~bt"
                    | "$winreagent"
                    | "lost+found"
                    | "swapfile"
            ) && path.parent().is_some_and(is_drive_root)
            {
                return Verdict::Refuse(
                    "The operating system needs this item. It isn't safe to remove.".into(),
                );
            }
        }
        if self.forbidden.iter().any(|f| is_inside(path, f)) {
            return Verdict::Refuse(
                "This is part of the operating system. NomNom won't change it.".into(),
            );
        }
        if self.exact.iter().any(|e| is_same(path, e)) {
            return Verdict::Refuse("This folder holds programs or your account's data. Work on what is inside it instead.".into());
        }
        if self.careful.iter().any(|c| is_inside(path, c)) {
            return Verdict::Warn("This is inside a program or app-data folder. Removing it can break an installed program.".into());
        }
        Verdict::Ok
    }
}

/// Checks that `name` is safe for a new file or folder across filesystems (including NTFS/exFAT/vfat).
pub fn valid_new_name(name: &str) -> Result<(), String> {
    let n = name.trim_end_matches([' ', '.']);
    if name.is_empty() || n.is_empty() {
        return Err("The name can't be empty.".into());
    }
    if name.len() > 255 || name.encode_utf16().count() > 255 {
        return Err("The name is too long (255 characters at most).".into());
    }
    if name.chars().any(|c| {
        matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 32
    }) {
        return Err("A name can't contain < > : \" / \\ | ? * or control characters.".into());
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"];
    let is_dev = reserved.contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0');
    if is_dev {
        return Err("That name is reserved for a system device. Pick another.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn set() -> Protected {
        Protected {
            careful: vec![
                PathBuf::from(r"C:\Program Files"),
                PathBuf::from(r"C:\ProgramData"),
            ],
            forbidden: vec![PathBuf::from(r"C:\Windows")],
            exact: vec![
                PathBuf::from(r"C:\Users\Me"),
                PathBuf::from(r"C:\Program Files"),
            ],
        }
    }

    #[cfg(not(windows))]
    fn set() -> Protected {
        Protected {
            careful: vec![PathBuf::from("/opt"), PathBuf::from("/home/me/.config")],
            forbidden: vec![PathBuf::from("/usr"), PathBuf::from("/etc")],
            exact: vec![PathBuf::from("/home/me"), PathBuf::from("/opt")],
        }
    }

    #[test]
    #[cfg(windows)]
    fn drives_and_windows_are_refused() {
        let p = set();
        assert!(matches!(p.check(Path::new(r"C:\")), Verdict::Refuse(_)));
        assert!(matches!(
            p.check(Path::new(r"C:\Windows")),
            Verdict::Refuse(_)
        ));
        assert!(matches!(
            p.check(Path::new(r"c:\windows\system32\drivers\x.sys")),
            Verdict::Refuse(_)
        ));
        assert!(matches!(
            p.check(Path::new(r"C:\$Recycle.Bin")),
            Verdict::Refuse(_)
        ));
        assert!(matches!(
            p.check(Path::new(r"C:\pagefile.sys")),
            Verdict::Refuse(_)
        ));
        assert!(matches!(
            p.check(Path::new(r"C:\System Volume Information")),
            Verdict::Refuse(_)
        ));
    }

    #[test]
    #[cfg(not(windows))]
    fn root_and_system_dirs_are_refused_on_linux() {
        let p = set();
        assert!(matches!(p.check(Path::new("/")), Verdict::Refuse(_)));
        assert!(matches!(p.check(Path::new("/usr")), Verdict::Refuse(_)));
        assert!(matches!(
            p.check(Path::new("/usr/bin/pacman")),
            Verdict::Refuse(_)
        ));
        assert!(matches!(
            p.check(Path::new("/lost+found")),
            Verdict::Refuse(_)
        ));
    }

    #[test]
    fn the_profile_folder_itself_is_refused_but_its_contents_are_fine() {
        let p = set();
        let home = if cfg!(windows) {
            Path::new(r"C:\Users\Me")
        } else {
            Path::new("/home/me")
        };
        let vid = if cfg!(windows) {
            Path::new(r"C:\Users\Me\Videos\old.mp4")
        } else {
            Path::new("/home/me/Videos/old.mp4")
        };
        assert!(matches!(p.check(home), Verdict::Refuse(_)));
        assert_eq!(p.check(vid), Verdict::Ok);
    }

    #[test]
    fn program_folders_warn_and_their_roots_are_refused() {
        let p = set();
        let (prog_root, prog_inside, data_inside) = if cfg!(windows) {
            (
                Path::new(r"C:\Program Files"),
                Path::new(r"C:\Program Files\App\data.bin"),
                Path::new(r"C:\ProgramData\Thing"),
            )
        } else {
            (
                Path::new("/opt"),
                Path::new("/opt/app/data.bin"),
                Path::new("/home/me/.config/app"),
            )
        };
        assert!(matches!(p.check(prog_root), Verdict::Refuse(_)));
        assert!(matches!(p.check(prog_inside), Verdict::Warn(_)));
        assert!(matches!(p.check(data_inside), Verdict::Warn(_)));
    }

    #[test]
    fn relative_and_climbing_paths_are_refused() {
        let p = set();
        assert!(matches!(
            p.check(Path::new("folder/file.txt")),
            Verdict::Refuse(_)
        ));
        let climb = if cfg!(windows) {
            Path::new(r"C:\Users\Me\..\..\Windows")
        } else {
            Path::new("/home/me/../../usr")
        };
        assert!(matches!(p.check(climb), Verdict::Refuse(_)));
    }

    #[test]
    fn new_names_are_checked() {
        assert!(valid_new_name("report (final).docx").is_ok());
        for bad in [
            "",
            "   ",
            "a/b",
            "a:b",
            "what?",
            "CON",
            "nul.txt",
            "com1",
            "x*y",
            &"a".repeat(256),
        ] {
            assert!(valid_new_name(bad).is_err(), "{bad:?}");
        }
        assert!(valid_new_name("COM0").is_ok());
        assert!(valid_new_name("console").is_ok());
    }
}
