use serde::{Serialize, Serializer};

/// Everything that can go wrong with a download, with messages written for the user.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DlError {
    #[error("{0}")]
    InvalidUrl(String),
    #[error("This site isn't supported.")]
    Unsupported,
    #[error(
        "This video needs you to sign in. You can let NOMNOM use your Firefox cookies in Settings."
    )]
    NeedsLogin,
    #[error("This video isn't available in your region.")]
    Geo,
    #[error("This video is DRM-protected and can't be downloaded.")]
    Drm,
    #[error("{0} isn't installed yet.")]
    ToolMissing(&'static str),
    #[error("Couldn't download this video. {0}")]
    Failed(String),
    #[error("Cancelled.")]
    Cancelled,
    #[error("{0}")]
    Io(String),
}

impl Serialize for DlError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

/// Maps yt-dlp's stderr to a friendly error.
pub fn classify_stderr(stderr: &str) -> DlError {
    let s = stderr.to_ascii_lowercase();
    let has = |needle: &str| s.contains(needle);
    if has("unsupported url") {
        DlError::Unsupported
    } else if has("drm") && (has("protected") || has("encrypted")) {
        DlError::Drm
    } else if has("sign in")
        || has("log in")
        || has("login")
        || has("private video")
        || has("members-only")
        || has("cookies")
    {
        DlError::NeedsLogin
    } else if has("not available in your country") || has("geo restrict") || has("geo-restrict") {
        DlError::Geo
    } else {
        let last = stderr
            .lines()
            .rev()
            .find(|l| l.contains("ERROR"))
            .or_else(|| stderr.lines().next_back())
            .unwrap_or("")
            .trim();
        let last = last.trim_start_matches("ERROR:").trim();
        // Drop the trailing "(caused by ...)" detail; it only repeats the message.
        let last = last.split(" (caused by").next().unwrap_or(last).trim();
        DlError::Failed(last.chars().take(200).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_failures() {
        assert_eq!(
            classify_stderr("ERROR: Unsupported URL: https://x.test/"),
            DlError::Unsupported
        );
        assert_eq!(
            classify_stderr("ERROR: [youtube] Sign in to confirm"),
            DlError::NeedsLogin
        );
        assert_eq!(classify_stderr("ERROR: Private video"), DlError::NeedsLogin);
        assert_eq!(
            classify_stderr("This video is not available in your country"),
            DlError::Geo
        );
        assert_eq!(
            classify_stderr("ERROR: DRM protected content"),
            DlError::Drm
        );
    }

    #[test]
    fn unknown_failure_keeps_last_error_line() {
        let e = classify_stderr("noise\nERROR: HTTP Error 500\n");
        assert_eq!(e, DlError::Failed("HTTP Error 500".into()));
    }
}
