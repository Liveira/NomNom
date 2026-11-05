//! The software check. A folder that looks like an application, an installer, a game or a
//! project is never opened, never entered and never changed. One suspicious file is enough, and
//! when in doubt the whole folder is skipped.

use crate::winfs;
use std::collections::HashSet;
use std::path::Path;

/// Extensions of programs, scripts, libraries, installers, settings and data files that belong to
/// software. None of these is ever moved, and any of them inside a folder marks it as software.
pub const PROGRAM_EXTS: &[&str] = &[
    "exe",
    "dll",
    "msi",
    "msix",
    "msixbundle",
    "appx",
    "appxbundle",
    "sys",
    "drv",
    "ocx",
    "cpl",
    "scr",
    "com",
    "bat",
    "cmd",
    "ps1",
    "psm1",
    "psd1",
    "vbs",
    "vbe",
    "js",
    "jse",
    "mjs",
    "cjs",
    "wsf",
    "wsh",
    "hta",
    "jar",
    "class",
    "reg",
    "inf",
    "cat",
    "lnk",
    "url",
    "apk",
    "dmg",
    "pkg",
    "deb",
    "rpm",
    "sh",
    "py",
    "pyc",
    "rb",
    "pl",
    "php",
    "lua",
    "so",
    "dylib",
    "node",
    "wasm",
    "asar",
    "iso",
    "img",
    "vhd",
    "vhdx",
    "bin",
    "dat",
    "db",
    "sqlite",
    "pak",
    "ini",
    "cfg",
    "conf",
    "json",
    "yaml",
    "yml",
    "toml",
    "xml",
    "manifest",
    "dmp",
    "log",
];

/// Archives are not assumed to be safe: they may hold an installer or a whole project.
pub const ARCHIVE_EXTS: &[&str] = &[
    "zip", "7z", "rar", "tar", "gz", "bz2", "xz", "zst", "tgz", "cab", "lz", "lzma", "z",
];

const MARKER_FILES: &[&str] = &[
    "package.json",
    "cargo.toml",
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "cmakelists.txt",
    "makefile",
    "dockerfile",
    ".gitignore",
    "autorun.inf",
    "setup.ini",
    "steam_appid.txt",
];
const MARKER_SUFFIXES: &[&str] = &[".sln", ".csproj", ".vcxproj", ".uproject", ".xcodeproj"];
const MARKER_DIRS: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "bin",
    "obj",
    "target",
    "dist",
    "build",
    ".vscode",
    ".idea",
    "_commonredist",
    "redist",
    "engine",
    "binaries",
];
const IN_PROGRESS_SUFFIXES: &[&str] = &[
    ".part",
    ".crdownload",
    ".download",
    ".partial",
    ".tmp",
    ".!ut",
    ".opdownload",
    ".aria2",
    ".ytdl",
];
const HARMLESS_HIDDEN: &[&str] = &["thumbs.db"];

pub fn ext_of(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty() && ext.len() <= 12).then(|| ext.to_ascii_lowercase())
}

pub fn is_program_ext(ext: &str) -> bool {
    PROGRAM_EXTS.contains(&ext)
}

pub fn is_archive_ext(ext: &str) -> bool {
    ARCHIVE_EXTS.contains(&ext)
}

/// A file that is still being written by a browser or download manager.
pub fn is_in_progress_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.starts_with("~$")
        || IN_PROGRESS_SUFFIXES.iter().any(|s| n.ends_with(s))
        || n.contains(".part-")
}

#[derive(Debug, PartialEq, Eq)]
pub enum DirVerdict {
    Safe,
    Unsafe(String),
}

pub struct Limits {
    pub max_entries: usize,
    pub max_depth: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_entries: 5_000,
            max_depth: 8,
        }
    }
}

