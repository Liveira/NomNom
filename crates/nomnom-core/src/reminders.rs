//! Reminders typed in plain words: "in 20 minutes stretch", "tomorrow at 9am call mom", "em 2 horas
//! tirar o bolo", "mañana a las 8 pagar la luz", "every day at 18:30 drink water". English,
//! Portuguese and Spanish. This turns the words into a text and a moment; storing, and showing the
//! card when the time comes, is the app's job.

use chrono::{Duration, NaiveDateTime, NaiveTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Daily,
    Weekly,
}

impl Repeat {
    pub fn as_str(self) -> &'static str {
        match self {
            Repeat::Daily => "daily",
            Repeat::Weekly => "weekly",
        }
    }
    pub fn parse(s: &str) -> Option<Repeat> {
        match s {
            "daily" => Some(Repeat::Daily),
            "weekly" => Some(Repeat::Weekly),
            _ => None,
        }
    }
    /// The next time after `when` (a repeating reminder that fired).
    pub fn next(self, when: NaiveDateTime) -> NaiveDateTime {
        match self {
            Repeat::Daily => when + Duration::days(1),
            Repeat::Weekly => when + Duration::days(7),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub text: String,
    pub when: NaiveDateTime,
    pub repeat: Option<Repeat>,
}

/// The reminder words that start a sentence ("remind me to ..."), removed. `None` when the text
/// doesn't start like a reminder.
pub fn strip_trigger(input: &str) -> Option<&str> {
    let lower = input.to_lowercase();
    const TRIGGERS: &[&str] = &[
        "remind me to ",
        "remind me that ",
        "remind me about ",
        "remind me ",
        "reminder ",
        "me lembre de ",
        "me lembre que ",
        "me lembre ",
        "me lembra de ",
        "me lembra ",
        "lembre-me de ",
        "lembre-me ",
        "lembrete ",
        "lembrar de ",
        "lembrar ",
        "recuérdame que ",
        "recuérdame de ",
        "recuérdame ",
        "recuerdame que ",
        "recuerdame ",
        "recordatorio ",
        "recordarme ",
    ];
    TRIGGERS
        .iter()
        .find(|t| lower.starts_with(*t))
        .map(|t| input[t.len()..].trim())
}

fn number_word(w: &str) -> Option<u32> {
    Some(match w {
        "a" | "an" | "um" | "uma" | "un" | "una" | "one" => 1,
        "two" | "dois" | "duas" | "dos" => 2,
        "three" | "três" | "tres" => 3,
        "five" | "cinco" => 5,
        "ten" | "dez" | "diez" => 10,
        _ => return w.parse().ok(),
    })
}

/// Seconds in a unit word, or `None`.
fn unit_seconds(w: &str) -> Option<i64> {
    Some(match w {
        "s" | "sec" | "secs" | "second" | "seconds" | "seg" | "segundo" | "segundos" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" | "minuto" | "minutos" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" | "hora" | "horas" => 3600,
        "d" | "day" | "days" | "dia" | "dias" | "día" | "días" => 86_400,
        "w" | "week" | "weeks" | "semana" | "semanas" => 604_800,
        _ => return None,
    })
}

/// A time of day written as `9`, `9am`, `9:30`, `9:30pm`, `18h`, `18h30`.
fn clock(w: &str, bare_ok: bool) -> Option<NaiveTime> {
    let (body, suffix) = if let Some(b) = w.strip_suffix("am") {
        (b, Some(false))
    } else if let Some(b) = w.strip_suffix("pm") {
        (b, Some(true))
    } else {
        (w, None)
    };
    let (h, m) = if let Some((h, m)) = body.split_once(':') {
        (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?)
    } else if let Some((h, m)) = body.split_once('h') {
        (
            h.parse::<u32>().ok()?,
            if m.is_empty() {
                0
            } else {
                m.parse::<u32>().ok()?
            },
        )
    } else {
        // A bare number is only a time after "at" or "às".
        if suffix.is_none() && !bare_ok {
            return None;
        }
        (body.parse::<u32>().ok()?, 0)
    };
    let h = match suffix {
        Some(pm) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            (h % 12) + if pm { 12 } else { 0 }
        }
        None => h,
    };
    NaiveTime::from_hms_opt(h, m, 0)
}

fn is_filler(w: &str) -> bool {
    matches!(
        w,
        "to" | "that" | "about" | "de" | "que" | "para" | "a" | "o" | "of" | "for"
    )
}

