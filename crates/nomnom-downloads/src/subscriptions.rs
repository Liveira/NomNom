//! Following a channel or a playlist: reading what yt-dlp lists, telling which videos are new, and
//! deciding when a subscription is due for another look. No network and no database here.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The most videos looked at in one check, and the most started from one check.
pub const LOOK_AT: usize = 30;
pub const START_AT_MOST: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub url: String,
    pub title: String,
}

/// The videos in the answer of `yt-dlp -J --flat-playlist`, newest first (as sites list them). A
/// link to one video (not a list) gives nothing, so it can't be followed by mistake.
pub fn parse_entries(json: &str) -> Vec<Entry> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(list) = v.get("entries").and_then(|e| e.as_array()) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    list.iter()
        .filter_map(entry_of)
        .filter(|e| seen.insert(e.id.clone()))
        .take(LOOK_AT)
        .collect()
}

fn entry_of(e: &serde_json::Value) -> Option<Entry> {
    let id = e.get("id")?.as_str()?.to_string();
    if id.is_empty() {
        return None;
    }
    let raw = e.get("url").and_then(|u| u.as_str()).unwrap_or("");
    let url = if raw.starts_with("https://") || raw.starts_with("http://") {
        raw.to_string()
    } else if e
        .get("ie_key")
        .and_then(|k| k.as_str())
        .is_some_and(|k| k.eq_ignore_ascii_case("youtube"))
        || raw.is_empty()
    {
        format!("https://www.youtube.com/watch?v={id}")
    } else {
        return None;
    };
    let title = e
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    Some(Entry { id, url, title })
}

/// The name of the channel or playlist, when the answer has one.
pub fn parse_title(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let t = v.get("title").and_then(|t| t.as_str())?.trim().to_string();
    (!t.is_empty()).then_some(t)
}

/// The videos not seen before, oldest first (so they are downloaded in the order they came out),
/// at most `START_AT_MOST`.
pub fn fresh(entries: &[Entry], seen: &HashSet<String>) -> Vec<Entry> {
    let mut new: Vec<Entry> = entries
        .iter()
        .filter(|e| !seen.contains(&e.id))
        .cloned()
        .collect();
    new.reverse();
    // When there is a flood (a first check, a long break), the newest ones are the ones wanted.
    if new.len() > START_AT_MOST {
        new.drain(..new.len() - START_AT_MOST);
    }
    new
}

/// Is it time to look again? `every_hours` is clamped to 1..=168.
pub fn due(last_checked: Option<i64>, every_hours: u32, now: i64) -> bool {
    let every = i64::from(every_hours.clamp(1, 168)) * 3600;
    last_checked.is_none_or(|t| now - t >= every)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"{"title":"Brad's Art School","entries":[
        {"id":"c3","url":"https://www.youtube.com/watch?v=c3","title":"Third","ie_key":"Youtube"},
        {"id":"b2","url":"https://www.youtube.com/watch?v=b2","title":"Second"},
        {"id":"a1","title":"First","ie_key":"Youtube"},
        {"id":"b2","url":"https://www.youtube.com/watch?v=b2","title":"Second again"},
        {"id":"","url":"https://x"},
        {"id":"z9","url":"/relative","title":"odd"}
    ]}"#;

    #[test]
    fn the_listed_videos_are_read_once_each_with_a_usable_link() {
        let got = parse_entries(LIST);
        let ids: Vec<_> = got.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["c3", "b2", "a1"]);
        assert_eq!(got[2].url, "https://www.youtube.com/watch?v=a1");
        assert_eq!(parse_title(LIST).as_deref(), Some("Brad's Art School"));
    }

    #[test]
    fn one_video_or_nonsense_cannot_be_followed() {
        assert!(parse_entries(r#"{"id":"x","title":"One video"}"#).is_empty());
        assert!(parse_entries("not json").is_empty());
        assert_eq!(parse_title("{}"), None);
    }

    #[test]
    fn only_unseen_videos_start_and_the_oldest_goes_first() {
        let entries = parse_entries(LIST);
        let seen: HashSet<String> = ["a1".to_string()].into();
        let new = fresh(&entries, &seen);
        assert_eq!(
            new.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["b2", "c3"]
        );
        assert!(fresh(
            &entries,
            &["a1", "b2", "c3"].iter().map(|s| s.to_string()).collect()
        )
        .is_empty());
    }

    #[test]
    fn a_flood_is_cut_to_the_newest_ones() {
        let entries: Vec<Entry> = (0..25)
            .map(|i| Entry {
                id: format!("v{i}"),
                url: format!("https://x/{i}"),
                title: String::new(),
            })
            .collect(); // v0 is the newest
        let new = fresh(&entries, &HashSet::new());
        assert_eq!(new.len(), START_AT_MOST);
        assert_eq!(
            new.last().map(|e| e.id.as_str()),
            Some("v0"),
            "the newest is last, so it starts last"
        );
        assert_eq!(new.first().map(|e| e.id.as_str()), Some("v9"));
    }

    #[test]
    fn a_subscription_is_due_when_it_never_ran_or_the_wait_is_over() {
        assert!(due(None, 6, 1_000));
        assert!(!due(Some(1_000), 6, 1_000 + 5 * 3600));
        assert!(due(Some(1_000), 6, 1_000 + 6 * 3600));
        assert!(
            due(Some(0), 0, 3600),
            "never faster than hourly: 0 counts as 1"
        );
        assert!(!due(Some(0), 0, 3599));
    }
}
