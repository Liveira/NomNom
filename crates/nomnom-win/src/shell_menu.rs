//! The Explorer right-click menu: what it offers, the classic-menu fallback in the registry, and
//! the command line it runs.
//!
//! The modern Windows 11 menu is built by `nomnom-shell` (a COM DLL) from the same `TABLE`, and the
//! classic menu entries here come from it too, so the two can never disagree.

use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

/// Where per-user file associations live. Tests pass another root, never this one.
pub const CLASSES_ROOT: &str = r"Software\Classes";
/// Name of the registry key the classic menu lives under.
pub const MENU_NAME: &str = "NOMNOM";
/// What people read in the menu.
pub const MENU_TITLE: &str = "NomNom";
/// The COM class of the modern menu (`nomnom-shell`). The same value is in the sparse package.
pub const SHELL_CLSID: &str = "7C2D5B1E-3A64-4F0B-9A6E-5D1C8B7F2E40";
const OWNED_MARKER: &str = "NomNomManaged";

pub struct Entry {
    pub id: &'static str,
    pub title: &'static str,
    /// Handed to `nomnom.exe --shell-action`.
    pub action: &'static str,
    /// The entry shows only when every selected file has one of these extensions.
    pub exts: &'static [&'static str],
    /// Only offered when more than one file is selected.
    pub needs_several: bool,
    /// Left out of the classic menu, which can't tell how many files are selected.
    pub modern_only: bool,
}

const IMAGES: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "ico",
];
// `.ts` is deliberately missing: it is also TypeScript.
const VIDEO: &[&str] = &[
    "mp4", "m4v", "mkv", "mov", "webm", "avi", "wmv", "flv", "mpg", "mpeg", "3gp",
];
const AUDIO: &[&str] = &[
    "mp3", "m4a", "aac", "wav", "flac", "ogg", "oga", "opus", "wma", "aiff", "aif",
];
const TEXT: &[&str] = &["md", "markdown", "txt"];
const MARKDOWN: &[&str] = &["md", "markdown"];
const OFFICE: &[&str] = &[
    "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "rtf",
];

/// Every extension that gets a NOMNOM menu.
pub fn all_exts() -> Vec<&'static str> {
    let mut v: Vec<&str> = [IMAGES, VIDEO, AUDIO, TEXT, OFFICE]
        .iter()
        .flat_map(|l| l.iter().copied())
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

