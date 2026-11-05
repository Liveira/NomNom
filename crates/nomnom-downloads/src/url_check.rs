//! URL validation, supported-site gating and duplicate keys.

use crate::error::DlError;
use std::net::IpAddr;
use url::{Host, Url};

const MAX_LEN: usize = 2048;

/// Sites worth opening the quick popup for. This is only a gate for the shortcut: the real answer
/// always comes from yt-dlp, so the main app accepts any valid web URL.
const KNOWN_SITES: &[&str] = &[
    "youtube.com",
    "youtu.be",
    "x.com",
    "twitter.com",
    "instagram.com",
    "reddit.com",
    "redd.it",
    "tiktok.com",
    "vimeo.com",
    "twitch.tv",
    "dailymotion.com",
    "facebook.com",
    "fb.watch",
    "soundcloud.com",
    "bilibili.com",
    "streamable.com",
    "imgur.com",
    "bsky.app",
    "tumblr.com",
    "pinterest.com",
];

pub fn validate_url(input: &str) -> Result<Url, DlError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DlError::InvalidUrl("Paste a link first.".into()));
    }
    if input.len() > MAX_LEN {
        return Err(DlError::InvalidUrl("That link is too long.".into()));
    }
    let url = Url::parse(input)
        .map_err(|_| DlError::InvalidUrl("That doesn't look like a link.".into()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(DlError::InvalidUrl(
            "Only http and https links work.".into(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(DlError::InvalidUrl(
            "Links with a username or password aren't allowed.".into(),
        ));
    }
    match url.host() {
        None => {
            return Err(DlError::InvalidUrl(
                "That link has no website in it.".into(),
            ))
        }
        Some(Host::Domain(d)) => {
            let d = d.to_ascii_lowercase();
            if d == "localhost" || d.ends_with(".localhost") || d.ends_with(".local") {
                return Err(DlError::InvalidUrl(
                    "Links to this computer aren't allowed.".into(),
                ));
            }
        }
        Some(Host::Ipv4(ip)) if is_private(IpAddr::V4(ip)) => {
            return Err(DlError::InvalidUrl(
                "Links to local networks aren't allowed.".into(),
            ))
        }
        Some(Host::Ipv6(ip)) if is_private(IpAddr::V6(ip)) => {
            return Err(DlError::InvalidUrl(
                "Links to local networks aren't allowed.".into(),
            ))
        }
        _ => {}
    }
    Ok(url)
}

fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

fn host_of(url: &Url) -> String {
    url.host_str().unwrap_or("").to_ascii_lowercase()
}

/// Does the link look like it belongs to a site yt-dlp commonly handles?
pub fn is_probably_supported(url: &Url) -> bool {
    let host = host_of(url);
    KNOWN_SITES
        .iter()
        .any(|site| host == *site || host.ends_with(&format!(".{site}")))
}

/// Short name for the site, for display.
pub fn site_label(url: &Url) -> String {
    let host = host_of(url);
    let host = host.trim_start_matches("www.").trim_start_matches("m.");
    match host {
        "youtu.be" => "YouTube".into(),
        h if h.ends_with("youtube.com") => "YouTube".into(),
        "x.com" | "twitter.com" => "X".into(),
        h if h.ends_with("instagram.com") => "Instagram".into(),
        h if h.ends_with("reddit.com") || h == "redd.it" => "Reddit".into(),
        h if h.ends_with("tiktok.com") => "TikTok".into(),
        h if h.ends_with("vimeo.com") => "Vimeo".into(),
        h => h.to_string(),
    }
}

fn is_tracking(key: &str) -> bool {
    key.starts_with("utm_")
        || matches!(
            key,
            "si" | "feature" | "t" | "fbclid" | "igshid" | "ref" | "s" | "pp" | "app"
        )
}

/// Stable key for duplicate detection: the same video behind different links maps to one key.
pub fn canonical_key(url: &Url) -> String {
    let host = host_of(url);
    let host = host
        .trim_start_matches("www.")
        .trim_start_matches("m.")
        .to_string();
    let query = |name: &str| {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    let segments: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|p| !p.is_empty()).collect())
        .unwrap_or_default();

    if host == "youtu.be" {
        if let Some(id) = segments.first() {
            return format!("youtube:{id}");
        }
    }
    if host.ends_with("youtube.com") {
        if let Some(id) = query("v") {
            return format!("youtube:{id}");
        }
        if let ["shorts" | "live" | "embed", id, ..] = segments.as_slice() {
            return format!("youtube:{id}");
        }
    }

    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !is_tracking(k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();
    let q: Vec<String> = pairs.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let path = segments.join("/");
    if q.is_empty() {
        format!("{host}/{path}")
    } else {
        format!("{host}/{path}?{}", q.join("&"))
    }
}

/// Why the clipboard text can't start a quick download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardReject {
    Empty,
    NotALink,
    UnknownSite,
}

impl ClipboardReject {
    pub fn message(self) -> &'static str {
        match self {
            ClipboardReject::Empty => "The clipboard is empty.",
            ClipboardReject::NotALink => "There's no link on the clipboard.",
            ClipboardReject::UnknownSite => "This link isn't from a site NOMNOM recognises.",
        }
    }
}

/// Decides whether clipboard text should open the quick-download popup. Only a single, valid
/// link to a known media site passes; anything else is ignored so the shortcut stays quiet.
pub fn check_clipboard(text: &str) -> Result<Url, ClipboardReject> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ClipboardReject::Empty);
    }
    if text.contains(char::is_whitespace) {
        return Err(ClipboardReject::NotALink);
    }
    let url = validate_url(text).map_err(|_| ClipboardReject::NotALink)?;
    if is_probably_supported(&url) {
        Ok(url)
    } else {
        Err(ClipboardReject::UnknownSite)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> Url {
        validate_url(s).unwrap()
    }

    #[test]
    fn accepts_normal_links_and_trims_whitespace() {
        assert_eq!(
            ok("  https://www.youtube.com/watch?v=abc  ").host_str(),
            Some("www.youtube.com")
        );
    }

    #[test]
    fn rejects_bad_input() {
        for bad in [
            "",
            "not a url",
            "file:///C:/Windows/win.ini",
            "ftp://example.com/x",
            "javascript:alert(1)",
            "https://user:pw@example.com/",
            "http://localhost:8080/",
            "http://127.0.0.1/x",
            "http://192.168.1.10/x",
            "http://10.0.0.1/x",
            "http://[::1]/x",
            "http://printer.local/x",
        ] {
            assert!(validate_url(bad).is_err(), "should reject {bad:?}");
        }
        assert!(validate_url(&format!("https://example.com/{}", "a".repeat(3000))).is_err());
    }

    #[test]
    fn detects_known_sites_including_subdomains() {
        assert!(is_probably_supported(&ok(
            "https://m.youtube.com/watch?v=1"
        )));
        assert!(is_probably_supported(&ok("https://youtu.be/abc")));
        assert!(is_probably_supported(&ok("https://x.com/a/status/1")));
        assert!(!is_probably_supported(&ok("https://example.com/video")));
        assert!(
            !is_probably_supported(&ok("https://notyoutube.com/x")),
            "suffix must be a whole label"
        );
    }

    #[test]
    fn same_video_maps_to_same_key() {
        let a = canonical_key(&ok(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL1&si=xyz",
        ));
        let b = canonical_key(&ok("https://youtu.be/dQw4w9WgXcQ?si=abc"));
        let c = canonical_key(&ok("https://youtube.com/shorts/dQw4w9WgXcQ"));
        assert_eq!(a, "youtube:dQw4w9WgXcQ");
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn tracking_params_are_ignored_for_other_sites() {
        let a = canonical_key(&ok("https://x.com/u/status/1?utm_source=a&s=20"));
        let b = canonical_key(&ok("https://www.x.com/u/status/1/"));
        assert_eq!(a, b);
        assert_ne!(a, canonical_key(&ok("https://x.com/u/status/2")));
    }
}

#[cfg(test)]
mod clipboard_tests {
    use super::*;

    #[test]
    fn accepts_a_single_known_link() {
        assert!(check_clipboard("  https://youtu.be/abc\r\n").is_ok());
        assert!(check_clipboard("https://x.com/a/status/1").is_ok());
    }

    #[test]
    fn rejects_everything_else_quietly() {
        assert_eq!(check_clipboard(""), Err(ClipboardReject::Empty));
        assert_eq!(check_clipboard("   \n"), Err(ClipboardReject::Empty));
        assert_eq!(
            check_clipboard("hello world"),
            Err(ClipboardReject::NotALink)
        );
        assert_eq!(
            check_clipboard("see https://youtu.be/abc"),
            Err(ClipboardReject::NotALink)
        );
        assert_eq!(
            check_clipboard(r"C:\Users\me\file.txt"),
            Err(ClipboardReject::NotALink)
        );
        assert_eq!(
            check_clipboard("file:///C:/x.mp4"),
            Err(ClipboardReject::NotALink)
        );
        assert_eq!(
            check_clipboard("http://localhost/x"),
            Err(ClipboardReject::NotALink)
        );
        assert_eq!(
            check_clipboard("https://example.com/video"),
            Err(ClipboardReject::UnknownSite)
        );
    }

    #[test]
    fn multiple_lines_are_not_a_link() {
        assert_eq!(
            check_clipboard("https://youtu.be/a\nhttps://youtu.be/b"),
            Err(ClipboardReject::NotALink)
        );
    }
}
