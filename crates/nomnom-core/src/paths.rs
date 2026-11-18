//! Resolves well-known directories. On Windows these come from the Known Folder
//! API; on Linux they follow the XDG Base Directory and `user-dirs.dirs` standards.
//! Tests inject their own roots.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Paths {
    pub downloads: PathBuf,
    pub documents: PathBuf,
    /// NOMNOM's own data directory (settings, database, logs, tools).
    pub data_dir: PathBuf,
}

impl Paths {
    /// Builds a `Paths` rooted entirely inside `root`. Used by tests.
    pub fn sandboxed(root: &Path) -> Self {
        Paths {
            downloads: root.join("Downloads"),
            documents: root.join("Documents"),
            data_dir: root.join("NOMNOM"),
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }
    pub fn db_file(&self) -> PathBuf {
        self.data_dir.join("nomnom.db")
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }
    pub fn tools_dir(&self) -> PathBuf {
        self.data_dir.join("tools")
    }

    #[cfg(windows)]
    pub fn resolve() -> crate::Result<Self> {
        use windows::Win32::UI::Shell::{
            FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_LocalAppData,
        };
        #[allow(unused_mut)]
        let mut paths = Paths {
            downloads: known_folder(&FOLDERID_Downloads)?,
            documents: known_folder(&FOLDERID_Documents)?,
            data_dir: known_folder(&FOLDERID_LocalAppData)?.join("NOMNOM"),
        };
        apply_debug_overrides(&mut paths);
        Ok(paths)
    }

    #[cfg(not(windows))]
    pub fn resolve() -> crate::Result<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or_else(|| crate::Error::Other("HOME is not set or not an absolute path".into()))?;
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local").join("share"));
        let user_dirs = std::fs::read_to_string(config_home.join("user-dirs.dirs")).ok();

        #[allow(unused_mut)]
        let mut paths = Paths {
            downloads: resolve_xdg_user_dir(
                user_dirs.as_deref(),
                &home,
                "XDG_DOWNLOAD_DIR",
                "Downloads",
            ),
            documents: resolve_xdg_user_dir(
                user_dirs.as_deref(),
                &home,
                "XDG_DOCUMENTS_DIR",
                "Documents",
            ),
            data_dir: data_home.join("nomnom"),
        };
        apply_debug_overrides(&mut paths);
        Ok(paths)
    }

    /// Folders the organizer must never work inside (OS directories, programs, app data, NomNom's
    /// own data) and folders it must never work on directly (the whole user profile).
    #[cfg(windows)]
    pub fn protected_folders(&self) -> (Vec<PathBuf>, Vec<PathBuf>) {
        use windows::Win32::UI::Shell::{
            FOLDERID_LocalAppData, FOLDERID_Profile, FOLDERID_ProgramData, FOLDERID_ProgramFiles,
            FOLDERID_ProgramFilesX86, FOLDERID_RoamingAppData, FOLDERID_Windows,
        };
        let mut zones: Vec<PathBuf> = [
            &FOLDERID_Windows,
            &FOLDERID_ProgramFiles,
            &FOLDERID_ProgramFilesX86,
            &FOLDERID_ProgramData,
            &FOLDERID_RoamingAppData,
            &FOLDERID_LocalAppData,
        ]
        .into_iter()
        .filter_map(|id| known_folder(id).ok())
        .collect();
        zones.push(self.data_dir.clone());
        let exact = known_folder(&FOLDERID_Profile).ok().into_iter().collect();
        (zones, exact)
    }

    #[cfg(not(windows))]
    pub fn protected_folders(&self) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut zones: Vec<PathBuf> = [
            "/usr", "/etc", "/bin", "/sbin", "/lib", "/lib64", "/boot", "/proc", "/sys", "/dev",
            "/run", "/var", "/opt", "/root", "/snap",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();
        let mut exact: Vec<PathBuf> = vec![PathBuf::from("/"), PathBuf::from("/home")];
        if let Some(home) = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            for dot in [".config", ".local", ".cache", ".ssh", ".gnupg", ".mozilla"] {
                zones.push(home.join(dot));
            }
            exact.push(home);
        }
        zones.push(self.data_dir.clone());
        (zones, exact)
    }
}

