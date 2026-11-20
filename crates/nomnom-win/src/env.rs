//! The PATH of the current user (and, to read only, of the whole machine), straight from the registry.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};
use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, REG_EXPAND_SZ,
};
use winreg::{RegKey, RegValue};

const USER_KEY: &str = "Environment";
const MACHINE_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The user's own PATH as it is written (with `%NAME%` parts still in it). Empty when there is none.
pub fn user_path() -> Option<String> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(USER_KEY, KEY_READ)
        .ok()?;
    key.get_value::<String, _>("Path").ok()
}

pub fn machine_path() -> Option<String> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(MACHINE_KEY, KEY_READ)
        .ok()?;
    key.get_value::<String, _>("Path").ok()
}

/// Fills in `%NAME%` parts the way Windows does.
pub fn expand(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let src = wide(s);
    // SAFETY: both buffers are valid for the sizes given.
    unsafe {
        let n = ExpandEnvironmentStringsW(PCWSTR(src.as_ptr()), None);
        if n == 0 {
            return s.to_string();
        }
        let mut buf = vec![0u16; n as usize];
        let n = ExpandEnvironmentStringsW(PCWSTR(src.as_ptr()), Some(&mut buf));
        if n == 0 {
            return s.to_string();
        }
        String::from_utf16_lossy(&buf[..(n as usize).saturating_sub(1)])
    }
}

/// Writes `Path` under a key of the current user as an expandable string.
fn write_expand(subkey: &str, value: &str) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey_with_flags(subkey, KEY_SET_VALUE)
        .map_err(|e| format!("Couldn't open the setting: {e}"))?;
    let bytes: Vec<u8> = wide(value).into_iter().flat_map(u16::to_le_bytes).collect();
    key.set_raw_value(
        "Path",
        &RegValue {
            bytes,
            vtype: REG_EXPAND_SZ,
        },
    )
    .map_err(|e| format!("Couldn't save it: {e}"))
}

/// Writes the user's PATH (as a value that keeps `%NAME%` parts) and tells open programs.
pub fn set_user_path(value: &str) -> Result<(), String> {
    write_expand(USER_KEY, value)?;
    let env = wide("Environment");
    // SAFETY: the string outlives the call; a program that doesn't answer in time is skipped.
    unsafe {
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(env.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            3000,
            None,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_and_machine_path_can_be_read() {
        // There is always a machine PATH on Windows.
        assert!(machine_path().is_some_and(|p| !p.is_empty()));
        let _ = user_path();
    }

    #[test]
    fn a_written_path_keeps_its_names_and_reads_back_the_same() {
        // A scratch key, never the real Environment one.
        let sub = r"Software\NOMNOM-test-path";
        let value = r"%USERPROFILE%\bin;C:\Tools;D:\ünï";
        write_expand(sub, value).unwrap();
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(sub, KEY_READ)
            .unwrap();
        let raw = key.get_raw_value("Path").unwrap();
        assert_eq!(raw.vtype, REG_EXPAND_SZ);
        assert_eq!(key.get_value::<String, _>("Path").unwrap(), value);
        let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(sub);
    }

    #[test]
    fn names_between_percent_signs_are_filled_in() {
        assert_eq!(expand("plain"), "plain");
        assert!(expand("%SystemRoot%\\System32")
            .to_lowercase()
            .ends_with("\\system32"));
        assert!(!expand("%SystemRoot%\\System32").contains('%'));
        // A name that doesn't exist stays as it is.
        assert_eq!(
            expand("%NOMNOM_NO_SUCH_VARIABLE%\\x"),
            "%NOMNOM_NO_SUCH_VARIABLE%\\x"
        );
    }
}