const IMG_NOT_WEBP: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "ico"];
const IMG_NOT_PNG: &[&str] = &["jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "ico"];
const IMG_NOT_JPG: &[&str] = &["png", "webp", "gif", "bmp", "tif", "tiff", "ico"];
const VID_NOT_MP4: &[&str] = &[
    "m4v", "mkv", "mov", "webm", "avi", "wmv", "flv", "mpg", "mpeg", "3gp",
];
const VID_NOT_WEBM: &[&str] = &[
    "mp4", "m4v", "mkv", "mov", "avi", "wmv", "flv", "mpg", "mpeg", "3gp",
];
const AUD_NOT_MP3: &[&str] = &[
    "m4a", "aac", "wav", "flac", "ogg", "oga", "opus", "wma", "aiff", "aif",
];
const AUD_NOT_FLAC: &[&str] = &[
    "mp3", "m4a", "aac", "wav", "ogg", "oga", "opus", "wma", "aiff", "aif",
];
const AUD_NOT_WAV: &[&str] = &[
    "mp3", "m4a", "aac", "flac", "ogg", "oga", "opus", "wma", "aiff", "aif",
];

macro_rules! entry {
    ($id:expr, $title:expr, $action:expr, $exts:expr) => {
        Entry {
            id: $id,
            title: $title,
            action: $action,
            exts: $exts,
            needs_several: false,
            modern_only: false,
        }
    };
}

pub static TABLE: &[Entry] = &[
    entry!("to-webp", "Convert to WebP", "convert:webp", IMG_NOT_WEBP),
    entry!("to-png", "Convert to PNG", "convert:png", IMG_NOT_PNG),
    entry!("to-jpg", "Convert to JPG", "convert:jpg", IMG_NOT_JPG),
    entry!("img-pdf", "Convert to PDF", "convert:pdf", IMAGES),
    Entry {
        id: "img-combine",
        title: "Combine into one PDF",
        action: "preset:photos-pdf",
        exts: IMAGES,
        needs_several: true,
        modern_only: true,
    },
    entry!("to-mp4", "Convert to MP4", "convert:mp4", VID_NOT_MP4),
    entry!("to-webm", "Convert to WebM", "convert:webm", VID_NOT_WEBM),
    entry!("to-gif", "Convert to GIF", "convert:gif", VIDEO),
    entry!("vid-mp3", "Extract sound as MP3", "convert:mp3", VIDEO),
    entry!("aud-mp3", "Convert to MP3", "convert:mp3", AUD_NOT_MP3),
    entry!("aud-flac", "Convert to FLAC", "convert:flac", AUD_NOT_FLAC),
    entry!("aud-wav", "Convert to WAV", "convert:wav", AUD_NOT_WAV),
    entry!("txt-pdf", "Convert to PDF", "convert:pdf", TEXT),
    entry!("md-html", "Convert to HTML", "convert:html", MARKDOWN),
    entry!("doc-pdf", "Convert to PDF", "convert:pdf", OFFICE),
    entry!("more", "More options…", "choose", &[]),
];

/// The "More options" entry works for every supported extension.
pub fn entry_matches(e: &Entry, exts: &[String], count: usize) -> bool {
    if exts.is_empty() {
        return false;
    }
    if e.needs_several && count < 2 {
        return false;
    }
    let all = all_exts();
    exts.iter().all(|x| {
        let x = x.to_ascii_lowercase();
        if e.exts.is_empty() {
            all.contains(&x.as_str())
        } else {
            e.exts.contains(&x.as_str())
        }
    })
}

/// The entries to show for a selection, in table order.
pub fn entries_for(exts: &[String], count: usize) -> Vec<&'static Entry> {
    TABLE
        .iter()
        .filter(|e| entry_matches(e, exts, count))
        .collect()
}

// ---- the command line -------------------------------------------------------------------------

/// Quotes one argument the way `CommandLineToArgvW` reads it back.
pub fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"', '\n']) {
        return arg.to_string();
    }
    let mut out = String::from("\"");
    let mut backslashes = 0;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(backslashes * 2));
    out.push('"');
    out
}

/// More than this many files, or a longer command line, goes through a list file instead.
pub const MAX_INLINE_FILES: usize = 16;
pub const MAX_INLINE_CHARS: usize = 8000;

pub enum Selection {
    Inline(String),
    /// The command line, and the text to write into the list file it names.
    ListFile {
        command_line: String,
        list: String,
    },
}

/// Builds `"<exe>" --shell-action <action> --files ...`, or the list-file form for big selections.
/// `list_path` is where the caller will write the list.
pub fn command_line(exe: &Path, action: &str, files: &[PathBuf], list_path: &Path) -> Selection {
    let head = format!(
        "{} --shell-action {}",
        quote_arg(&exe.to_string_lossy()),
        quote_arg(action)
    );
    let inline: String = files
        .iter()
        .map(|f| format!(" {}", quote_arg(&f.to_string_lossy())))
        .collect();
    if files.len() <= MAX_INLINE_FILES && head.len() + 9 + inline.len() <= MAX_INLINE_CHARS {
        return Selection::Inline(format!("{head} --files{inline}"));
    }
    let list = files
        .iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    Selection::ListFile {
        command_line: format!(
            "{head} --files-from {}",
            quote_arg(&list_path.to_string_lossy())
        ),
        list,
    }
}

// ---- the classic menu (registry) ----------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum MenuError {
    #[error("NomNom wasn't found at {0}.")]
    ExeMissing(PathBuf),
    #[error("Couldn't change the right-click menu: {0}")]
    Registry(String),
}

fn reg(e: std::io::Error) -> MenuError {
    MenuError::Registry(e.to_string())
}

fn top_key(root: &str, ext: &str) -> String {
    format!(r"{root}\SystemFileAssociations\.{ext}\shell\{MENU_NAME}")
}

