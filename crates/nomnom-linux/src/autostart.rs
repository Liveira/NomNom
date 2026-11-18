//! Start NomNom on login via XDG Autostart (`~/.config/autostart/nomnom.desktop`). Off unless the
//! user asks.

use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "nomnom.desktop";
const MANAGED_MARKER: &str = "# NomNomManaged";

pub fn default_autostart_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|h| h.join(".config"))
        })
        .map(|c| c.join("autostart"))
}

fn escape_exec(exe: &Path) -> String {
    let s = exe
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("\"{s}\" --background")
}

pub fn desktop_entry(exe: &Path) -> String {
    format!(
        "{MANAGED_MARKER}\n\
[Desktop Entry]\n\
Type=Application\n\
Version=1.0\n\
Name=NomNom\n\
Comment=Local downloads, file tools, organizer and disk analyzer\n\
Exec={}\n\
Icon=nomnom\n\
Terminal=false\n\
StartupNotify=false\n\
X-GNOME-Autostart-enabled=true\n\
X-KDE-autostart-after=panel\n",
        escape_exec(exe)
    )
}

pub fn set(enabled: bool, exe: &Path, autostart_dir: &Path) -> Result<(), String> {
    let file = autostart_dir.join(FILE_NAME);
    if enabled {
        std::fs::create_dir_all(autostart_dir).map_err(|e| e.to_string())?;
        std::fs::write(&file, desktop_entry(exe)).map_err(|e| e.to_string())
    } else {
        let Ok(existing) = std::fs::read_to_string(&file) else {
            return Ok(());
        };
        if !existing.contains(MANAGED_MARKER) && !existing.contains("Name=NomNom") {
            return Ok(());
        }
        match std::fs::remove_file(&file) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// `Some(true)` when the autostart `.desktop` file exists and points to `exe --background`.
pub fn status(exe: &Path, autostart_dir: &Path) -> Option<bool> {
    let text = std::fs::read_to_string(autostart_dir.join(FILE_NAME)).ok()?;
    let expected_exec = format!("Exec={}", escape_exec(exe));
    let mut has_exec = false;
    for line in text.lines().map(str::trim) {
        if line.eq_ignore_ascii_case("Hidden=true")
            || line.eq_ignore_ascii_case("X-GNOME-Autostart-enabled=false")
        {
            return None;
        }
        if line == expected_exec {
            has_exec = true;
        }
    }
    has_exec.then_some(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_autostart_on_and_off_manages_only_nomnom_desktop_file() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other.desktop");
        std::fs::write(&other, "[Desktop Entry]\nName=Other\n").unwrap();
        let exe = Path::new("/usr/bin/nomnom");

        assert_eq!(status(exe, dir.path()), None);
        set(true, exe, dir.path()).unwrap();
        assert_eq!(status(exe, dir.path()), Some(true));
        assert_eq!(status(Path::new("/opt/nomnom/nomnom"), dir.path()), None);

        set(false, exe, dir.path()).unwrap();
        assert_eq!(status(exe, dir.path()), None);
        assert!(other.exists());
    }
}