fn apply_debug_overrides(_paths: &mut Paths) {
    // Debug builds only (compiled out of release builds): lets tests run against throwaway
    // folders without touching the real Downloads, Documents, settings or history.
    #[cfg(debug_assertions)]
    {
        let over = |name: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
        };
        if let Some(p) = over("NOMNOM_TEST_DOWNLOADS") {
            _paths.downloads = p;
        }
        if let Some(p) = over("NOMNOM_TEST_DOCUMENTS") {
            _paths.documents = p;
        }
        if let Some(p) = over("NOMNOM_TEST_DATA") {
            _paths.data_dir = p;
        }
    }
}

/// Parses an entry such as `XDG_DOWNLOAD_DIR="$HOME/Downloads"` from `user-dirs.dirs`, falling
/// back to `home.join(fallback)` when unset or relative.
pub fn resolve_xdg_user_dir(
    user_dirs_text: Option<&str>,
    home: &Path,
    key: &str,
    fallback: &str,
) -> PathBuf {
    if let Some(text) = user_dirs_text {
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            if k.trim() != key {
                continue;
            }
            let unquoted = v.trim().trim_matches('"');
            let resolved = if unquoted == "$HOME" || unquoted == "${HOME}" {
                // XDG disables a user dir by pointing it at $HOME itself; use the named subfolder.
                home.join(fallback)
            } else if let Some(rel) = unquoted
                .strip_prefix("$HOME/")
                .or_else(|| unquoted.strip_prefix("${HOME}/"))
            {
                home.join(rel)
            } else {
                PathBuf::from(unquoted)
            };
            if resolved.is_absolute() {
                return resolved;
            }
        }
    }
    home.join(fallback)
}

#[cfg(windows)]
fn known_folder(id: &windows::core::GUID) -> crate::Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};
    // SAFETY: SHGetKnownFolderPath returns a CoTaskMem-allocated, NUL-terminated wide string that
    // we copy and then free exactly once.
    unsafe {
        let pwstr = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None)
            .map_err(|e| crate::Error::Other(format!("known folder lookup failed: {e}")))?;
        let s = std::ffi::OsString::from_wide(pwstr.as_wide());
        CoTaskMemFree(Some(pwstr.0 as *const _));
        Ok(PathBuf::from(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xdg_user_dirs_with_home_and_absolute_paths() {
        let home = if cfg!(windows) {
            Path::new(r"C:\Users\Arch")
        } else {
            Path::new("/home/arch")
        };
        let sample = r#"
            # Written by xdg-user-dirs-update
            XDG_DOWNLOAD_DIR="$HOME/Downloads"
            XDG_DOCUMENTS_DIR="$HOME/Documentos"
        "#;
        assert_eq!(
            resolve_xdg_user_dir(Some(sample), home, "XDG_DOWNLOAD_DIR", "Downloads"),
            home.join("Downloads")
        );
        assert_eq!(
            resolve_xdg_user_dir(Some(sample), home, "XDG_DOCUMENTS_DIR", "Documents"),
            home.join("Documentos")
        );
        assert_eq!(
            resolve_xdg_user_dir(None, home, "XDG_DOWNLOAD_DIR", "Downloads"),
            home.join("Downloads")
        );
    }

    #[test]
    #[cfg(windows)]
    fn the_system_folders_are_found_and_look_right() {
        let paths = Paths::resolve().unwrap();
        let (zones, exact) = paths.protected_folders();
        assert!(zones.len() >= 5, "{zones:?}");
        assert!(zones
            .iter()
            .any(|z| z.to_string_lossy().to_lowercase().ends_with("windows")));
        assert!(zones
            .iter()
            .any(|z| z.to_string_lossy().to_lowercase().contains("program files")));
        assert_eq!(exact.len(), 1);
        assert!(
            !zones.iter().any(|z| paths.downloads.starts_with(z)),
            "Downloads must remain organizable"
        );
        assert!(exact[0] != paths.downloads);
    }

    #[test]
    #[cfg(not(windows))]
    fn linux_system_folders_protect_os_and_dotfiles_while_keeping_downloads_free() {
        let paths = Paths::resolve().unwrap();
        let (zones, exact) = paths.protected_folders();
        assert!(zones.iter().any(|z| z == Path::new("/usr")));
        assert!(zones.iter().any(|z| z == Path::new("/etc")));
        assert!(exact.iter().any(|z| z == Path::new("/")));
        assert!(
            !zones.iter().any(|z| paths.downloads.starts_with(z)),
            "Downloads must remain organizable"
        );
    }
}
