//! Game mode: when a game is in front, NomNom can quiet its notices, ask Windows for its fastest power
//! plan and close the background programs the person chose, and put it all back afterwards. This file
//! is the decision part: when a game "started" and "ended" (so a quick Alt+Tab doesn't end it), and
//! the power-plan words.

/// Seconds a game must be in front before game mode starts.
pub const ENTER_AFTER: i64 = 8;
/// Seconds without a game in front before game mode ends (a look at the chat doesn't end it).
pub const LEAVE_AFTER: i64 = 45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Entered,
    Left,
}

#[derive(Debug, Default)]
pub struct Watch {
    on: bool,
    /// Since when the answer has been different from what game mode is doing.
    differs_since: Option<i64>,
}

impl Watch {
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// One look: is a game in front now, at time `now` (seconds)? Says when game mode flips.
    pub fn step(&mut self, game_now: bool, now: i64) -> Option<Change> {
        if game_now == self.on {
            self.differs_since = None;
            return None;
        }
        let since = *self.differs_since.get_or_insert(now);
        let wait = if game_now { ENTER_AFTER } else { LEAVE_AFTER };
        if now - since >= wait {
            self.on = game_now;
            self.differs_since = None;
            Some(if game_now {
                Change::Entered
            } else {
                Change::Left
            })
        } else {
            None
        }
    }

    /// Switched off by the person while it was on: forget it without calling that a change.
    pub fn reset(&mut self) {
        self.on = false;
        self.differs_since = None;
    }
}

pub const HIGH_PERFORMANCE: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
pub const ULTIMATE_PERFORMANCE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

/// Is this a power-plan id (a GUID)? It is passed to `powercfg`, so nothing else may get through.
pub fn is_guid(s: &str) -> bool {
    let p: Vec<&str> = s.split('-').collect();
    p.len() == 5
        && p.iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(x, n)| x.len() == n && x.chars().all(|c| c.is_ascii_hexdigit()))
}

/// The id in `powercfg /getactivescheme`: "Power Scheme GUID: 381b...  (Balanced)".
pub fn parse_active_scheme(out: &str) -> Option<String> {
    out.split_whitespace()
        .map(|w| w.trim_matches(|c| c == '(' || c == ')'))
        .find(|w| is_guid(w))
        .map(str::to_lowercase)
}

/// Is a plan with this id in the output of `powercfg /list`?
pub fn lists_scheme(list_out: &str, guid: &str) -> bool {
    list_out.to_lowercase().contains(&guid.to_lowercase())
}

/// Program names as typed ("OneDrive.exe, discord; Teams.exe") into plain file names.
pub fn parse_programs(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in text.split([',', ';', '\n']) {
        let mut n = part.trim().trim_matches('"').to_string();
        if n.is_empty() || n.contains(['\\', '/', ':', '*', '?', '<', '>', '|', '"']) {
            continue;
        }
        if !n.to_lowercase().ends_with(".exe") {
            n.push_str(".exe");
        }
        if n.len() <= 64
            && !out.iter().any(|o| o.eq_ignore_ascii_case(&n))
            && !is_protected_program(&n)
        {
            out.push(n);
        }
    }
    out
}

/// Programs that are never closed, whatever the list says: closing them breaks the session.
pub fn is_protected_program(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "explorer.exe"
            | "winlogon.exe"
            | "csrss.exe"
            | "lsass.exe"
            | "services.exe"
            | "svchost.exe"
            | "dwm.exe"
            | "smss.exe"
            | "wininit.exe"
            | "system"
            | "nomnom.exe"
            | "taskmgr.exe"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_has_to_stay_in_front_a_few_seconds_to_count() {
        let mut w = Watch::default();
        assert_eq!(w.step(true, 100), None);
        assert_eq!(w.step(true, 104), None);
        assert_eq!(w.step(true, 108), Some(Change::Entered));
        assert!(w.is_on());
        assert_eq!(w.step(true, 120), None);
    }

    #[test]
    fn a_short_look_away_does_not_end_it() {
        let mut w = Watch::default();
        w.step(true, 0);
        assert_eq!(w.step(true, 10), Some(Change::Entered));
        assert_eq!(w.step(false, 20), None);
        assert_eq!(w.step(false, 40), None);
        // Back in the game: the wait starts over.
        assert_eq!(w.step(true, 50), None);
        assert_eq!(w.step(false, 60), None);
        assert_eq!(w.step(false, 104), None);
        assert_eq!(w.step(false, 105), Some(Change::Left));
        assert!(!w.is_on());
    }

    #[test]
    fn a_flicker_of_a_game_does_not_start_it() {
        let mut w = Watch::default();
        assert_eq!(w.step(true, 0), None);
        assert_eq!(w.step(false, 3), None);
        assert_eq!(w.step(true, 5), None);
        assert_eq!(w.step(true, 12), None);
        assert_eq!(w.step(true, 13), Some(Change::Entered));
    }

    #[test]
    fn reset_forgets_without_announcing() {
        let mut w = Watch::default();
        w.step(true, 0);
        w.step(true, 9);
        assert!(w.is_on());
        w.reset();
        assert!(!w.is_on());
        assert_eq!(w.step(false, 100), None);
    }

    #[test]
    fn only_a_real_guid_reaches_powercfg() {
        assert!(is_guid(HIGH_PERFORMANCE));
        assert!(is_guid("381b4222-f694-41f0-9685-ff5bb260df2e"));
        assert!(!is_guid("381b4222-f694-41f0-9685-ff5bb260df2e & calc"));
        assert!(!is_guid("scheme_min"));
        assert!(!is_guid(""));
    }

    #[test]
    fn the_active_plan_is_found_in_the_tool_output() {
        let out = "Power Scheme GUID: 381b4222-F694-41f0-9685-ff5bb260df2e  (Balanced)\r\n";
        assert_eq!(
            parse_active_scheme(out).as_deref(),
            Some("381b4222-f694-41f0-9685-ff5bb260df2e")
        );
        assert!(parse_active_scheme("nothing here").is_none());
        assert!(lists_scheme(
            "GUID: 8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C  (High performance)",
            HIGH_PERFORMANCE
        ));
        assert!(!lists_scheme(
            "GUID: 381b4222-f694-41f0-9685-ff5bb260df2e",
            HIGH_PERFORMANCE
        ));
    }

    #[test]
    fn program_names_are_cleaned_and_system_ones_refused() {
        assert_eq!(
            parse_programs("OneDrive, discord.exe; \"Teams.exe\"\nonedrive"),
            vec!["OneDrive.exe", "discord.exe", "Teams.exe"]
        );
        assert!(parse_programs("explorer.exe, svchost, nomnom").is_empty());
        assert!(parse_programs("C:\\Windows\\x.exe, a/b.exe, ..\\x").is_empty());
        assert!(parse_programs("").is_empty());
    }
}
