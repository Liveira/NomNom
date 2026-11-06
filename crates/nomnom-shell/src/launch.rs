//! Reading `integration.json` and starting NomNom. No COM in here, so it can be tested directly.

use nomnom_win::shell_menu::{command_line, quote_arg, Selection};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CACHE_FOR: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Integration {
    pub exe: Option<PathBuf>,
    pub shell_menu: bool,
}

/// `%LOCALAPPDATA%\NOMNOM\integration.json`, the only file this DLL reads.
pub fn integration_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default();
    base.join("NOMNOM").join("integration.json")
}

pub fn read_integration(path: &Path) -> Integration {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Integration::default();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text.trim_start_matches('\u{feff}'))
    else {
        return Integration::default();
    };
    Integration {
        exe: v.get("exe").and_then(|e| e.as_str()).map(PathBuf::from),
        shell_menu: v
            .get("shellMenu")
            .and_then(|b| b.as_bool())
            .unwrap_or(false),
    }
}

static CACHE: Mutex<Option<(Instant, Integration)>> = Mutex::new(None);

/// Forgets what was read, so a test can change the file and see the effect at once.
#[doc(hidden)]
pub fn reset_cache() {
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn current() -> Integration {
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, i)) = guard.as_ref() {
        if at.elapsed() < CACHE_FOR {
            return i.clone();
        }
    }
    let fresh = read_integration(&integration_path());
    *guard = Some((Instant::now(), fresh.clone()));
    fresh
}

/// The menu is shown only when the app has switched it on and knows where NomNom is.
pub fn menu_enabled() -> bool {
    let i = current();
    i.shell_menu && i.exe.as_deref().is_some_and(is_nomnom)
}

/// Only a file literally named nomnom.exe is ever started.
pub fn is_nomnom(exe: &Path) -> bool {
    exe.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("nomnom.exe"))
        && exe.is_file()
}

/// Starts `nomnom.exe --shell-action <action> --files ...`.
pub fn start(action: &str, files: &[PathBuf]) -> std::io::Result<()> {
    let exe = current()
        .exe
        .filter(|e| is_nomnom(e))
        .ok_or_else(|| std::io::Error::other("NomNom isn't set up for the menu"))?;
    start_with(&exe, action, files, &std::env::temp_dir().join("nomnom"))
}

/// Split out so a test can start something harmless instead of NomNom.
pub fn start_with(
    exe: &Path,
    action: &str,
    files: &[PathBuf],
    list_dir: &Path,
) -> std::io::Result<()> {
    let list_path = list_dir.join(format!("sel-{}-{}.txt", std::process::id(), unique()));
    let line = match command_line(exe, action, files, &list_path) {
        Selection::Inline(l) => l,
        Selection::ListFile { command_line, list } => {
            std::fs::create_dir_all(list_dir)?;
            std::fs::write(&list_path, list)?;
            command_line
        }
    };
    // The line starts with the quoted program; Command adds it again, so hand over only the rest.
    let tail = line[quote_arg(&exe.to_string_lossy()).len()..]
        .trim_start()
        .to_string();
    std::process::Command::new(exe)
        .raw_arg(tail)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
}

fn unique() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_file_is_read_defensively() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("integration.json");
        assert_eq!(read_integration(&f), Integration::default());
        std::fs::write(
            &f,
            "\u{feff}{\"exe\":\"C:\\\\N\\\\nomnom.exe\",\"shellMenu\":true,\"x\":1}",
        )
        .unwrap();
        assert_eq!(
            read_integration(&f),
            Integration {
                exe: Some(PathBuf::from(r"C:\N\nomnom.exe")),
                shell_menu: true
            }
        );
        std::fs::write(&f, "not json").unwrap();
        assert_eq!(read_integration(&f), Integration::default());
        std::fs::write(&f, r#"{"exe":"C:\\a.exe"}"#).unwrap();
        assert!(
            !read_integration(&f).shell_menu,
            "off unless the app says on"
        );
    }

    #[test]
    fn only_a_real_nomnom_exe_counts() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("nomnom.exe");
        let other = dir.path().join("cmd.exe");
        std::fs::write(&good, b"x").unwrap();
        std::fs::write(&other, b"x").unwrap();
        assert!(is_nomnom(&good));
        assert!(!is_nomnom(&other));
        assert!(!is_nomnom(&dir.path().join("nomnom.exe.missing")));
    }
}
