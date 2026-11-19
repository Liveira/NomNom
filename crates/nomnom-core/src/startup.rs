//! What starts with the computer, and how much it slows the start-up down. The parts that need no
//! operating system live here: working out a program's name from its start-up command, reading
//! Windows' record of slow start-ups, and totting it up per program.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The program a start-up command runs: `"C:\Apps\My App\app.exe" --min` gives the path in the
/// quotes, `C:\Apps\app.exe --min` gives everything up to `.exe`.
pub fn program_of(command: &str) -> Option<String> {
    let c = command.trim();
    if let Some(rest) = c.strip_prefix('"') {
        let end = rest.find('"')?;
        let p = rest[..end].trim();
        return (!p.is_empty()).then(|| p.to_string());
    }
    let lower = c.to_ascii_lowercase();
    for ext in [".exe", ".bat", ".cmd", ".lnk", ".com"] {
        if let Some(i) = lower.find(ext) {
            let end = i + ext.len();
            // A real end of the name: the string stops, or a space follows.
            if lower[end..].is_empty() || lower[end..].starts_with(' ') {
                return Some(c[..end].to_string());
            }
        }
    }
    c.split_whitespace().next().map(str::to_string)
}

/// The file name of the program, in lower case, for matching against the system's record.
pub fn program_name(command: &str) -> Option<String> {
    let p = program_of(command)?;
    p.rsplit(['\\', '/'])
        .next()
        .map(|n| n.to_ascii_lowercase())
        .filter(|n| !n.is_empty())
}

/// One slow start-up of one program, from Windows' diagnostics record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slowdown {
    /// File name, lower case.
    pub name: String,
    pub path: String,
    /// How much longer the whole start-up took because of it, in milliseconds.
    pub delay_ms: u64,
    /// When it happened, as the record gives it (`2026-10-23T15:42:29`), for sorting.
    pub when: String,
}

/// How a program has affected the start-ups lately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Effect {
    pub times: u32,
    pub total_ms: u64,
    pub worst_ms: u64,
    pub last_ms: u64,
    pub last_when: String,
}

/// The data of each event `<Event>…</Event>` of a `wevtutil qe … /f:xml` answer.
pub fn parse_slowdowns(xml: &str) -> Vec<Slowdown> {
    xml.split("<Event ")
        .skip(1)
        .filter_map(|ev| {
            let name = data(ev, "Name")?.to_ascii_lowercase();
            let path = data(ev, "Path").unwrap_or_default();
            let delay_ms = data(ev, "DegradationTime")?.parse::<u64>().ok()?;
            let when = data(ev, "StartTime")
                .map(|t| t.chars().take(19).collect())
                .unwrap_or_default();
            (!name.is_empty()).then_some(Slowdown {
                name,
                path,
                delay_ms,
                when,
            })
        })
        .collect()
}

