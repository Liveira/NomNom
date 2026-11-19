//! Searching for files by name through what the system already keeps: the Windows Search index, or
//! `plocate` on Linux. NomNom builds no index of its own, so this costs no memory. This file makes
//! the query safe and reads the answer; running the search is the app's job.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    /// Seconds since 1970, or 0 when unknown.
    pub modified: i64,
}

pub const MAX_TERM: usize = 80;

/// The words to look for, cleaned up so they can't change the meaning of a query: no wildcards,
/// brackets or quotes, and at least two letters. `None` when nothing searchable is left.
pub fn clean_term(input: &str) -> Option<String> {
    let t: String = input
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '%' | '_' | '[' | ']' | '\'' | '"' | '*' | '?' | '\\' | ';' | '\0'
            ) && !c.is_control()
        })
        .collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let t: String = t.chars().take(MAX_TERM).collect();
    (t.chars().filter(|c| c.is_alphanumeric()).count() >= 2).then_some(t)
}

/// The SQL for the Windows Search index: names that contain every word, newest first.
pub fn windows_sql(term: &str, limit: u32) -> Option<String> {
    let clean = clean_term(term)?;
    let conditions: Vec<String> = clean
        .split(' ')
        .map(|w| format!("System.FileName LIKE '%{w}%'"))
        .collect();
    Some(format!(
        "SELECT TOP {} System.ItemPathDisplay, System.ItemType, System.Size, System.DateModified FROM SystemIndex WHERE {} ORDER BY System.DateModified DESC",
        limit.clamp(1, 100),
        conditions.join(" AND ")
    ))
}

/// Reads what the PowerShell script printed: a JSON array (or one object) of rows.
pub fn parse_windows(json: &str) -> Vec<Hit> {
    #[derive(Deserialize)]
    struct Row {
        #[serde(default)]
        path: Option<String>,
        #[serde(default, rename = "type")]
        kind: Option<String>,
        #[serde(default)]
        size: Option<f64>,
        #[serde(default)]
        modified: Option<f64>,
    }
    let text = json.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let rows: Vec<Row> = serde_json::from_str::<Vec<Row>>(text)
        .or_else(|_| serde_json::from_str::<Row>(text).map(|r| vec![r]))
        .unwrap_or_default();
    rows.into_iter()
        .filter_map(|r| {
            let path = r.path.filter(|p| !p.is_empty())?;
            let name = path
                .rsplit(['\\', '/'])
                .next()
                .filter(|n| !n.is_empty())
                .unwrap_or(&path)
                .to_string();
            Some(Hit {
                is_dir: r
                    .kind
                    .as_deref()
                    .is_some_and(|k| k.eq_ignore_ascii_case("Directory")),
                size: r.size.map(|s| s.max(0.0) as u64).unwrap_or(0),
                modified: r.modified.map(|m| m as i64).unwrap_or(0),
                name,
                path,
            })
        })
        .collect()
}

/// The paths `plocate` printed, one per line, with the name each ends in.
pub fn parse_locate(output: &str, limit: usize) -> Vec<Hit> {
    output
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('/'))
        .take(limit)
        .map(|p| Hit {
            name: p.rsplit('/').next().unwrap_or(p).to_string(),
            path: p.to_string(),
            is_dir: false,
            size: 0,
            modified: 0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_term_cannot_change_the_query() {
        assert_eq!(clean_term("report 2026").as_deref(), Some("report 2026"));
        assert_eq!(clean_term("  a'b%c_d[e]\"f  ").as_deref(), Some("abcdef"));
        assert_eq!(
            clean_term("x'); DROP TABLE y; --").as_deref(),
            Some("x) DROP TABLE y --")
        );
        assert!(clean_term("%%").is_none());
        assert!(clean_term("a").is_none());
        assert!(clean_term("   ").is_none());
    }

    #[test]
    fn the_sql_asks_for_every_word_and_nothing_else() {
        let q = windows_sql("my report", 30).unwrap();
        assert!(q.contains("System.FileName LIKE '%my%' AND System.FileName LIKE '%report%'"));
        assert!(q.starts_with("SELECT TOP 30 "));
        assert!(!q.contains(';'));
        assert!(windows_sql("'", 30).is_none());
        assert!(windows_sql("abc", 5000).unwrap().contains("TOP 100 "));
    }

    #[test]
    fn rows_become_hits() {
        let json = r#"[{"path":"C:\\Users\\a\\Docs\\Report 2026.pdf","type":".pdf","size":2048,"modified":1700000000},
                       {"path":"C:\\Users\\a\\Docs","type":"Directory","size":null,"modified":null}]"#;
        let hits = parse_windows(json);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].name, "Report 2026.pdf");
        assert_eq!(hits[0].size, 2048);
        assert!(!hits[0].is_dir);
        assert!(hits[1].is_dir);
        assert_eq!(hits[1].name, "Docs");
    }

    #[test]
    fn one_row_and_nothing_are_both_fine() {
        assert_eq!(
            parse_windows(r#"{"path":"C:\\a.txt","type":".txt","size":1,"modified":2}"#).len(),
            1
        );
        assert!(parse_windows("").is_empty());
        assert!(parse_windows("not json").is_empty());
        assert!(parse_windows(r#"[{"type":".txt"}]"#).is_empty());
    }

    #[test]
    fn locate_output_keeps_absolute_paths_only() {
        let hits = parse_locate("/home/a/notes.txt\nnoise\n/home/a/b/report.pdf\n", 10);
        assert_eq!(
            hits.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(),
            vec!["notes.txt", "report.pdf"]
        );
        assert_eq!(parse_locate("/a\n/b\n/c\n", 2).len(), 2);
    }
}
