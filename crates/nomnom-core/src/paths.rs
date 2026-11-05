//! Resolves well-known directories. On Windows these come from the Known Folder
//! API, never from hard-coded paths. Tests inject their own roots.

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
                paths.downloads = p;
            }
            if let Some(p) = over("NOMNOM_TEST_DOCUMENTS") {
                paths.documents = p;
            }
            if let Some(p) = over("NOMNOM_TEST_DATA") {
                paths.data_dir = p;
            }
        }
        Ok(paths)
    }

    /// Folders the organizer must never work inside (Windows, programs, app data, NOMNOM's own
    /// data) and folders it must never work on directly (the whole user profile).
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
    pub fn resolve() -> crate::Result<Self> {
        Err(crate::Error::Other(
            "NOMNOM currently supports Windows only".into(),
        ))
    }
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
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
        // Downloads and Documents sit inside the profile but are not protected zones
        assert!(
            !zones.iter().any(|z| paths.downloads.starts_with(z)),
            "Downloads must remain organizable"
        );
        assert!(exact[0] != paths.downloads);
    }
}
