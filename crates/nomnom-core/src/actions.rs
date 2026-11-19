//! Custom actions: things the person adds themselves, such as "open this folder", "open this site",
//! "start this program" or "run this command". They can have a global shortcut, show up in the
//! command palette and the launcher, and ask for values (parameters) when they run.
//!
//! Nothing here runs anything. It checks what was typed and turns it into a plan: a program with an
//! argument list (never a shell line), or a path or link to hand to the system.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const MAX_ACTIONS: usize = 200;
pub const MAX_PARAMS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Start a program (a window is expected).
    #[default]
    App,
    /// Open a folder or a file with whatever opens it.
    Folder,
    /// Open a web address (or a mail link).
    Link,
    /// Run a program and show what it printed.
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParamKind {
    #[default]
    Text,
    File,
    Folder,
}

/// A value asked for when the action runs. Written as `{name}` in the target or the arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Param {
    pub name: String,
    pub label: String,
    pub default: String,
    pub kind: ParamKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Action {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    /// A program, a folder, a file or a link, depending on `kind`. May contain `{name}` values.
    pub target: String,
    /// The arguments as typed: spaces separate them, quotes keep one together.
    pub args: String,
    pub params: Vec<Param>,
    /// A global shortcut such as `Ctrl+Alt+1`.
    pub shortcut: Option<String>,
    /// Show this action in the command palette and the launcher.
    pub in_palette: bool,
    /// The folder to run in (programs and commands).
    pub cwd: Option<String>,
}

/// What running an action comes down to.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Hand this folder, file or link to the system.
    Open(String),
    /// Start this program with these arguments. `wait` means: collect what it prints.
    Run {
        program: String,
        args: Vec<String>,
        cwd: Option<String>,
        wait: bool,
    },
}

/// Splits what was typed into arguments: spaces separate, `"double"` or `'single'` quotes keep text
/// together. A backslash is just a backslash (Windows paths), except `\"` inside double quotes.
pub fn split_args(text: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == '\\' && q == '"' && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else if c == q {
                    quote = None;
                } else {
                    cur.push(c);
                }
            }
            None => match c {
                '"' | '\'' => {
                    quote = Some(c);
                    started = true;
                }
                c if c.is_whitespace() => {
                    if started || !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                        started = false;
                    }
                }
                c => {
                    cur.push(c);
                    started = true;
                }
            },
        }
    }
    if quote.is_some() {
        return Err("A quote was opened and never closed.".into());
    }
    if started || !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

/// The `{name}` values written in `text`, in order of first appearance.
pub fn placeholders(text: &str) -> Vec<String> {
    let mut seen = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else { break };
        let name = &after[..end];
        if is_name(name) && !seen.iter().any(|s| s == name) {
            seen.push(name.to_string());
        }
        rest = &after[end + 1..];
    }
    seen
}

fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !s.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// Replaces every known `{name}` with its value. Unknown braces stay as typed.
pub fn expand(text: &str, values: &HashMap<String, String>, encode: bool) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) if values.contains_key(&after[..end]) => {
                let v = &values[&after[..end]];
                if encode {
                    out.push_str(&percent_encode(v));
                } else {
                    out.push_str(v);
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Encodes a value so it is safe inside a web address.
pub fn percent_encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Checks an action as typed. The message is shown next to the form, so it is plain.
pub fn validate(a: &Action) -> Result<(), String> {
    let name = a.name.trim();
    if name.is_empty() {
        return Err("Give it a name.".into());
    }
    if name.chars().count() > 60 {
        return Err("The name is too long (60 characters at most).".into());
    }
    let target = a.target.trim();
    if target.is_empty() {
        return Err(match a.kind {
            Kind::App | Kind::Command => "Choose the program to run.",
            Kind::Folder => "Choose the folder or file to open.",
            Kind::Link => "Type the web address to open.",
        }
        .into());
    }
    if a.kind == Kind::Link {
        let lower = target.to_ascii_lowercase();
        if !(lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("mailto:"))
        {
            return Err("A link has to start with http://, https:// or mailto:.".into());
        }
    }
    if a.params.len() > MAX_PARAMS {
        return Err(format!("At most {MAX_PARAMS} values."));
    }
    let mut names = HashSet::new();
    for p in &a.params {
        if !is_name(&p.name) {
            return Err(format!(
                "\"{}\" can't be a value name. Use letters, numbers and _, starting with a letter.",
                p.name
            ));
        }
        if !names.insert(p.name.as_str()) {
            return Err(format!("The value \"{}\" is listed twice.", p.name));
        }
    }
    let used: Vec<String> = placeholders(target)
        .into_iter()
        .chain(placeholders(&a.args))
        .collect();
    for u in used {
        if !names.contains(u.as_str()) {
            return Err(format!(
                "{{{u}}} is used but there is no value called \"{u}\". Add it, or remove the braces."
            ));
        }
    }
    split_args(&a.args)?;
    Ok(())
}

/// The plan for running `a` with `values` (a value not given falls back to its default).
pub fn plan(a: &Action, given: &HashMap<String, String>) -> Result<Plan, String> {
    validate(a)?;
    let mut values: HashMap<String, String> = HashMap::new();
    for p in &a.params {
        let v = given
            .get(&p.name)
            .cloned()
            .unwrap_or_else(|| p.default.clone());
        if v.contains('\0') {
            return Err("A value has a character that can't be used.".into());
        }
        values.insert(p.name.clone(), v);
    }
    let target = a.target.trim();
    match a.kind {
        Kind::Link => Ok(Plan::Open(expand(target, &values, true))),
        Kind::Folder => Ok(Plan::Open(expand(target, &values, false))),
        Kind::App | Kind::Command => {
            // Split first, then fill in: a value with spaces stays one argument.
            let args = split_args(&a.args)?
                .iter()
                .map(|x| expand(x, &values, false))
                .collect();
            Ok(Plan::Run {
                program: expand(target, &values, false),
                args,
                cwd: a.cwd.clone().filter(|c| !c.trim().is_empty()),
                wait: a.kind == Kind::Command,
            })
        }
    }
}

/// Keeps the list sane before it is saved: unique ids, no duplicate shortcuts, a size limit.
pub fn check_list(list: &[Action]) -> Result<(), String> {
    if list.len() > MAX_ACTIONS {
        return Err(format!("At most {MAX_ACTIONS} actions."));
    }
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();
    for a in list {
        if !ids.insert(a.id.as_str()) {
            return Err("Two actions have the same id.".into());
        }
        if let Some(k) = a.shortcut.as_deref().map(|k| k.trim().to_ascii_lowercase()) {
            if !k.is_empty() && !keys.insert(k) {
                return Err("Two actions use the same shortcut.".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(kind: Kind, target: &str, args: &str) -> Action {
        Action {
            id: "a1".into(),
            name: "Test".into(),
            kind,
            target: target.into(),
            args: args.into(),
            in_palette: true,
            ..Default::default()
        }
    }

    fn text_param(name: &str, default: &str) -> Param {
        Param {
            name: name.into(),
            label: name.into(),
            default: default.into(),
            kind: ParamKind::Text,
        }
    }

    #[test]
    fn split_keeps_quoted_text_together() {
        assert_eq!(
            split_args(r#"-o "C:\My Files\out.txt" --name 'a b' plain"#).unwrap(),
            vec!["-o", r"C:\My Files\out.txt", "--name", "a b", "plain"]
        );
        assert_eq!(split_args("").unwrap(), Vec::<String>::new());
        assert_eq!(split_args("   ").unwrap(), Vec::<String>::new());
        assert_eq!(split_args(r#""" x"#).unwrap(), vec!["", "x"]);
    }

    #[test]
    fn split_leaves_backslashes_alone_and_rejects_open_quotes() {
        assert_eq!(
            split_args(r"C:\a\b D:\c").unwrap(),
            vec![r"C:\a\b", r"D:\c"]
        );
        assert_eq!(split_args(r#""say \"hi\"""#).unwrap(), vec![r#"say "hi""#]);
        assert!(split_args(r#""never closed"#).is_err());
    }

    #[test]
    fn placeholders_are_found_once_in_order() {
        assert_eq!(placeholders("{a} x {b} {a} {} {1x} {c d}"), vec!["a", "b"]);
    }

    #[test]
    fn expand_fills_known_values_and_leaves_other_braces() {
        let mut v = HashMap::new();
        v.insert("q".to_string(), "a b&c".to_string());
        assert_eq!(expand("x={q} {other} {", &v, false), "x=a b&c {other} {");
        assert_eq!(expand("x={q}", &v, true), "x=a%20b%26c");
    }

    #[test]
    fn a_link_value_cannot_break_out_of_the_address() {
        let mut a = act(Kind::Link, "https://example.com/s?q={q}", "");
        a.params = vec![text_param("q", "")];
        let mut given = HashMap::new();
        given.insert("q".to_string(), "a&b=c #x".to_string());
        assert_eq!(
            plan(&a, &given).unwrap(),
            Plan::Open("https://example.com/s?q=a%26b%3Dc%20%23x".into())
        );
    }

    #[test]
    fn a_value_with_spaces_stays_one_argument() {
        let mut a = act(Kind::Command, "tool", "--file {f} --flag");
        a.params = vec![text_param("f", "")];
        let mut given = HashMap::new();
        given.insert("f".to_string(), r"C:\My Files\x y.txt; calc".to_string());
        match plan(&a, &given).unwrap() {
            Plan::Run { args, wait, .. } => {
                assert_eq!(args, vec!["--file", r"C:\My Files\x y.txt; calc", "--flag"]);
                assert!(wait);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_missing_value_uses_its_default() {
        let mut a = act(Kind::App, "prog", "{mode}");
        a.params = vec![text_param("mode", "fast")];
        match plan(&a, &HashMap::new()).unwrap() {
            Plan::Run { args, wait, .. } => {
                assert_eq!(args, vec!["fast"]);
                assert!(!wait);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn validation_catches_the_usual_mistakes() {
        assert!(validate(&act(Kind::App, "prog", "")).is_ok());
        let mut a = act(Kind::App, "prog", "");
        a.name = "  ".into();
        assert!(validate(&a).is_err());
        assert!(validate(&act(Kind::App, "", "")).is_err());
        assert!(validate(&act(Kind::Link, "ftp://x", "")).is_err());
        assert!(validate(&act(Kind::Link, "https://x.org", "")).is_ok());
        assert!(validate(&act(Kind::Link, "javascript:alert(1)", "")).is_err());
        assert!(validate(&act(Kind::App, "prog", "{nobody}")).is_err());
        assert!(validate(&act(Kind::App, "prog", r#""open"#)).is_err());
        let mut b = act(Kind::App, "prog", "");
        b.params = vec![text_param("x", ""), text_param("x", "")];
        assert!(validate(&b).is_err());
        b.params = vec![text_param("1x", "")];
        assert!(validate(&b).is_err());
    }

    #[test]
    fn the_list_check_refuses_repeated_ids_and_shortcuts() {
        let mut a = act(Kind::App, "p", "");
        let mut b = act(Kind::App, "p", "");
        b.id = "a2".into();
        assert!(check_list(&[a.clone(), b.clone()]).is_ok());
        b.id = "a1".into();
        assert!(check_list(&[a.clone(), b.clone()]).is_err());
        b.id = "a2".into();
        a.shortcut = Some("Ctrl+Alt+1".into());
        b.shortcut = Some("ctrl+alt+1".into());
        assert!(check_list(&[a, b]).is_err());
    }
}
