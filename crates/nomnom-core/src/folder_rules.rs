//! Folder rules: "when a file like this shows up in this folder, do these things to it". This file
//! is the part that decides: which files a rule covers, whether a rule makes sense, and what a
//! file name means. Moving and converting happen in the app, and nothing here deletes anything.

use serde::{Deserialize, Serialize};

pub const MAX_RULES: usize = 40;
pub const MAX_STEPS: usize = 6;

/// One thing done to the file, in order. The file that comes out of a step goes into the next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Step {
    /// Move it to this folder. A file already there with the same name is never replaced.
    Move { folder: String },
    /// Put a copy in this folder and carry on with the original.
    Copy { folder: String },
    /// Change the name (not the extension). Patterns such as `{date}_{name}`.
    Rename { template: String },
    /// Make a picture in another format next to it, and carry on with the new one.
    Convert {
        /// "webp", "jpg" or "png".
        to: String,
        /// Where the original goes afterwards. Empty keeps it where it is.
        originals: String,
    },
    /// Run one of "My actions", with `{file}` filled in.
    Run { action: String },
    /// Say what happened (one notice for a batch of files).
    Notify,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// The folder that is watched.
    pub folder: String,
    /// Also look in the folders inside it.
    pub subfolders: bool,
    /// Extensions without the dot, in lower case. Empty means any file.
    pub extensions: Vec<String>,
    /// The name has to contain this (any case). Empty means any name.
    pub name_has: String,
    /// Size limits in kilobytes. 0 means no limit.
    pub min_kb: u64,
    pub max_kb: u64,
    pub steps: Vec<Step>,
    /// When the rule was made or last turned on (seconds since 1970): older files are left alone.
    pub since: i64,
}

/// Names that say a file is still being written by a browser or a program.
pub fn is_partial(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("~$")
        || n.starts_with(".~lock")
        || [
            ".crdownload",
            ".part",
            ".partial",
            ".tmp",
            ".temp",
            ".download",
            ".opdownload",
            ".!ut",
            ".aria2",
        ]
        .iter()
        .any(|e| n.ends_with(e))
        || n == "desktop.ini"
        || n == "thumbs.db"
}

/// The extension in lower case, without the dot.
pub fn ext_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, e)) if !stem.is_empty() => e.to_lowercase(),
        _ => String::new(),
    }
}

/// Does this rule cover a file with this name and size (bytes)?
pub fn matches(rule: &Rule, name: &str, size: u64) -> bool {
    if is_partial(name) {
        return false;
    }
    if !rule.extensions.is_empty() {
        let e = ext_of(name);
        if !rule.extensions.contains(&e) {
            return false;
        }
    }
    if !rule.name_has.trim().is_empty()
        && !name
            .to_lowercase()
            .contains(&rule.name_has.trim().to_lowercase())
    {
        return false;
    }
    let kb = size / 1024;
    if rule.min_kb > 0 && kb < rule.min_kb {
        return false;
    }
    if rule.max_kb > 0 && kb > rule.max_kb {
        return false;
    }
    true
}

/// Extensions as typed ("png, .JPG  gif") into the stored form.
pub fn parse_extensions(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let e = part.trim().trim_start_matches('.').to_lowercase();
        if !e.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric()) && !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

/// The picture formats a `Convert` step can read.
pub fn is_picture(name: &str) -> bool {
    matches!(
        ext_of(name).as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" | "gif"
    )
}

/// Checks a rule as typed. The message is shown next to the form.
pub fn validate(r: &Rule) -> Result<(), String> {
    if r.name.trim().is_empty() {
        return Err("Give the rule a name.".into());
    }
    if r.name.chars().count() > 60 {
        return Err("The name is too long (60 characters at most).".into());
    }
    if r.folder.trim().is_empty() {
        return Err("Choose the folder to watch.".into());
    }
    if r.steps.is_empty() {
        return Err("Add at least one thing to do.".into());
    }
    if r.steps.len() > MAX_STEPS {
        return Err(format!("At most {MAX_STEPS} steps."));
    }
    for s in &r.steps {
        match s {
            Step::Move { folder } | Step::Copy { folder } => {
                if folder.trim().is_empty() {
                    return Err("A move or copy needs a folder.".into());
                }
            }
            Step::Rename { template } => {
                if template.trim().is_empty() {
                    return Err("Write the new name pattern, for example {date}_{name}.".into());
                }
                if template.contains(['/', '\\']) {
                    return Err(
                        "A name can't hold slashes. Use a Move step to choose the folder.".into(),
                    );
                }
            }
            Step::Convert { to, .. } => {
                if !matches!(to.as_str(), "webp" | "jpg" | "png") {
                    return Err("Convert to WebP, JPG or PNG.".into());
                }
            }
            Step::Run { action } => {
                if action.trim().is_empty() {
                    return Err("Choose which action to run.".into());
                }
            }
            Step::Notify => {}
        }
    }
    Ok(())
}

