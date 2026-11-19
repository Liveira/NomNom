//! Playtime goals: a limit ("at most 2 hours of games a day") or a target ("at least 2 hours of
//! Clip Studio a day"), for one app, one kind of app, or everything. This module holds the rules and
//! the weekly report; the app reads the hours from the database and shows the alerts.

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Stay under this much time.
    Limit,
    /// Reach at least this much time.
    Target,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Day,
    Week,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    /// `app:<id>`, `category:<name>` or `all`.
    pub scope: String,
    pub kind: Kind,
    pub period: Period,
    pub minutes: u32,
    pub enabled: bool,
}

/// Where a goal stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// A limit, with room left.
    Under,
    /// A limit, nearly used up (80% or more).
    Near,
    /// A limit that has been passed.
    Over,
    /// A target, not reached yet.
    Behind,
    /// A target that has been reached.
    Met,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Progress {
    pub used_secs: u64,
    pub goal_secs: u64,
    /// 0 to 100 (it can go above for a limit that was passed; the bar caps it).
    pub percent: u32,
    pub state: State,
}

pub fn progress(goal: &Goal, used_secs: u64) -> Progress {
    let goal_secs = u64::from(goal.minutes) * 60;
    let percent = (used_secs * 100)
        .checked_div(goal_secs)
        .unwrap_or(0)
        .min(999) as u32;
    let state = match goal.kind {
        Kind::Limit if used_secs >= goal_secs => State::Over,
        Kind::Limit if percent >= 80 => State::Near,
        Kind::Limit => State::Under,
        Kind::Target if used_secs >= goal_secs => State::Met,
        Kind::Target => State::Behind,
    };
    Progress {
        used_secs,
        goal_secs,
        percent,
        state,
    }
}

/// The days a goal looks at, as `YYYY-MM-DD`, from the start of the period up to `today`.
/// A day is today; a week runs Monday to Sunday.
pub fn period_range(period: Period, today: NaiveDate) -> (String, String) {
    let fmt = |d: NaiveDate| d.format("%Y-%m-%d").to_string();
    match period {
        Period::Day => (fmt(today), fmt(today)),
        Period::Week => (fmt(monday_of(today)), fmt(today)),
    }
}

pub fn monday_of(day: NaiveDate) -> NaiveDate {
    day - Duration::days(i64::from(day.weekday().num_days_from_monday()))
}

/// What identifies this period, so an alert is shown once per day or per week.
pub fn period_key(period: Period, today: NaiveDate) -> String {
    match period {
        Period::Day => today.format("%Y-%m-%d").to_string(),
        Period::Week => monday_of(today).format("%Y-%m-%d").to_string(),
    }
}

/// The Monday of the last week that is over.
pub fn last_full_week(today: NaiveDate) -> NaiveDate {
    monday_of(today) - Duration::days(7)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alert {
    LimitNear,
    LimitOver,
    TargetMet,
    /// A target is still short late in the day (or on the last day of the week).
    TargetBehind,
}

impl Alert {
    pub fn code(self) -> &'static str {
        match self {
            Alert::LimitNear => "near",
            Alert::LimitOver => "over",
            Alert::TargetMet => "met",
            Alert::TargetBehind => "behind",
        }
    }
}

/// The alerts that are due now. The caller remembers which ones it already showed in this period.
/// `reminder_hour` is the hour of the day for the "you are still short" reminder (24 = never).
pub fn due_alerts(
    goal: &Goal,
    used_secs: u64,
    hour: u32,
    weekday: Weekday,
    reminder_hour: u8,
) -> Vec<Alert> {
    if !goal.enabled || goal.minutes == 0 {
        return Vec::new();
    }
    let p = progress(goal, used_secs);
    let mut out = Vec::new();
    match goal.kind {
        Kind::Limit => {
            if p.state == State::Over {
                out.push(Alert::LimitOver);
            } else if p.state == State::Near {
                out.push(Alert::LimitNear);
            }
        }
        Kind::Target => {
            if p.state == State::Met {
                out.push(Alert::TargetMet);
            } else if reminder_hour < 24 && hour >= u32::from(reminder_hour) {
                let last_day = goal.period == Period::Day || weekday == Weekday::Sun;
                if last_day {
                    out.push(Alert::TargetBehind);
                }
            }
        }
    }
    out
}

/// One goal's result for a finished week.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalResult {
    pub scope: String,
    pub name: String,
    pub kind: Kind,
    pub period: Period,
    pub minutes: u32,
    /// Days (for a daily goal) or weeks (1) in which it was kept.
    pub kept: u32,
    pub out_of: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopApp {
    pub name: String,
    pub secs: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// The Monday the week began, `YYYY-MM-DD`.
    pub week_start: String,
    pub total_secs: u64,
    pub previous_total_secs: u64,
    /// Seconds for each day from Monday.
    pub days: [u64; 7],
    pub top_apps: Vec<TopApp>,
    pub goals: Vec<GoalResult>,
}

impl Report {
    /// How much more (positive) or less (negative) than the week before, in percent, when there is
    /// a week before to compare with.
    pub fn change_percent(&self) -> Option<i64> {
        if self.previous_total_secs == 0 {
            return None;
        }
        let now = self.total_secs as i64;
        let before = self.previous_total_secs as i64;
        Some(((now - before) * 100) / before)
    }

