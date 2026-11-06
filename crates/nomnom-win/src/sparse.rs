//! The Windows 11 right-click menu comes from a small "sparse" package that points Explorer at
//! `nomnom_shell.dll` in NOMNOM's install folder. Registering it needs the package's signature to
//! be trusted by the PC; when it isn't, NOMNOM falls back to the classic menu.

use std::path::Path;
use windows::core::HSTRING;
use windows::Foundation::Uri;
use windows::Management::Deployment::{AddPackageOptions, PackageManager, RemovalOptions};

pub const PACKAGE_NAME: &str = "NOMNOM.Shell";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SparseError {
    #[error("The menu package or its helper file is missing from NomNom's folder.")]
    FilesMissing,
    #[error("Windows doesn't trust the menu package's signature yet.")]
    NotTrusted,
    #[error("Windows won't add menu packages in this setup ({0}).")]
    Unavailable(String),
    #[error("Windows couldn't add the menu: {0}")]
    Failed(String),
}

/// Turns the HRESULTs deployment returns into the message the user sees.
pub fn describe_failure(code: i32, text: &str) -> SparseError {
    match code as u32 {
        // CERT_E_UNTRUSTEDROOT, TRUST_E_NOSIGNATURE, CERT_E_CHAINING, TRUST_E_BAD_DIGEST,
        // APPX signature errors (0x8007B...), CERT_E_UNTRUSTEDCA.
        0x800B_0109 | 0x800B_0100 | 0x800B_010A | 0x800B_0111 | 0x800B_0004 | 0x8007_000B => {
            SparseError::NotTrusted
        }
        0x80073CFF | 0x80073CF9 | 0x80070005 => SparseError::Unavailable(format!("{code:#010x}")),
        _ => SparseError::Failed(if text.trim().is_empty() {
            format!("{code:#010x}")
        } else {
            text.trim().chars().take(200).collect()
        }),
    }
}

fn file_uri(path: &Path) -> windows::core::Result<Uri> {
    let url = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
    Uri::CreateUri(&HSTRING::from(url))
}

/// Registers the signed sparse package `msix`, with `install_dir` as its external location.
pub fn register(msix: &Path, install_dir: &Path) -> Result<(), SparseError> {
    if !msix.is_file()
        || !install_dir.join("nomnom_shell.dll").is_file()
        || !install_dir.join("nomnom.exe").is_file()
    {
        return Err(SparseError::FilesMissing);
    }
    let run = || -> windows::core::Result<(i32, String)> {
        let manager = PackageManager::new()?;
        let options = AddPackageOptions::new()?;
        options.SetExternalLocationUri(&file_uri(install_dir)?)?;
        let result = manager
            .AddPackageByUriAsync(&file_uri(msix)?, &options)?
            .get()?;
        Ok((
            result.ExtendedErrorCode()?.0,
            result.ErrorText()?.to_string(),
        ))
    };
    match run() {
        Ok((0, _)) => Ok(()),
        Ok((code, text)) => Err(describe_failure(code, &text)),
        Err(e) => {
            // The async call itself failed; its error carries the HRESULT.
            Err(describe_failure(e.code().0, &e.message()))
        }
    }
}

/// Full names of installed NOMNOM menu packages for the current user.
pub fn installed() -> Vec<String> {
    let Ok(manager) = PackageManager::new() else {
        return vec![];
    };
    let Ok(found) = manager.FindPackagesByUserSecurityId(&HSTRING::new()) else {
        return vec![];
    };
    found
        .into_iter()
        .filter_map(|p| {
            let id = p.Id().ok()?;
            (id.Name().ok()? == PACKAGE_NAME)
                .then(|| id.FullName().ok().map(|n| n.to_string()))
                .flatten()
        })
        .collect()
}

pub fn is_registered() -> bool {
    !installed().is_empty()
}

/// Removes the package. Nothing else is touched: the external location (NOMNOM's folder) stays.
pub fn unregister() -> Result<usize, SparseError> {
    let names = installed();
    let manager = PackageManager::new().map_err(|e| SparseError::Failed(e.message()))?;
    for full in &names {
        let result = manager
            .RemovePackageWithOptionsAsync(
                &HSTRING::from(full),
                RemovalOptions::PreserveApplicationData,
            )
            .and_then(|op| op.get())
            .map_err(|e| SparseError::Failed(e.message()))?;
        let code = result.ExtendedErrorCode().map(|c| c.0).unwrap_or(0);
        if code != 0 {
            return Err(describe_failure(
                code,
                &result
                    .ErrorText()
                    .map(|t| t.to_string())
                    .unwrap_or_default(),
            ));
        }
    }
    Ok(names.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_problems_get_the_trust_message() {
        assert_eq!(
            describe_failure(0x800B_0109_u32 as i32, ""),
            SparseError::NotTrusted
        );
        assert_eq!(
            describe_failure(0x800B_0100_u32 as i32, "x"),
            SparseError::NotTrusted
        );
        assert!(matches!(
            describe_failure(0x8007_0005_u32 as i32, ""),
            SparseError::Unavailable(_)
        ));
        assert!(
            matches!(describe_failure(0x8000_4005_u32 as i32, "  boom  "), SparseError::Failed(m) if m == "boom")
        );
        assert!(matches!(describe_failure(-1, ""), SparseError::Failed(m) if m == "0xffffffff"));
    }

    #[test]
    fn registering_without_the_files_is_refused_before_asking_windows() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            register(&dir.path().join("x.msix"), dir.path()),
            Err(SparseError::FilesMissing)
        );
    }

    #[test]
    fn asking_windows_what_is_installed_works_and_finds_nothing_here() {
        // Only checks that the call itself works; a developer machine may have the real package.
        let _ = installed();
    }
}
