//! The launcher's pure parts: reading a Linux `.desktop` entry and deciding which shortcuts from the
//! Start menu are worth listing. Scanning the disk happens in the app.

/// A program the launcher can start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub name: String,
    /// The command line, with the `%f`-style placeholders removed.
    pub exec: String,
}

/// Reads the `[Desktop Entry]` of a `.desktop` file. `None` for entries that are hidden, are not
/// programs or have no name or command.
pub fn parse_desktop_entry(text: &str) -> Option<DesktopEntry> {
    let mut in_entry = false;
    let (mut name, mut exec) = (None, None);
    let (mut kind_ok, mut hidden) = (true, false);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k.trim() {
            "Name" => name = Some(v.trim().to_string()),
            "Exec" => exec = Some(v.trim().to_string()),
            "Type" => kind_ok = v.trim() == "Application",
            "NoDisplay" | "Hidden" => hidden |= v.trim().eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
    if !kind_ok || hidden {
        return None;
    }
    let name = name.filter(|n| !n.is_empty())?;
    let exec = strip_field_codes(&exec?);
    (!exec.is_empty()).then_some(DesktopEntry { name, exec })
}

/// Removes `%f`, `%U` and the other placeholders a launcher fills in; `%%` becomes `%`.
pub fn strip_field_codes(exec: &str) -> String {
    let mut out = String::new();
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            if chars.next() == Some('%') {
                out.push('%');
            }
        } else {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Start-menu entries that are not something a person wants to launch.
pub fn is_noise(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("uninstall")
        || n.starts_with("uninst")
        || n.ends_with(" help")
        || n.ends_with(" documentation")
        || n.ends_with(" readme")
        || n == "readme"
        || n.contains("release notes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_normal_entry_is_read() {
        let e = parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox %u\nIcon=firefox\n\n[Desktop Action new]\nName=New window\nExec=firefox --new-window\n",
        )
        .unwrap();
        assert_eq!(e.name, "Firefox");
        assert_eq!(e.exec, "firefox");
    }

    #[test]
    fn hidden_and_non_program_entries_are_skipped() {
        assert!(parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n"
        )
        .is_none());
        assert!(parse_desktop_entry("[Desktop Entry]\nType=Link\nName=X\nExec=x\n").is_none());
        assert!(parse_desktop_entry("[Desktop Entry]\nType=Application\nName=X\n").is_none());
        assert!(parse_desktop_entry("[Other]\nName=X\nExec=x\n").is_none());
    }

    #[test]
    fn field_codes_go_and_percent_signs_stay() {
        assert_eq!(
            strip_field_codes("app --file %F --x %%done"),
            "app --file --x %done"
        );
        assert_eq!(strip_field_codes("app"), "app");
    }

    #[test]
    fn the_noise_filter_keeps_real_programs() {
        assert!(is_noise("Uninstall Foo"));
        assert!(is_noise("Foo Documentation"));
        assert!(!is_noise("Visual Studio Code"));
        assert!(!is_noise("Helpful Tool"));
    }
}
