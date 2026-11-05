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
        use windows::core::GUID;
        use windows::Win32::System::Com::CoTaskMemFree;
        use windows::Win32::UI::Shell::{
            FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_LocalAppData, SHGetKnownFolderPath,
            KF_FLAG_DEFAULT,
        };

        fn known(id: &GUID) -> crate::Result<PathBuf> {
            use std::os::windows::ffi::OsStringExt;
            // SAFETY: SHGetKnownFolderPath returns a CoTaskMem-allocated, NUL-terminated
            // wide string that we copy and then free exactly once.
            unsafe {
                let pwstr = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None)
                    .map_err(|e| crate::Error::Other(format!("known folder lookup failed: {e}")))?;
                let s = std::ffi::OsString::from_wide(pwstr.as_wide());
                CoTaskMemFree(Some(pwstr.0 as *const _));
                Ok(PathBuf::from(s))
            }
        }

        Ok(Paths {
            downloads: known(&FOLDERID_Downloads)?,
            documents: known(&FOLDERID_Documents)?,
            data_dir: known(&FOLDERID_LocalAppData)?.join("NOMNOM"),
        })
    }

    #[cfg(not(windows))]
    pub fn resolve() -> crate::Result<Self> {
        Err(crate::Error::Other(
            "NOMNOM currently supports Windows only".into(),
        ))
    }
}