/// Adds the NOMNOM submenu under each supported extension. Returns the keys it made.
/// The classic menu launches one process per selected file; NOMNOM gathers them into one batch.
pub fn install_classic(
    exe: &Path,
    root: &str,
    owned_file: &Path,
) -> Result<Vec<String>, MenuError> {
    if !exe.is_file() {
        return Err(MenuError::ExeMissing(exe.to_path_buf()));
    }
    // Start clean so a changed table or a moved exe never leaves stale entries.
    uninstall_classic(root, owned_file)?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let mut owned = Vec::new();
    for ext in all_exts() {
        let single = vec![ext.to_string()];
        let entries: Vec<&Entry> = entries_for(&single, 1)
            .into_iter()
            .filter(|e| !e.modern_only)
            .collect();
        if entries.is_empty() {
            continue;
        }
        let top = top_key(root, ext);
        let (key, _) = hkcu.create_subkey(&top).map_err(reg)?;
        key.set_value("MUIVerb", &MENU_TITLE).map_err(reg)?;
        key.set_value("SubCommands", &"").map_err(reg)?;
        key.set_value(OWNED_MARKER, &"1").map_err(reg)?;
        owned.push(top.clone());
        for (n, e) in entries.iter().enumerate() {
            // Numbered so the order in the table is the order on screen.
            let (sub, _) = hkcu
                .create_subkey(format!(r"{top}\shell\{:02}-{}", n, e.id))
                .map_err(reg)?;
            sub.set_value("MUIVerb", &e.title).map_err(reg)?;
            sub.set_value("MultiSelectModel", &"Document")
                .map_err(reg)?;
            let (cmd, _) = sub.create_subkey("command").map_err(reg)?;
            let line = format!(
                "{} --shell-action {} --files \"%1\"",
                quote_arg(&exe.to_string_lossy()),
                e.action
            );
            cmd.set_value("", &line).map_err(reg)?;
        }
    }
    let record = serde_json::json!({ "keys": owned });
    if let Some(dir) = owned_file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| MenuError::Registry(e.to_string()))?;
    }
    std::fs::write(owned_file, record.to_string())
        .map_err(|e| MenuError::Registry(e.to_string()))?;
    Ok(owned)
}

/// Removes exactly the keys NOMNOM recorded, and only if they still carry NOMNOM's marker.
pub fn uninstall_classic(root: &str, owned_file: &Path) -> Result<usize, MenuError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let keys: Vec<String> = std::fs::read_to_string(owned_file)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| {
            v.get("keys").and_then(|k| k.as_array()).map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
        })
        .unwrap_or_default();
    let mut removed = 0;
    for k in keys {
        // Never trust the record blindly: it must still be one of our keys under the given root.
        if !k.starts_with(&format!(r"{root}\SystemFileAssociations\"))
            || !k.ends_with(&format!(r"\shell\{MENU_NAME}"))
        {
            continue;
        }
        let ours = hkcu
            .open_subkey_with_flags(&k, KEY_READ)
            .ok()
            .and_then(|key| key.get_value::<String, _>(OWNED_MARKER).ok())
            .is_some();
        if ours {
            hkcu.delete_subkey_all(&k).map_err(reg)?;
            removed += 1;
            // Tidy the empty parents we created (these refuse to go if anything else is inside).
            if let Some((shell, _)) = k.rsplit_once('\\') {
                let _ = hkcu.delete_subkey(shell);
                if let Some((ext_key, _)) = shell.rsplit_once('\\') {
                    let _ = hkcu.delete_subkey(ext_key);
                }
            }
        }
    }
    match std::fs::remove_file(owned_file) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(MenuError::Registry(e.to_string())),
    }
    Ok(removed)
}

