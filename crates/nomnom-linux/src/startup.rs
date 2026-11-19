//! Programs that start when you log in on Linux: the "autostart" entries of the desktop (`.desktop`
//! files in `~/.config/autostart` for you, `/etc/xdg/autostart` for everyone). Turning one off adds
//! `Hidden=true` to your own copy, which every desktop understands; turning it on removes that
//! line. Nothing is deleted, and the system's own files are never touched.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The file name, such as `discord.desktop`.
    pub file: String,
    pub name: String,
    pub command: String,
    pub enabled: bool,
    /// From the user's own folder (otherwise from the system's).
    pub own: bool,
}

pub fn user_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|h| h.join(".config"))
        })?;
    Some(base.join("autostart"))
}

pub fn system_dir() -> PathBuf {
    PathBuf::from("/etc/xdg/autostart")
}

/// The value of `key` in the `[Desktop Entry]` group (the first one wins; a translated
/// `Name[pt]` is ignored in favour of plain `Name`).
fn value(text: &str, key: &str) -> Option<String> {
    let mut in_entry = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_entry = l == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn flag(text: &str, key: &str) -> bool {
    value(text, key).is_some_and(|v| v.eq_ignore_ascii_case("true"))
}

fn read_entry(path: &Path, own: bool) -> Option<Entry> {
    let text = std::fs::read_to_string(path).ok()?;
    let file = path.file_name()?.to_string_lossy().into_owned();
    if !file.ends_with(".desktop") {
        return None;
    }
    // A GNOME switch as well as the standard one.
    let off = flag(&text, "Hidden")
        || value(&text, "X-GNOME-Autostart-enabled")
            .is_some_and(|v| v.eq_ignore_ascii_case("false"));
    Some(Entry {
        name: value(&text, "Name").unwrap_or_else(|| file.trim_end_matches(".desktop").to_string()),
        command: value(&text, "Exec").unwrap_or_default(),
        file,
        enabled: !off,
        own,
    })
}

fn entries_in(dir: &Path, own: bool) -> Vec<Entry> {
    let mut out: Vec<Entry> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| read_entry(&e.path(), own))
        .collect();
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}

/// What starts at login. A file in your own folder takes the place of a system file of the same
/// name (that is how a system entry gets turned off).
pub fn list_in(user: &Path, system: &Path) -> Vec<Entry> {
    let mine = entries_in(user, true);
    let mut out = mine.clone();
    for e in entries_in(system, false) {
        if !mine.iter().any(|m| m.file == e.file) {
            out.push(e);
        }
    }
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}

pub fn list() -> Vec<Entry> {
    match user_dir() {
        Some(u) => list_in(&u, &system_dir()),
        None => entries_in(&system_dir(), false),
    }
}

fn safe_file(file: &str) -> bool {
    file.ends_with(".desktop") && !file.contains(['/', '\\']) && !file.starts_with('.')
}

/// Turns one entry off or on, in your own folder.
pub fn set_enabled_in(user: &Path, system: &Path, file: &str, enabled: bool) -> Result<(), String> {
    if !safe_file(file) {
        return Err("That isn't a start-up entry.".into());
    }
    let own = user.join(file);
    let text = match std::fs::read_to_string(&own) {
        Ok(t) => t,
        // A system entry: start from a copy of it.
        Err(_) => std::fs::read_to_string(system.join(file))
            .map_err(|_| "That entry is gone.".to_string())?,
    };
    let mut lines: Vec<String> = Vec::new();
    let mut in_entry = false;
    let mut placed = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            if in_entry && !placed && !enabled {
                lines.push("Hidden=true".into());
                placed = true;
            }
            in_entry = t == "[Desktop Entry]";
            lines.push(line.to_string());
            continue;
        }
        if in_entry
            && t.split_once('=')
                .is_some_and(|(k, _)| matches!(k.trim(), "Hidden" | "X-GNOME-Autostart-enabled"))
        {
            continue; // replaced below
        }
        lines.push(line.to_string());
        if in_entry && !placed && !enabled && t == "[Desktop Entry]" {
            // handled when the group ends
        }
    }
    if !placed && !enabled {
        if !in_entry {
            return Err("That start-up entry can't be read.".into());
        }
        lines.push("Hidden=true".into());
    }
    std::fs::create_dir_all(user).map_err(|e| e.to_string())?;
    let mut body = lines.join("\n");
    body.push('\n');
    std::fs::write(&own, body).map_err(|e| e.to_string())
}

