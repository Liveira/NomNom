//! Registers the NomNom native messaging host for Firefox (and LibreWolf when present) on Linux.
//!
//! Mozilla browsers on Linux read per-user host manifests from
//! `~/.mozilla/native-messaging-hosts/com.nomnom.host.json` (and `~/.librewolf/native-messaging-hosts`
//! for LibreWolf, common on Arch Linux).

use std::path::{Path, PathBuf};

pub const HOST_NAME: &str = "com.nomnom.host";
pub const EXTENSION_ID: &str = "nomnom@nomnom.local";
/// The Chrome, Chromium and Edge extension's ID (it comes from the public key in the manifest).
pub const CHROME_EXTENSION_ID: &str = "kejgmoigmjpbghoihdjlahegijloeghc";
pub const MANIFEST_FILE: &str = "com.nomnom.host.json";

#[derive(Debug, thiserror::Error)]
pub enum RegError {
    #[error("The host program wasn't found at {0}.")]
    HostMissing(PathBuf),
    #[error("Couldn't write the connection files: {0}")]
    Io(String),
}

pub fn default_hosts_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())?;
    Some(home.join(".mozilla").join("native-messaging-hosts"))
}

fn companion_librewolf_dir(mozilla_hosts_dir: &Path) -> Option<PathBuf> {
    let mozilla_dir = mozilla_hosts_dir.parent()?;
    if mozilla_dir.file_name().and_then(|n| n.to_str()) != Some(".mozilla") {
        return None;
    }
    let home = mozilla_dir.parent()?;
    let lw = home.join(".librewolf");
    lw.is_dir().then(|| lw.join("native-messaging-hosts"))
}

/// The folders where Chromium-based browsers read host manifests, for the browsers that are
/// installed (their settings folder exists).
fn companion_chromium_dirs(mozilla_hosts_dir: &Path) -> Vec<PathBuf> {
    let Some(home) = mozilla_hosts_dir
        .parent()
        .filter(|m| m.file_name().and_then(|n| n.to_str()) == Some(".mozilla"))
        .and_then(Path::parent)
    else {
        return Vec::new();
    };
    [
        "google-chrome",
        "chromium",
        "microsoft-edge",
        "BraveSoftware/Brave-Browser",
    ]
    .iter()
    .map(|b| home.join(".config").join(b))
    .filter(|d| d.is_dir())
    .map(|d| d.join("NativeMessagingHosts"))
    .collect()
}

pub fn manifest_json(host_exe: &Path) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": HOST_NAME,
        "description": "NomNom download helper for Firefox",
        "path": host_exe,
        "type": "stdio",
        "allowed_extensions": [EXTENSION_ID],
        "allowed_origins": [format!("chrome-extension://{CHROME_EXTENSION_ID}/")],
    }))
    .expect("manifest is plain data")
}

/// Writes `com.nomnom.host.json` into `hosts_dir` (and LibreWolf's native-messaging-hosts when
/// `~/.librewolf` exists). Safe to run repeatedly to repair a moved install.
pub fn install(host_exe: &Path, hosts_dir: &Path) -> Result<(), RegError> {
    if !host_exe.is_file() {
        return Err(RegError::HostMissing(host_exe.to_path_buf()));
    }
    let body = manifest_json(host_exe);
    std::fs::create_dir_all(hosts_dir).map_err(|e| RegError::Io(e.to_string()))?;
    std::fs::write(hosts_dir.join(MANIFEST_FILE), &body)
        .map_err(|e| RegError::Io(e.to_string()))?;

    if let Some(lw_hosts) = companion_librewolf_dir(hosts_dir) {
        let _ = std::fs::create_dir_all(&lw_hosts);
        let _ = std::fs::write(lw_hosts.join(MANIFEST_FILE), &body);
    }
    for dir in companion_chromium_dirs(hosts_dir) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(MANIFEST_FILE), &body);
    }
    Ok(())
}

fn remove_if_ours(file: &Path) -> Result<(), RegError> {
    if !file.exists() {
        return Ok(());
    }
    let ours = std::fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .is_some_and(|v| v.get("name").and_then(|n| n.as_str()) == Some(HOST_NAME));
    if ours {
        match std::fs::remove_file(file) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(RegError::Io(e.to_string())),
        }
    }
    Ok(())
}

/// Removes `com.nomnom.host.json` only if it belongs to NomNom.
pub fn uninstall(hosts_dir: &Path) -> Result<(), RegError> {
    remove_if_ours(&hosts_dir.join(MANIFEST_FILE))?;
    if let Some(lw_hosts) = companion_librewolf_dir(hosts_dir) {
        let _ = remove_if_ours(&lw_hosts.join(MANIFEST_FILE));
    }
    for dir in companion_chromium_dirs(hosts_dir) {
        let _ = remove_if_ours(&dir.join(MANIFEST_FILE));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    NotRegistered,
    Broken(String),
    Ready,
}

pub fn status(hosts_dir: &Path) -> Status {
    let manifest = hosts_dir.join(MANIFEST_FILE);
    if !manifest.exists() {
        return Status::NotRegistered;
    }
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return Status::Broken("The connection file couldn't be read.".into());
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

    #[test]
    fn install_status_repair_and_uninstall_work_safely() {
        let dir = tempfile::tempdir().unwrap();
        let host = dir.path().join("nomnom-native-host");
        std::fs::write(&host, b"#!/bin/sh\n").unwrap();
        let hosts_dir = dir.path().join(".mozilla").join("native-messaging-hosts");
        let lw_dir = dir.path().join(".librewolf");
        std::fs::create_dir_all(&lw_dir).unwrap();

        assert_eq!(status(&hosts_dir), Status::NotRegistered);
        install(&host, &hosts_dir).unwrap();
        assert_eq!(status(&hosts_dir), Status::Ready);
        assert!(lw_dir
            .join("native-messaging-hosts")
            .join(MANIFEST_FILE)
            .is_file());

        // Foreign manifest is never deleted.
        let foreign = hosts_dir.join(MANIFEST_FILE);
        std::fs::write(&foreign, r#"{"name":"other.host"}"#).unwrap();
        uninstall(&hosts_dir).unwrap();
        assert!(foreign.exists());

        // Repair overwrites broken manifest and uninstall removes ours.
        install(&host, &hosts_dir).unwrap();
        assert_eq!(status(&hosts_dir), Status::Ready);
        uninstall(&hosts_dir).unwrap();
        assert_eq!(status(&hosts_dir), Status::NotRegistered);
    }
}
