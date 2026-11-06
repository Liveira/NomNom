//! Start NOMNOM with Windows: one value under the per-user Run key. Off unless the user asks.

use std::path::Path;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// Not plain "NOMNOM": another program may already use that name, and we must never replace its entry.
pub const VALUE_NAME: &str = "NomNom Utility Hub";

fn quoted(exe: &Path) -> String {
    format!("\"{}\" --background", exe.display())
}

pub fn set(enabled: bool, exe: &Path, key_path: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if enabled {
        let (key, _) = hkcu.create_subkey(key_path).map_err(|e| e.to_string())?;
        key.set_value(VALUE_NAME, &quoted(exe))
            .map_err(|e| e.to_string())
    } else {
        let Ok(key) = hkcu.open_subkey_with_flags(key_path, KEY_SET_VALUE) else {
            return Ok(());
        };
        match key.delete_value(VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// True when the Run entry exists. `None` means it exists but points somewhere else (a moved or
/// reinstalled NOMNOM), which the caller can repair by setting it again.
pub fn status(exe: &Path, key_path: &str) -> Option<bool> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(key_path, KEY_READ)
        .ok()?;
    let value: String = key.get_value(VALUE_NAME).ok()?;
    if value == quoted(exe) {
        Some(true)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestKey(String);
    impl TestKey {
        // Each test gets a whole key of its own, so tests running side by side never share one.
        fn new(tag: &str) -> Self {
            TestKey(format!(
                r"Software\NOMNOM-Test-run-{tag}-{}",
                std::process::id()
            ))
        }
    }
    impl Drop for TestKey {
        fn drop(&mut self) {
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.0);
        }
    }

    #[test]
    fn turning_it_on_and_off_only_touches_our_value() {
        let k = TestKey::new("onoff");
        let exe = Path::new(r"C:\Program Files\NomNom\nomnom.exe");
        // Another program's entry in the same key must survive.
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(&k.0)
            .unwrap();
        key.set_value("Other", &"other.exe").unwrap();

        assert_eq!(status(exe, &k.0), None);
        set(true, exe, &k.0).unwrap();
        assert_eq!(status(exe, &k.0), Some(true));
        let v: String = key.get_value(VALUE_NAME).unwrap();
        assert_eq!(v, r#""C:\Program Files\NomNom\nomnom.exe" --background"#);

        set(false, exe, &k.0).unwrap();
        assert_eq!(status(exe, &k.0), None);
        assert_eq!(key.get_value::<String, _>("Other").unwrap(), "other.exe");
        // Turning it off again is fine.
        set(false, exe, &k.0).unwrap();
    }

    #[test]
    fn an_entry_for_a_different_install_is_not_reported_as_ours() {
        let k = TestKey::new("other");
        set(true, Path::new(r"C:\Old\nomnom.exe"), &k.0).unwrap();
        assert_eq!(status(Path::new(r"C:\New\nomnom.exe"), &k.0), None);
    }
}