/// The value of `<Data Name='key'>value</Data>` inside one event.
fn data(ev: &str, key: &str) -> Option<String> {
    let open = format!("<Data Name='{key}'>");
    let start = ev.find(&open)? + open.len();
    let end = ev[start..].find("</Data>")? + start;
    Some(unescape(&ev[start..end]))
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Per program (by file name): how often and by how much it slowed the start-up.
pub fn effects(slowdowns: &[Slowdown]) -> HashMap<String, Effect> {
    let mut map: HashMap<String, Effect> = HashMap::new();
    for s in slowdowns {
        let e = map.entry(s.name.clone()).or_default();
        e.times += 1;
        e.total_ms += s.delay_ms;
        e.worst_ms = e.worst_ms.max(s.delay_ms);
        if s.when >= e.last_when {
            e.last_when = s.when.clone();
            e.last_ms = s.delay_ms;
        }
    }
    map
}

/// The boot time of the last start-up in milliseconds, from the "Windows has started up" event.
pub fn parse_boot_time(xml: &str) -> Option<u64> {
    xml.split("<Event ")
        .nth(1)
        .and_then(|ev| data(ev, "BootTime"))
        .and_then(|v| v.parse().ok())
}

/// "25 s", "1 min 12 s", "4.5 s" for a number of milliseconds.
pub fn duration_text(ms: u64) -> String {
    let s = ms as f64 / 1000.0;
    if s < 10.0 {
        format!("{s:.1} s")
    } else if s < 60.0 {
        format!("{:.0} s", s)
    } else {
        let total = s.round() as u64;
        format!("{} min {} s", total / 60, total % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_program_is_found_in_a_start_up_command() {
        assert_eq!(
            program_of(r#""C:\Apps\My App\app.exe" --min"#).as_deref(),
            Some(r"C:\Apps\My App\app.exe")
        );
        assert_eq!(
            program_of(r"C:\Tools\tool.exe -x").as_deref(),
            Some(r"C:\Tools\tool.exe")
        );
        assert_eq!(
            program_of(r"C:\Tools\tool.exe").as_deref(),
            Some(r"C:\Tools\tool.exe")
        );
        assert_eq!(program_of("notepad").as_deref(), Some("notepad"));
        assert_eq!(program_of("   "), None);
        assert_eq!(
            program_name(
                r#""C:\Users\Me\AppData\Local\Discord\Update.exe" --processStart Discord.exe"#
            )
            .as_deref(),
            Some("update.exe")
        );
        assert_eq!(
            program_name("/usr/bin/app --flag").as_deref(),
            Some("/usr/bin/app".rsplit('/').next().unwrap())
        );
    }

    const XML: &str = "<Event xmlns='x'><System><EventID>101</EventID></System><EventData>\
        <Data Name='StartTime'>2026-10-23T15:42:29.6473383Z</Data><Data Name='Name'>Discord.exe</Data>\
        <Data Name='Path'>C:\\Users\\Me\\Discord.exe</Data><Data Name='TotalTime'>30821</Data>\
        <Data Name='DegradationTime'>25821</Data></EventData></Event>\
        <Event xmlns='x'><EventData><Data Name='StartTime'>2026-11-02T08:00:00.1Z</Data>\
        <Data Name='Name'>Discord.exe</Data><Data Name='DegradationTime'>9000</Data></EventData></Event>\
        <Event xmlns='x'><EventData><Data Name='Name'>Odd &amp; Ends.exe</Data><Data Name='DegradationTime'>4000</Data></EventData></Event>\
        <Event xmlns='x'><EventData><Data Name='Name'></Data><Data Name='DegradationTime'>1</Data></EventData></Event>";

    #[test]
    fn slow_starts_are_read_from_the_record() {
        let all = parse_slowdowns(XML);
        assert_eq!(all.len(), 3, "an event without a name is skipped");
        assert_eq!(all[0].name, "discord.exe");
        assert_eq!(all[0].delay_ms, 25_821);
        assert_eq!(all[0].when, "2026-10-23T15:42:29");
        assert_eq!(all[2].name, "odd & ends.exe");
        assert!(parse_slowdowns("garbage").is_empty());
    }

    #[test]
    fn the_effect_of_a_program_adds_up_over_the_boots() {
        let e = &effects(&parse_slowdowns(XML))["discord.exe"];
        assert_eq!(e.times, 2);
        assert_eq!(e.total_ms, 34_821);
        assert_eq!(e.worst_ms, 25_821);
        assert_eq!(e.last_ms, 9_000, "the later boot is the last one");
    }

    #[test]
    fn the_boot_time_and_durations_read_well() {
        let boot = "<Event x><EventData><Data Name='BootTime'>72345</Data></EventData></Event>";
        assert_eq!(parse_boot_time(boot), Some(72_345));
        assert_eq!(parse_boot_time("<Event x></Event>"), None);
        assert_eq!(duration_text(4_500), "4.5 s");
        assert_eq!(duration_text(25_821), "26 s");
        assert_eq!(duration_text(72_345), "1 min 12 s");
    }
}