/// Decides whether a whole folder (everything under it) is plain media and documents.
/// `known_exts` are the extensions the organizer is willing to sort.
pub fn judge_dir(dir: &Path, known_exts: &HashSet<String>, limits: &Limits) -> DirVerdict {
    use DirVerdict::Unsafe;
    let mut stack = vec![(dir.to_path_buf(), 0u32)];
    let mut seen = 0usize;
    while let Some((current, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            return Unsafe("It couldn't be read completely.".into());
        };
        for entry in entries {
            let Ok(entry) = entry else {
                return Unsafe("It couldn't be read completely.".into());
            };
            seen += 1;
            if seen > limits.max_entries {
                return Unsafe("It has too many items to check safely.".into());
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let Ok(info) = winfs::info(&path) else {
                return Unsafe(format!("It has an item that couldn't be read: {name}."));
            };
            if info.is_reparse() {
                return Unsafe("It contains a link to somewhere else.".into());
            }
            if info.is_cloud_or_offline() {
                return Unsafe("It contains files that live in the cloud.".into());
            }
            if info.is_hidden_or_system() && !HARMLESS_HIDDEN.contains(&name.as_str()) {
                return Unsafe("It contains hidden or system files.".into());
            }
            if info.is_dir() {
                if MARKER_DIRS.contains(&name.as_str()) {
                    return Unsafe(format!(
                        "It has a '{name}' folder, which software and projects use."
                    ));
                }
                if depth + 1 > limits.max_depth {
                    return Unsafe("It is nested too deeply to check.".into());
                }
                stack.push((path, depth + 1));
                continue;
            }
            if MARKER_FILES.contains(&name.as_str())
                || MARKER_SUFFIXES.iter().any(|s| name.ends_with(s))
            {
                return Unsafe(format!("It has '{name}', which marks a software project."));
            }
            let Some(ext) = ext_of(&name) else {
                return Unsafe(format!("It has a file without an extension: {name}."));
            };
            if is_program_ext(&ext) {
                return Unsafe(format!("It has a .{ext} file, so it looks like software."));
            }
            if is_archive_ext(&ext) {
                return Unsafe(format!(
                    "It has an archive (.{ext}) that could hold software."
                ));
            }
            if !known_exts.contains(&ext) {
                return Unsafe(format!(
                    "It has .{ext} files, which NomNom doesn't recognise."
                ));
            }
            match winfs::read_head(&path, 4) {
                Ok(h)
                    if h.starts_with(b"MZ")
                        || h.starts_with(b"\x7fELF")
                        || h.starts_with(b"#!") =>
                {
                    return Unsafe(format!(
                        "{name} looks like a program, whatever its name says."
                    ));
                }
                Ok(_) => {}
                Err(_) => return Unsafe(format!("It has a file that couldn't be opened: {name}.")),
            }
        }
    }
    DirVerdict::Safe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_read_safely() {
        assert_eq!(ext_of("movie.MP4").as_deref(), Some("mp4"));
        assert_eq!(ext_of("archive.tar.gz").as_deref(), Some("gz"));
        assert_eq!(ext_of(".gitignore"), None);
        assert_eq!(ext_of("noext"), None);
        assert_eq!(ext_of("trailing."), None);
    }

    #[test]
    fn unfinished_downloads_are_recognised() {
        for n in [
            "a.mp4.part",
            "b.CRDOWNLOAD",
            "c.download",
            "d.tmp",
            "e.!ut",
            "f.opdownload",
            "~$report.docx",
            "g.mp4.part-Frag3",
            "h.ytdl",
        ] {
            assert!(is_in_progress_name(n), "{n}");
        }
        assert!(!is_in_progress_name("holiday.mp4"));
    }

    #[test]
    fn software_extensions_are_never_treated_as_media() {
        for e in [
            "exe", "dll", "msi", "bat", "ps1", "vbs", "js", "jar", "sys", "reg",
        ] {
            assert!(is_program_ext(e), "{e}");
        }
        for e in ["mp4", "png", "pdf", "mp3", "txt"] {
            assert!(!is_program_ext(e), "{e}");
        }
        assert!(is_archive_ext("zip") && is_archive_ext("7z") && is_archive_ext("rar"));
    }
}