/// Reads a reminder. `now` is the local time. `None` when no moment is written in the text.
pub fn parse(input: &str, now: NaiveDateTime) -> Option<Parsed> {
    let rest = strip_trigger(input).unwrap_or(input.trim());
    let words: Vec<&str> = rest.split_whitespace().collect();
    let lower: Vec<String> = words
        .iter()
        .map(|w| w.trim_matches(|c: char| ",.;".contains(c)).to_lowercase())
        .collect();
    let mut used = vec![false; words.len()];

    let mut after: Option<Duration> = None;
    let mut day_offset: Option<i64> = None;
    let mut time: Option<NaiveTime> = None;
    let mut repeat: Option<Repeat> = None;

    let mut i = 0;
    while i < words.len() {
        let w = lower[i].as_str();
        // "in 20 minutes", "em 2 horas", "en 5 minutos".
        if matches!(w, "in" | "em" | "en" | "daqui" | "dentro") {
            let mut j = i + 1;
            if matches!(lower.get(j).map(String::as_str), Some("de")) {
                j += 1;
            }
            if let (Some(n), Some(u)) = (
                lower.get(j).and_then(|x| number_word(x)),
                lower.get(j + 1).and_then(|x| unit_seconds(x)),
            ) {
                after = Some(Duration::seconds(i64::from(n) * u));
                (i..=j + 1).for_each(|k| used[k] = true);
                i = j + 2;
                continue;
            }
            // "in half an hour" / "em meia hora"
            if matches!(
                lower.get(j).map(String::as_str),
                Some("half" | "meia" | "media")
            ) {
                let k = if matches!(
                    lower.get(j + 1).map(String::as_str),
                    Some("an" | "a" | "uma" | "un")
                ) {
                    j + 2
                } else {
                    j + 1
                };
                if lower.get(k).and_then(|x| unit_seconds(x)) == Some(3600) {
                    after = Some(Duration::minutes(30));
                    (i..=k).for_each(|x| used[x] = true);
                    i = k + 1;
                    continue;
                }
            }
        }
        // "20 minutes" with no "in" ("stretch 20 minutes from now" is rare; "20min" is common).
        if let Some(n) = w
            .strip_suffix("min")
            .or_else(|| w.strip_suffix("m"))
            .and_then(|d| d.parse::<u32>().ok())
            .filter(|_| w.chars().next().is_some_and(|c| c.is_ascii_digit()))
        {
            after = Some(Duration::minutes(i64::from(n)));
            used[i] = true;
            i += 1;
            continue;
        }
        match w {
            "today" | "hoje" | "hoy" => {
                day_offset = Some(0);
                used[i] = true;
            }
            "tomorrow" | "amanhã" | "amanha" | "mañana" | "manana" => {
                day_offset = Some(1);
                used[i] = true;
            }
            "every" | "todo" | "toda" | "todos" | "todas" | "cada" => {
                let next = lower.get(i + 1).map(String::as_str);
                match next {
                    Some("day" | "dia" | "día" | "days" | "dias" | "días") => {
                        repeat = Some(Repeat::Daily);
                        used[i] = true;
                        used[i + 1] = true;
                        i += 2;
                        continue;
                    }
                    Some("week" | "semana") => {
                        repeat = Some(Repeat::Weekly);
                        used[i] = true;
                        used[i + 1] = true;
                        i += 2;
                        continue;
                    }
                    _ => {}
                }
            }
            "at" | "às" | "as" | "às." | "a" | "la" | "las" => {
                // "at 9", "às 9h", "a las 8": the time is the next word (after "las"/"la").
                let mut j = i + 1;
                if matches!(w, "a")
                    && matches!(lower.get(j).map(String::as_str), Some("las" | "la"))
                {
                    j += 1;
                }
                if let Some(t) = lower.get(j).and_then(|x| clock(x, true)) {
                    time = Some(t);
                    (i..=j).for_each(|k| used[k] = true);
                    i = j + 1;
                    continue;
                }
            }
            _ => {
                if time.is_none() {
                    if let Some(t) = clock(w, false) {
                        time = Some(t);
                        used[i] = true;
                    }
                }
            }
        }
        i += 1;
    }

    let when = if let Some(d) = after {
        now + d
    } else if let Some(t) = time {
        let base = now.date() + Duration::days(day_offset.unwrap_or(0));
        let at = base.and_time(t);
        if at <= now && day_offset.is_none() {
            if repeat == Some(Repeat::Weekly) {
                at + Duration::days(7)
            } else {
                at + Duration::days(1)
            }
        } else {
            at
        }
    } else {
        // "tomorrow" alone: nine in the morning.
        (now.date() + Duration::days(day_offset?)).and_hms_opt(9, 0, 0)?
    };
    if when <= now && after.is_none() {
        return None;
    }

    let mut kept: Vec<&str> = words
        .iter()
        .enumerate()
        .filter(|(i, _)| !used[*i])
        .map(|(_, w)| *w)
        .collect();
    while kept.first().is_some_and(|w| is_filler(&w.to_lowercase())) {
        kept.remove(0);
    }
    while kept.last().is_some_and(|w| is_filler(&w.to_lowercase())) {
        kept.pop();
    }
    let text = kept.join(" ");
    Some(Parsed {
        text: if text.trim().is_empty() {
            "Reminder".into()
        } else {
            text
        },
        when,
        repeat,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    /// Thursday 2026-11-19, 14:30.
    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 19)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap()
    }
    fn at(d: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, d)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
    }

    #[test]
    fn a_wait_in_minutes() {
        let p = parse("in 20 minutes stretch", now()).unwrap();
        assert_eq!(p.text, "stretch");
        assert_eq!(p.when, at(19, 14, 50));
        assert_eq!(
            parse("remind me to call mom in 2 hours", now())
                .unwrap()
                .when,
            at(19, 16, 30)
        );
        assert_eq!(
            parse("in an hour check the oven", now()).unwrap().when,
            at(19, 15, 30)
        );
        assert_eq!(
            parse("in half an hour tea", now()).unwrap().when,
            at(19, 15, 0)
        );
    }

    #[test]
    fn tomorrow_at_a_time() {
        let p = parse("tomorrow at 9am call mom", now()).unwrap();
        assert_eq!((p.text.as_str(), p.when), ("call mom", at(20, 9, 0)));
        assert_eq!(
            parse("tomorrow 18:30 gym", now()).unwrap().when,
            at(20, 18, 30)
        );
        assert_eq!(
            parse("tomorrow pay rent", now()).unwrap().when,
            at(20, 9, 0)
        );
    }

    #[test]
    fn a_time_that_passed_means_tomorrow_and_one_that_has_not_means_today() {
        assert_eq!(parse("at 9 stand up", now()).unwrap().when, at(20, 9, 0));
        assert_eq!(parse("at 18:00 dinner", now()).unwrap().when, at(19, 18, 0));
        assert_eq!(parse("at 3pm meeting", now()).unwrap().when, at(19, 15, 0));
    }

    #[test]
    fn portuguese() {
        let p = parse("me lembre em 20 minutos de tirar o bolo", now()).unwrap();
        assert_eq!((p.text.as_str(), p.when), ("tirar o bolo", at(19, 14, 50)));
        let q = parse("amanhã às 9h reunião", now()).unwrap();
        assert_eq!((q.text.as_str(), q.when), ("reunião", at(20, 9, 0)));
        assert_eq!(
            parse("hoje às 18h30 jantar", now()).unwrap().when,
            at(19, 18, 30)
        );
        assert_eq!(
            parse("em meia hora chá", now()).unwrap().when,
            at(19, 15, 0)
        );
    }

    #[test]
    fn spanish() {
        let p = parse("recuérdame mañana a las 8 pagar la luz", now()).unwrap();
        assert_eq!((p.text.as_str(), p.when), ("pagar la luz", at(20, 8, 0)));
        assert_eq!(
            parse("en 5 minutos café", now()).unwrap().when,
            at(19, 14, 35)
        );
    }

    #[test]
    fn repeating() {
        let p = parse("every day at 18:30 drink water", now()).unwrap();
        assert_eq!(
            (p.text.as_str(), p.when, p.repeat),
            ("drink water", at(19, 18, 30), Some(Repeat::Daily))
        );
        let w = parse("todo dia às 7h correr", now()).unwrap();
        assert_eq!(w.repeat, Some(Repeat::Daily));
        assert_eq!(w.when, at(20, 7, 0));
        assert_eq!(Repeat::Weekly.next(at(19, 9, 0)), at(26, 9, 0));
    }

    #[test]
    fn no_moment_no_reminder() {
        assert!(parse("buy milk", now()).is_none());
        assert!(parse("", now()).is_none());
        assert!(parse("call 3 people", now()).is_none());
    }

    #[test]
    fn the_text_keeps_its_case_and_words() {
        let p = parse("in 5 min Call Dr. Silva about the report", now()).unwrap();
        assert_eq!(p.text, "Call Dr. Silva about the report");
    }

    #[test]
    fn triggers_are_removed_only_at_the_start() {
        assert_eq!(
            strip_trigger("Remind me to stretch in 5 min"),
            Some("stretch in 5 min")
        );
        assert_eq!(
            strip_trigger("me lembre de beber água em 10 min"),
            Some("beber água em 10 min")
        );
        assert_eq!(strip_trigger("please remind me"), None);
    }
}
