//! Text snippets: a piece of text saved under a name, pasted with a couple of keys. The text can hold
//! values that are filled in when it is used: today's date, the time, what is on the clipboard.

pub const MAX_BODY: usize = 20_000;
pub const MAX_NAME: usize = 60;

/// What a snippet can mention. The app fills these in at the moment of use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    /// 2026-11-19
    pub date: String,
    /// 14:30
    pub time: String,
    /// Thursday
    pub weekday: String,
    pub year: String,
    pub month: String,
    pub day: String,
    pub clipboard: String,
}

/// The names a snippet may use, for the help text on the page.
pub const VARIABLES: &[(&str, &str)] = &[
    ("{date}", "today's date, like 2026-11-19"),
    ("{time}", "the time, like 14:30"),
    ("{weekday}", "the day of the week"),
    ("{year}", "the year"),
    ("{month}", "the month number"),
    ("{day}", "the day of the month"),
    ("{clipboard}", "whatever is on the clipboard"),
];

/// The text with every known `{name}` filled in. Anything else in braces stays as it is, and a value
/// is never read again as text to fill in, so a clipboard that holds "{date}" stays "{date}".
pub fn expand(body: &str, ctx: &Context) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let value = match &after[..end] {
            "date" => Some(&ctx.date),
            "time" => Some(&ctx.time),
            "weekday" => Some(&ctx.weekday),
            "year" => Some(&ctx.year),
            "month" => Some(&ctx.month),
            "day" => Some(&ctx.day),
            "clipboard" => Some(&ctx.clipboard),
            _ => None,
        };
        match value {
            Some(v) => {
                out.push_str(v);
                rest = &after[end + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Checks a name and a body as typed.
pub fn validate(name: &str, body: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Give the snippet a name.".into());
    }
    if name.chars().count() > MAX_NAME {
        return Err(format!(
            "The name is too long ({MAX_NAME} characters at most)."
        ));
    }
    if body.trim().is_empty() {
        return Err("Write the text of the snippet.".into());
    }
    if body.chars().count() > MAX_BODY {
        return Err("That text is too long for a snippet.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context {
            date: "2026-11-19".into(),
            time: "14:30".into(),
            weekday: "Thursday".into(),
            year: "2026".into(),
            month: "11".into(),
            day: "19".into(),
            clipboard: "copied text".into(),
        }
    }

    #[test]
    fn known_values_are_filled_in() {
        assert_eq!(
            expand(
                "Hello! Today is {weekday}, {date} at {time}. Re: {clipboard}",
                &ctx()
            ),
            "Hello! Today is Thursday, 2026-11-19 at 14:30. Re: copied text"
        );
        assert_eq!(expand("{day}/{month}/{year}", &ctx()), "19/11/2026");
    }

    #[test]
    fn other_braces_stay_as_they_are() {
        assert_eq!(expand("a {b} c { d", &ctx()), "a {b} c { d");
        assert_eq!(
            expand("json: {\"a\": 1} {date}", &ctx()),
            "json: {\"a\": 1} 2026-11-19"
        );
        assert_eq!(expand("open {", &ctx()), "open {");
        assert_eq!(expand("", &ctx()), "");
    }

    #[test]
    fn a_value_is_not_filled_in_again() {
        let mut c = ctx();
        c.clipboard = "{date} {clipboard}".into();
        assert_eq!(expand("[{clipboard}]", &c), "[{date} {clipboard}]");
    }

    #[test]
    fn the_checks_catch_the_usual_mistakes() {
        assert!(validate("Sign off", "Best regards").is_ok());
        assert!(validate("  ", "x").is_err());
        assert!(validate("x", "   ").is_err());
        assert!(validate(&"n".repeat(61), "x").is_err());
        assert!(validate("x", &"y".repeat(20_001)).is_err());
    }
}