pub fn check_list(list: &[Rule]) -> Result<(), String> {
    if list.len() > MAX_RULES {
        return Err(format!("At most {MAX_RULES} rules."));
    }
    let mut ids = std::collections::HashSet::new();
    for r in list {
        if !ids.insert(r.id.as_str()) {
            return Err("Two rules have the same id.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> Rule {
        Rule {
            id: "r1".into(),
            name: "Pictures".into(),
            enabled: true,
            folder: "C:\\Users\\x\\Downloads".into(),
            extensions: vec!["png".into(), "jpg".into()],
            steps: vec![Step::Move {
                folder: "C:\\Users\\x\\Pictures".into(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_rule_covers_only_the_files_it_describes() {
        let r = rule();
        assert!(matches(&r, "cat.PNG", 10_000));
        assert!(matches(&r, "a.b.jpg", 10_000));
        assert!(!matches(&r, "cat.gif", 10_000));
        assert!(!matches(&r, "noext", 10_000));
    }

    #[test]
    fn a_rule_with_no_extensions_covers_any_name() {
        let mut r = rule();
        r.extensions.clear();
        r.name_has = "invoice".into();
        assert!(matches(&r, "My INVOICE 2026.pdf", 5));
        assert!(!matches(&r, "photo.png", 5));
    }

    #[test]
    fn size_limits_are_in_kilobytes_and_zero_means_none() {
        let mut r = rule();
        r.min_kb = 100;
        r.max_kb = 1000;
        assert!(!matches(&r, "a.png", 50 * 1024));
        assert!(matches(&r, "a.png", 500 * 1024));
        assert!(!matches(&r, "a.png", 2000 * 1024));
        r.min_kb = 0;
        r.max_kb = 0;
        assert!(matches(&r, "a.png", 9_000_000_000));
    }

    #[test]
    fn a_file_still_being_written_is_never_covered() {
        let mut r = rule();
        r.extensions.clear();
        for n in [
            "a.crdownload",
            "b.part",
            "~$doc.docx",
            "c.tmp",
            "Thumbs.db",
            "desktop.ini",
            "x.download",
        ] {
            assert!(!matches(&r, n, 10), "{n}");
        }
        assert!(matches(&r, "movie.mp4", 10));
    }

    #[test]
    fn extensions_typed_loosely_are_cleaned() {
        assert_eq!(
            parse_extensions("png, .JPG  gif;webp, png"),
            vec!["png", "jpg", "gif", "webp"]
        );
        assert_eq!(parse_extensions("  ,, "), Vec::<String>::new());
        assert_eq!(parse_extensions("p*g"), Vec::<String>::new());
    }

    #[test]
    fn the_extension_of_a_dot_file_is_nothing() {
        assert_eq!(ext_of(".gitignore"), "");
        assert_eq!(ext_of("a.tar.GZ"), "gz");
    }

    #[test]
    fn validation_catches_the_usual_mistakes() {
        assert!(validate(&rule()).is_ok());
        let mut r = rule();
        r.name = " ".into();
        assert!(validate(&r).is_err());
        let mut r = rule();
        r.steps.clear();
        assert!(validate(&r).is_err());
        let mut r = rule();
        r.folder = "".into();
        assert!(validate(&r).is_err());
        let mut r = rule();
        r.steps = vec![Step::Rename {
            template: "a/b".into(),
        }];
        assert!(validate(&r).is_err());
        let mut r = rule();
        r.steps = vec![Step::Convert {
            to: "tiff".into(),
            originals: String::new(),
        }];
        assert!(validate(&r).is_err());
        let mut r = rule();
        r.steps = vec![Step::Move { folder: " ".into() }];
        assert!(validate(&r).is_err());
    }

    #[test]
    fn steps_survive_being_saved_and_read() {
        let r = rule();
        let text = serde_json::to_string(&r).unwrap();
        assert!(text.contains("\"type\":\"move\""));
        assert_eq!(serde_json::from_str::<Rule>(&text).unwrap(), r);
    }

    #[test]
    fn the_list_check_refuses_repeated_ids() {
        let a = rule();
        let mut b = rule();
        assert!(check_list(&[a.clone(), b.clone()]).is_err());
        b.id = "r2".into();
        assert!(check_list(&[a, b]).is_ok());
    }

    #[test]
    fn pictures_are_told_by_extension() {
        assert!(is_picture("a.JPEG"));
        assert!(!is_picture("a.pdf"));
    }
}