    /// The day (0 = Monday) with the most time, when there was any.
    pub fn busiest_day(&self) -> Option<usize> {
        let (i, secs) = self.days.iter().enumerate().max_by_key(|(_, s)| **s)?;
        (*secs > 0).then_some(i)
    }
}

/// How a daily goal did on one day.
pub fn kept(goal_kind: Kind, goal_minutes: u32, used_secs: u64) -> bool {
    let goal_secs = u64::from(goal_minutes) * 60;
    match goal_kind {
        Kind::Limit => used_secs <= goal_secs,
        Kind::Target => used_secs >= goal_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn goal(kind: Kind, period: Period, minutes: u32) -> Goal {
        Goal {
            id: "g".into(),
            scope: "app:exe:csp.exe".into(),
            kind,
            period,
            minutes,
            enabled: true,
        }
    }
    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn a_limit_goes_from_under_to_near_to_over() {
        let g = goal(Kind::Limit, Period::Day, 120);
        assert_eq!(progress(&g, 0).state, State::Under);
        assert_eq!(progress(&g, 100 * 60).state, State::Near);
        assert_eq!(progress(&g, 120 * 60).state, State::Over);
        assert_eq!(progress(&g, 60 * 60).percent, 50);
    }

    #[test]
    fn a_target_is_behind_until_it_is_reached() {
        let g = goal(Kind::Target, Period::Day, 120);
        assert_eq!(progress(&g, 119 * 60).state, State::Behind);
        assert_eq!(progress(&g, 120 * 60).state, State::Met);
        assert_eq!(progress(&goal(Kind::Target, Period::Day, 0), 5).percent, 0);
    }

    #[test]
    fn days_and_weeks_start_where_people_expect() {
        // 2026-11-19 is a Thursday.
        assert_eq!(
            period_range(Period::Day, day("2026-11-19")),
            ("2026-11-19".into(), "2026-11-19".into())
        );
        assert_eq!(
            period_range(Period::Week, day("2026-11-19")),
            ("2026-11-16".into(), "2026-11-19".into())
        );
        assert_eq!(period_key(Period::Week, day("2026-11-22")), "2026-11-16");
        assert_eq!(period_key(Period::Week, day("2026-11-23")), "2026-11-23");
        assert_eq!(last_full_week(day("2026-11-19")), day("2026-11-09"));
        assert_eq!(last_full_week(day("2026-11-23")), day("2026-11-16"));
    }

    #[test]
    fn a_limit_warns_when_near_and_again_when_over() {
        let g = goal(Kind::Limit, Period::Day, 100);
        assert!(due_alerts(&g, 10 * 60, 12, Weekday::Thu, 20).is_empty());
        assert_eq!(
            due_alerts(&g, 85 * 60, 12, Weekday::Thu, 20),
            vec![Alert::LimitNear]
        );
        assert_eq!(
            due_alerts(&g, 101 * 60, 12, Weekday::Thu, 20),
            vec![Alert::LimitOver]
        );
    }

    #[test]
    fn a_target_celebrates_and_reminds_only_late() {
        let g = goal(Kind::Target, Period::Day, 120);
        assert_eq!(
            due_alerts(&g, 130 * 60, 9, Weekday::Thu, 20),
            vec![Alert::TargetMet]
        );
        assert!(
            due_alerts(&g, 30 * 60, 15, Weekday::Thu, 20).is_empty(),
            "too early to remind"
        );
        assert_eq!(
            due_alerts(&g, 30 * 60, 20, Weekday::Thu, 20),
            vec![Alert::TargetBehind]
        );
        assert!(
            due_alerts(&g, 30 * 60, 23, Weekday::Thu, 24).is_empty(),
            "reminders are off"
        );
    }

    #[test]
    fn a_weekly_target_reminds_only_on_the_last_day() {
        let g = goal(Kind::Target, Period::Week, 600);
        assert!(due_alerts(&g, 60, 21, Weekday::Wed, 20).is_empty());
        assert_eq!(
            due_alerts(&g, 60, 21, Weekday::Sun, 20),
            vec![Alert::TargetBehind]
        );
    }

    #[test]
    fn a_goal_that_is_off_says_nothing() {
        let mut g = goal(Kind::Limit, Period::Day, 10);
        g.enabled = false;
        assert!(due_alerts(&g, 99_999, 12, Weekday::Mon, 20).is_empty());
    }

    #[test]
    fn the_report_compares_with_the_week_before() {
        let mut r = Report {
            week_start: "2026-11-09".into(),
            total_secs: 3600 * 12,
            previous_total_secs: 3600 * 10,
            days: [0, 3600, 7200, 0, 3600, 0, 0],
            top_apps: vec![],
            goals: vec![],
        };
        assert_eq!(r.change_percent(), Some(20));
        assert_eq!(r.busiest_day(), Some(2));
        r.previous_total_secs = 0;
        assert_eq!(r.change_percent(), None);
        r.days = [0; 7];
        assert_eq!(r.busiest_day(), None);
    }

    #[test]
    fn a_day_is_kept_when_the_limit_holds_or_the_target_is_reached() {
        assert!(kept(Kind::Limit, 60, 3600));
        assert!(!kept(Kind::Limit, 60, 3601));
        assert!(kept(Kind::Target, 60, 3600));
        assert!(!kept(Kind::Target, 60, 3599));
    }
}