/// True when the classic entries are present for at least one extension.
pub fn classic_installed(root: &str) -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(top_key(root, "png"), KEY_READ)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRoot(String);
    impl TestRoot {
        fn new(tag: &str) -> Self {
            TestRoot(format!(
                r"Software\NOMNOM-Test-menu-{tag}-{}\Classes",
                std::process::id()
            ))
        }
    }
    impl Drop for TestRoot {
        fn drop(&mut self) {
            // The whole key belongs to this test (see autostart tests for why).
            let own = self.0.trim_end_matches(r"\Classes");
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(own);
        }
    }

    fn exts(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_png_selection_gets_picture_conversions_and_more_options() {
        let ids: Vec<_> = entries_for(&exts(&["png"]), 1)
            .iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, ["to-webp", "to-jpg", "img-pdf", "more"]);
    }

    #[test]
    fn combining_into_one_pdf_needs_several_pictures() {
        let one: Vec<_> = entries_for(&exts(&["png"]), 1)
            .iter()
            .map(|e| e.id)
            .collect();
        let many: Vec<_> = entries_for(&exts(&["png", "JPG"]), 2)
            .iter()
            .map(|e| e.id)
            .collect();
        assert!(!one.contains(&"img-combine"));
        assert!(many.contains(&"img-combine"));
    }

    #[test]
    fn mixed_kinds_get_only_more_options() {
        let ids: Vec<_> = entries_for(&exts(&["png", "mp4"]), 2)
            .iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, ["more"]);
    }

    #[test]
    fn unsupported_files_and_empty_selections_get_nothing() {
        assert!(entries_for(&exts(&["exe"]), 1).is_empty());
        assert!(
            entries_for(&exts(&["ts"]), 1).is_empty(),
            ".ts is TypeScript"
        );
        assert!(entries_for(&[], 0).is_empty());
        assert!(entries_for(&exts(&["png", "exe"]), 2).is_empty());
    }

    #[test]
    fn a_format_is_never_offered_for_itself() {
        let ids =
            |e: &str| -> Vec<&str> { entries_for(&exts(&[e]), 1).iter().map(|x| x.id).collect() };
        assert!(!ids("webp").contains(&"to-webp"));
        assert!(!ids("mp3").contains(&"aud-mp3"));
        assert!(!ids("mp4").contains(&"to-mp4"));
        assert!(ids("md").contains(&"md-html"));
        assert!(!ids("txt").contains(&"md-html"));
    }

    #[test]
    fn quoting_round_trips_through_the_windows_rules() {
        assert_eq!(quote_arg("plain"), "plain");
        assert_eq!(quote_arg(r"C:\a b\c.png"), r#""C:\a b\c.png""#);
        assert_eq!(quote_arg(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(quote_arg(r"C:\dir with space\"), r#""C:\dir with space\\""#);
        assert_eq!(quote_arg(""), "\"\"");
    }

    #[test]
    fn a_few_files_go_on_the_command_line() {
        let files = vec![PathBuf::from(r"C:\a b\1.png"), PathBuf::from(r"C:\2.png")];
        match command_line(
            Path::new(r"C:\NomNom\nomnom.exe"),
            "convert:webp",
            &files,
            Path::new("x"),
        ) {
            Selection::Inline(l) => assert_eq!(
                l,
                r#"C:\NomNom\nomnom.exe --shell-action convert:webp --files "C:\a b\1.png" C:\2.png"#
            ),
            Selection::ListFile { .. } => panic!("should be inline"),
        }
    }

    #[test]
    fn many_files_or_a_long_line_use_a_list_file() {
        let many: Vec<PathBuf> = (0..40)
            .map(|i| PathBuf::from(format!(r"C:\p\{i}.png")))
            .collect();
        match command_line(
            Path::new(r"C:\n.exe"),
            "convert:jpg",
            &many,
            Path::new(r"C:\t\sel.txt"),
        ) {
            Selection::ListFile { command_line, list } => {
                assert!(command_line.ends_with(r"--files-from C:\t\sel.txt"));
                assert_eq!(list.lines().count(), 40);
            }
            Selection::Inline(_) => panic!("should use a list file"),
        }
        let long = vec![PathBuf::from(format!(r"C:\{}\a.png", "x".repeat(9000)))];
        assert!(matches!(
            command_line(Path::new(r"C:\n.exe"), "choose", &long, Path::new("t")),
            Selection::ListFile { .. }
        ));
    }

    #[test]
    fn classic_menu_install_and_uninstall_leave_nothing_behind() {
        let root = TestRoot::new("classic");
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("nomnom.exe");
        std::fs::write(&exe, b"x").unwrap();
        let owned = dir.path().join("registry-owned.json");

        assert!(!classic_installed(&root.0));
        let keys = install_classic(&exe, &root.0, &owned).unwrap();
        assert!(keys.len() > 30, "{}", keys.len());
        assert!(classic_installed(&root.0));

        // The command carries the action, quotes the exe and receives one file per launch.
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let cmd: String = hkcu
            .open_subkey(format!(
                r"{}\shell\00-to-webp\command",
                top_key(&root.0, "png")
            ))
            .unwrap()
            .get_value("")
            .unwrap();
        assert!(
            cmd.ends_with(r#"--shell-action convert:webp --files "%1""#),
            "{cmd}"
        );
        // modern-only entries never reach the classic menu
        assert!(hkcu
            .open_subkey(format!(r"{}\shell\04-img-combine", top_key(&root.0, "png")))
            .is_err());

        // Installing again is safe and doesn't duplicate anything.
        let again = install_classic(&exe, &root.0, &owned).unwrap();
        assert_eq!(again.len(), keys.len());

        let removed = uninstall_classic(&root.0, &owned).unwrap();
        assert_eq!(removed, keys.len());
        assert!(!classic_installed(&root.0));
        assert!(!owned.exists());
        // The test root itself holds nothing from us any more.
        let leftovers = hkcu
            .open_subkey(format!(r"{}\SystemFileAssociations", root.0))
            .map(|k| k.enum_keys().count())
            .unwrap_or(0);
        assert_eq!(leftovers, 0, "no empty extension keys are left behind");
    }

    #[test]
    fn uninstall_only_removes_keys_that_carry_our_marker() {
        let root = TestRoot::new("marker");
        let dir = tempfile::tempdir().unwrap();
        let owned = dir.path().join("registry-owned.json");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        // Someone else's key at the same place, and a record that (wrongly) lists it.
        let theirs = top_key(&root.0, "png");
        let (key, _) = hkcu.create_subkey(&theirs).unwrap();
        key.set_value("MUIVerb", &"Not ours").unwrap();
        std::fs::write(
            &owned,
            serde_json::json!({ "keys": [theirs, r"Software\Classes\txtfile"] }).to_string(),
        )
        .unwrap();
        assert_eq!(uninstall_classic(&root.0, &owned).unwrap(), 0);
        assert!(
            hkcu.open_subkey(top_key(&root.0, "png")).is_ok(),
            "a key without our marker must stay"
        );
    }

    #[test]
    fn a_missing_exe_is_refused_before_touching_the_registry() {
        let root = TestRoot::new("noexe");
        let dir = tempfile::tempdir().unwrap();
        let err = install_classic(
            &dir.path().join("nope.exe"),
            &root.0,
            &dir.path().join("o.json"),
        );
        assert!(matches!(err, Err(MenuError::ExeMissing(_))));
        assert!(!classic_installed(&root.0));
    }

    #[test]
    fn every_conversion_in_the_table_is_one_the_converter_can_do() {
        use nomnom_converter::model::{valid_for, Target};
        for e in TABLE {
            if e.action == "choose" {
                continue;
            }
            for ext in e.exts {
                let file = PathBuf::from(format!("C:\\x\\a.{ext}"));
                if let Some(t) = e.action.strip_prefix("convert:") {
                    let target: Target =
                        serde_json::from_value(serde_json::Value::String(t.into()))
                            .unwrap_or_else(|_| panic!("{} names an unknown format", e.id));
                    assert!(
                        valid_for(&file, target),
                        "{}: .{ext} can't become {t}",
                        e.id
                    );
                } else if let Some(id) = e.action.strip_prefix("preset:") {
                    let p = nomnom_converter::presets()
                        .into_iter()
                        .find(|p| p.id == id)
                        .unwrap_or_else(|| panic!("{} names an unknown preset", e.id));
                    assert!(valid_for(&file, p.target), "{}: .{ext}", e.id);
                } else {
                    panic!("{} has an action the app doesn't understand", e.id);
                }
            }
        }
    }

    #[test]
    fn every_converter_format_that_matters_is_reachable_from_the_menu() {
        // Each kind of file has at least one conversion beyond "More options".
        for ext in ["png", "mp4", "mp3", "md", "txt", "docx"] {
            let ids = entries_for(&exts(&[ext]), 1);
            assert!(ids.len() >= 2, ".{ext} only has {} entries", ids.len());
        }
    }
}