pub fn set_enabled(file: &str, enabled: bool) -> Result<(), String> {
    let user = user_dir().ok_or("The home folder wasn't found.")?;
    set_enabled_in(&user, &system_dir(), file, enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: &str = "[Desktop Entry]\nType=Application\nName=Chat App\nName[pt]=App de Conversa\nExec=chat --minimized\n";

    fn dirs() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let user = t.path().join("user");
        let system = t.path().join("system");
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&system).unwrap();
        (t, user, system)
    }

    #[test]
    fn entries_are_read_with_their_plain_name_and_command() {
        let (_t, user, system) = dirs();
        std::fs::write(system.join("chat.desktop"), APP).unwrap();
        std::fs::write(
            user.join("mine.desktop"),
            "[Desktop Entry]\nName=Mine\nExec=mine\nHidden=true\n",
        )
        .unwrap();
        std::fs::write(user.join("notes.txt"), "not an entry").unwrap();
        let all = list_in(&user, &system);
        assert_eq!(all.len(), 2);
        let chat = all.iter().find(|e| e.file == "chat.desktop").unwrap();
        assert_eq!(
            (
                chat.name.as_str(),
                chat.command.as_str(),
                chat.enabled,
                chat.own
            ),
            ("Chat App", "chat --minimized", true, false)
        );
        let mine = all.iter().find(|e| e.file == "mine.desktop").unwrap();
        assert!(!mine.enabled && mine.own);
    }

    #[test]
    fn a_system_entry_is_turned_off_in_a_copy_and_the_original_is_untouched() {
        let (_t, user, system) = dirs();
        std::fs::write(system.join("chat.desktop"), APP).unwrap();
        set_enabled_in(&user, &system, "chat.desktop", false).unwrap();
        assert_eq!(
            std::fs::read_to_string(system.join("chat.desktop")).unwrap(),
            APP,
            "the system file is not changed"
        );
        let all = list_in(&user, &system);
        assert_eq!(all.len(), 1, "the copy takes the place of the system file");
        assert!(!all[0].enabled && all[0].own);
        set_enabled_in(&user, &system, "chat.desktop", true).unwrap();
        let again = list_in(&user, &system);
        assert!(again[0].enabled);
        assert!(!std::fs::read_to_string(user.join("chat.desktop"))
            .unwrap()
            .contains("Hidden"));
    }

    #[test]
    fn nothing_outside_the_folders_can_be_reached_by_a_name() {
        let (_t, user, system) = dirs();
        assert!(set_enabled_in(&user, &system, "../evil.desktop", false).is_err());
        assert!(set_enabled_in(&user, &system, "/etc/passwd", false).is_err());
        assert!(set_enabled_in(&user, &system, "x.txt", false).is_err());
        assert!(set_enabled_in(&user, &system, "missing.desktop", false).is_err());
    }

    #[test]
    fn turning_off_twice_keeps_one_hidden_line() {
        let (_t, user, system) = dirs();
        std::fs::write(user.join("a.desktop"), APP).unwrap();
        set_enabled_in(&user, &system, "a.desktop", false).unwrap();
        set_enabled_in(&user, &system, "a.desktop", false).unwrap();
        let t = std::fs::read_to_string(user.join("a.desktop")).unwrap();
        assert_eq!(t.matches("Hidden=true").count(), 1);
        assert!(t.contains("Exec=chat --minimized"));
    }
}
