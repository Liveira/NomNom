//! Clipboard history: what is worth keeping, how an entry is named in a list, and when it expires.
//! Only text is kept, and only on this computer.

/// The biggest text kept (characters). A whole book copied by accident is not history.
pub const MAX_CHARS: usize = 100_000;

/// A fixed fingerprint of a text (FNV-1a, 64 bits), so the same text is one entry however often it
/// is copied. It must never change between versions, so it is written out here.
pub fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The text to keep, or `None` when it isn't worth keeping (empty, only spaces, or far too long).
pub fn worth_keeping(text: &str) -> Option<&str> {
    if text.trim().is_empty() || text.chars().count() > MAX_CHARS {
        return None;
    }
    Some(text)
}

/// One line of at most `max` characters, for a list: new lines and runs of spaces become one space.
pub fn one_line(text: &str, max: usize) -> String {
    let mut out = String::new();
    let mut count = 0;
    let mut last_space = true;
    for c in text.chars() {
        let c = if c.is_whitespace() { ' ' } else { c };
        if c == ' ' && last_space {
            continue;
        }
        last_space = c == ' ';
        if count == max {
            out.push('…');
            return out;
        }
        out.push(c);
        count += 1;
    }
    out.trim_end().to_string()
}

/// Entries last used before this time (seconds since 1970) are old enough to forget, or `None`
/// when they are kept forever (`keep_days` of 0).
pub fn expires_before(now: i64, keep_days: u32) -> Option<i64> {
    (keep_days > 0).then(|| now - i64::from(keep_days) * 86_400)
}

/// A search for `query` in SQL `LIKE` form, with the characters that mean something escaped
/// (the query uses `ESCAPE '\\'`).
pub fn like_pattern(query: &str) -> String {
    let mut out = String::from("%");
    for c in query.trim().chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_text_has_the_same_fingerprint_forever() {
        assert_eq!(fingerprint("hello"), fingerprint("hello"));
        assert_ne!(fingerprint("hello"), fingerprint("Hello"));
        // A fixed value: if this changes, every saved entry would look new.
        assert_eq!(fingerprint(""), "cbf29ce484222325");
        assert_eq!(fingerprint("a"), "af63dc4c8601ec8c");
    }

    #[test]
    fn empty_and_huge_texts_are_not_kept() {
        assert_eq!(worth_keeping("  \n\t "), None);
        assert_eq!(worth_keeping(""), None);
        assert_eq!(
            worth_keeping("a link https://x.test"),
            Some("a link https://x.test")
        );
        assert_eq!(worth_keeping(&"x".repeat(MAX_CHARS + 1)), None);
        assert!(worth_keeping(&"x".repeat(MAX_CHARS)).is_some());
    }

    #[test]
    fn a_list_shows_one_short_line() {
        assert_eq!(
            one_line("line one\n\n  line   two", 40),
            "line one line two"
        );
        assert_eq!(one_line("abcdefghij", 5), "abcde…");
        assert_eq!(one_line("abcde", 5), "abcde");
        assert_eq!(one_line("  padded  ", 20), "padded");
    }

    #[test]
    fn entries_expire_only_when_a_limit_is_set() {
        assert_eq!(expires_before(1_000_000, 0), None);
        assert_eq!(expires_before(1_000_000, 1), Some(1_000_000 - 86_400));
        assert_eq!(
            expires_before(10_000_000, 30),
            Some(10_000_000 - 30 * 86_400)
        );
    }

    #[test]
    fn a_search_cannot_use_like_wildcards() {
        assert_eq!(like_pattern(" cat "), "%cat%");
        assert_eq!(like_pattern("100%_x"), "%100\\%\\_x%");
        assert_eq!(like_pattern("a\\b"), "%a\\\\b%");
    }
}
