//! Cleaning a link: the parts that only say where you came from (utm_source, fbclid and the like)
//! are taken off, so the link can be shared or kept as the page itself. Nothing is fetched.

/// Parameters that are only tracking, wherever they appear.
const TRACKING: &[&str] = &[
    "fbclid",
    "gclid",
    "gclsrc",
    "dclid",
    "msclkid",
    "yclid",
    "igshid",
    "igsh",
    "mc_cid",
    "mc_eid",
    "_hsenc",
    "_hsmi",
    "mkt_tok",
    "ref_src",
    "ref_url",
    "twclid",
    "ttclid",
    "li_fat_id",
    "vero_id",
    "wickedid",
    "oly_enc_id",
    "oly_anon_id",
    "__s",
    "s_cid",
    "spm",
    "scm",
    "_ga",
    "_gl",
];

/// Parameters that are only tracking on some sites (they mean something elsewhere).
fn site_tracking(host: &str, name: &str) -> bool {
    let h = host.trim_start_matches("www.");
    match name {
        "si" => matches!(
            h,
            "youtu.be" | "youtube.com" | "music.youtube.com" | "open.spotify.com" | "spotify.link"
        ),
        "feature" => matches!(h, "youtu.be" | "youtube.com"),
        "pp" => matches!(h, "youtu.be" | "youtube.com"),
        "t" | "s" => matches!(h, "twitter.com" | "x.com"),
        "ref" | "ref_" | "tag" | "linkCode" | "psc" | "pf_rd_r" | "pf_rd_p" | "pd_rd_r"
        | "pd_rd_w" | "pd_rd_wg" | "content-id" => h.contains("amazon."),
        "share_id" | "share_link_id" => matches!(h, "reddit.com" | "old.reddit.com"),
        "utm_name" | "utm_id" => true,
        _ => false,
    }
}

fn is_tracking(host: &str, name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.starts_with("utm_") || TRACKING.contains(&n.as_str()) || site_tracking(host, name)
}

/// The link without tracking, or `None` when it is not a web link or there was nothing to take off.
pub fn clean(text: &str) -> Option<String> {
    let t = text.trim();
    if t.contains(char::is_whitespace) || !(t.starts_with("http://") || t.starts_with("https://")) {
        return None;
    }
    let (before_fragment, fragment) = match t.split_once('#') {
        Some((a, b)) => (a, Some(b)),
        None => (t, None),
    };
    let (base, query) = before_fragment.split_once('?')?;
    let host = base
        .split_once("://")?
        .1
        .split(['/', ':'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| !p.is_empty())
        .filter(|p| !is_tracking(&host, p.split('=').next().unwrap_or("")))
        .collect();
    let total = query.split('&').filter(|p| !p.is_empty()).count();
    if kept.len() == total {
        return None;
    }
    let mut out = base.to_string();
    if !kept.is_empty() {
        out.push('?');
        out.push_str(&kept.join("&"));
    }
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(f);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_and_click_ids_go_and_the_rest_stays() {
        assert_eq!(
            clean("https://example.com/a?id=7&utm_source=x&utm_medium=y&fbclid=abc&page=2#top")
                .unwrap(),
            "https://example.com/a?id=7&page=2#top"
        );
    }

    #[test]
    fn a_link_that_is_all_tracking_loses_the_question_mark() {
        assert_eq!(
            clean("https://example.com/post?utm_campaign=z&gclid=1").unwrap(),
            "https://example.com/post"
        );
    }

    #[test]
    fn a_clean_link_or_not_a_link_gives_nothing() {
        assert!(clean("https://example.com/a?id=7").is_none());
        assert!(clean("https://example.com/a").is_none());
        assert!(clean("hello world").is_none());
        assert!(clean("ftp://x.org/?utm_source=a").is_none());
        assert!(clean("https://a.com/?utm_source=a and text").is_none());
        assert!(clean("").is_none());
    }

    #[test]
    fn site_specific_ones_only_on_those_sites() {
        assert_eq!(
            clean("https://youtu.be/abc?si=XYZ&t=10").unwrap(),
            "https://youtu.be/abc?t=10"
        );
        assert_eq!(
            clean("https://www.youtube.com/watch?v=abc&si=XYZ&feature=share").unwrap(),
            "https://www.youtube.com/watch?v=abc"
        );
        // "si" means something on other sites.
        assert!(clean("https://example.com/?si=12").is_none());
        assert_eq!(
            clean("https://x.com/user/status/1?s=20&t=abc").unwrap(),
            "https://x.com/user/status/1"
        );
        assert_eq!(
            clean("https://www.amazon.com/dp/B0?ref=abc&psc=1&th=1").unwrap(),
            "https://www.amazon.com/dp/B0?th=1"
        );
    }

    #[test]
    fn names_are_matched_without_regard_to_case() {
        assert_eq!(
            clean("https://a.com/?UTM_Source=1&q=x").unwrap(),
            "https://a.com/?q=x"
        );
    }
}
