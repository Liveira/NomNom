//! Lets Firefox find the NOMNOM native host: a manifest file plus one registry key under
//! `HKCU\Software\Mozilla\NativeMessagingHosts`.

use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

pub const HOST_NAME: &str = "com.nomnom.host";
pub const EXTENSION_ID: &str = "nomnom@nomnom.local";
/// Where Firefox looks. Tests pass a different root so the real one is never touched.
pub const FIREFOX_ROOT: &str = r"Software\Mozilla\NativeMessagingHosts";

#[derive(Debug, thiserror::Error)]
pub enum RegError {
    #[error("The host program wasn't found at {0}.")]
    HostMissing(PathBuf),
    #[error("Couldn't write the connection files: {0}")]
    Io(String),
}

pub fn manifest_json(host_exe: &Path) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": HOST_NAME,
        "description": "NomNom download helper for Firefox",
        "path": host_exe,
        "type": "stdio",
        "allowed_extensions": [EXTENSION_ID],
    }))
    .expect("manifest is plain data")
}

fn key_path(root: &str) -> String {
    format!(r"{root}\{HOST_NAME}")
}

/// Writes the manifest and registers it. Safe to run again (it repairs a broken setup).
pub fn install(host_exe: &Path, manifest_path: &Path, root: &str) -> Result<(), RegError> {
    if !host_exe.is_file() {
        return Err(RegError::HostMissing(host_exe.to_path_buf()));
    }
    if let Some(dir) = manifest_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| RegError::Io(e.to_string()))?;
    }
    std::fs::write(manifest_path, manifest_json(host_exe))
        .map_err(|e| RegError::Io(e.to_string()))?;
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(key_path(root))
        .map_err(|e| RegError::Io(e.to_string()))?;
    key.set_value("", &manifest_path.to_string_lossy().to_string())
        .map_err(|e| RegError::Io(e.to_string()))
}

/// Removes the registry key and the manifest file, but only if they are ours.
pub fn uninstall(manifest_path: &Path, root: &str) -> Result<(), RegError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.delete_subkey_all(key_path(root)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(RegError::Io(e.to_string())),
    }
    let ours = std::fs::read_to_string(manifest_path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .is_some_and(|v| v.get("name").and_then(|n| n.as_str()) == Some(HOST_NAME));
    if ours {
        std::fs::remove_file(manifest_path).map_err(|e| RegError::Io(e.to_string()))?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    NotRegistered,
    /// Registered, but the manifest or the host program it points to is missing or wrong.
    Broken(String),
    Ready,
}

pub fn status(root: &str) -> Status {
    let Ok(key) =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(key_path(root), KEY_READ)
    else {
        return Status::NotRegistered;
    };
    let Ok(manifest) = key.get_value::<String, _>("") else {
        return Status::Broken("The registry entry is empty.".into());
    };
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return Status::Broken("The connection file is missing.".into());
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Status::Broken("The connection file is damaged.".into());
    };
    let host = v.get("path").and_then(|p| p.as_str()).map(PathBuf::from);
    if !host.as_deref().is_some_and(Path::is_file) {
        return Status::Broken("The helper program it points to is missing.".into());
    }
    let allowed = v
        .get("allowed_extensions")
        .and_then(|a| a.as_array())
        .is_some_and(|a| a.iter().any(|x| x.as_str() == Some(EXTENSION_ID)));
    if !allowed {
        return Status::Broken("The connection file doesn't allow the NomNom extension.".into());
    }
    Status::Ready
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway registry root that is removed afterwards. Never the real Firefox key.
    struct TestRoot(String);
    impl TestRoot {
        fn new(tag: &str) -> Self {
            TestRoot(format!(
                r"Software\NOMNOM-Test\{tag}-{}",
                std::process::id()
            ))
        }
    }
    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(r"Software\NOMNOM-Test");
        }
    }

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let host = dir.path().join("nomnom-native-host.exe");
        std::fs::write(&host, b"x").unwrap();
        let manifest = dir.path().join("com.nomnom.host.json");
        (dir, host, manifest)
    }

    #[test]
    fn install_makes_firefox_able_to_find_the_host_and_uninstall_removes_it() {
        let root = TestRoot::new("roundtrip");
        let (_dir, host, manifest) = setup();
        assert_eq!(status(&root.0), Status::NotRegistered);

        install(&host, &manifest, &root.0).unwrap();
        assert_eq!(status(&root.0), Status::Ready);
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
        assert_eq!(v["name"], HOST_NAME);
        assert_eq!(v["type"], "stdio");
        assert_eq!(v["allowed_extensions"], serde_json::json!([EXTENSION_ID]));
        assert_eq!(
            PathBuf::from(v["path"].as_str().unwrap()),
            host,
            "path survives JSON escaping"
        );

        uninstall(&manifest, &root.0).unwrap();
        assert_eq!(status(&root.0), Status::NotRegistered);
        assert!(!manifest.exists());
        assert!(host.exists(), "the host program itself is never deleted");
    }

    #[test]
    fn installing_twice_is_fine_and_repairs_a_damaged_setup() {
        let root = TestRoot::new("repair");
        let (_dir, host, manifest) = setup();
        install(&host, &manifest, &root.0).unwrap();
        std::fs::write(&manifest, "garbage").unwrap();
        assert!(matches!(status(&root.0), Status::Broken(_)));
        install(&host, &manifest, &root.0).unwrap();
        assert_eq!(status(&root.0), Status::Ready);
    }

    #[test]
    fn a_missing_host_is_refused_and_a_moved_host_is_reported() {
        let root = TestRoot::new("missing");
        let (dir, host, manifest) = setup();
        assert!(matches!(
            install(&dir.path().join("nope.exe"), &manifest, &root.0),
            Err(RegError::HostMissing(_))
        ));
        install(&host, &manifest, &root.0).unwrap();
        std::fs::remove_file(&host).unwrap();
        assert!(matches!(status(&root.0), Status::Broken(m) if m.contains("helper")));
    }

    #[test]
    fn uninstall_never_deletes_a_file_that_is_not_ours() {
        let root = TestRoot::new("foreign");
        let (_dir, host, manifest) = setup();
        install(&host, &manifest, &root.0).unwrap();
        std::fs::write(&manifest, r#"{"name":"someone.elses.host"}"#).unwrap();
        uninstall(&manifest, &root.0).unwrap();
        assert!(
            manifest.exists(),
            "a manifest that isn't ours is left alone"
        );
    }

    #[test]
    fn uninstalling_when_nothing_is_installed_is_not_an_error() {
        let root = TestRoot::new("none");
        let (_dir, _host, manifest) = setup();
        uninstall(&manifest, &root.0).unwrap();
    }

    #[test]
    fn a_manifest_that_does_not_allow_our_extension_is_broken() {
        let root = TestRoot::new("allow");
        let (_dir, host, manifest) = setup();
        install(&host, &manifest, &root.0).unwrap();
        let mut v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
        v["allowed_extensions"] = serde_json::json!(["someone@else"]);
        std::fs::write(&manifest, v.to_string()).unwrap();
        assert!(matches!(status(&root.0), Status::Broken(m) if m.contains("extension")));
    }
}
