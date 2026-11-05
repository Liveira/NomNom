//! Plain-file downloads for images and clips that sites serve directly (X, Instagram, Reddit).
//! yt-dlp is for videos; this covers the rest. Only a fixed list of media hosts is ever contacted,
//! over https, including after redirects.

use crate::error::DlError;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const EXACT_HOSTS: &[&str] = &[
    "pbs.twimg.com",
    "video.twimg.com",
    "ton.twimg.com",
    "i.redd.it",
    "preview.redd.it",
    "external-preview.redd.it",
    "v.redd.it",
];
const SUFFIX_HOSTS: &[&str] = &[".cdninstagram.com", ".fbcdn.net"];

pub fn is_allowed_media_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    EXACT_HOSTS.contains(&h.as_str()) || SUFFIX_HOSTS.iter().any(|s| h.ends_with(s))
}

/// https only, a known media host, no credentials.
pub fn validate_media_url(input: &str) -> Result<Url, DlError> {
    let url = Url::parse(input.trim())
        .map_err(|_| DlError::InvalidUrl("That media link isn't valid.".into()))?;
    let ok = url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(is_allowed_media_host);
    if ok {
        Ok(url)
    } else {
        Err(DlError::InvalidUrl(
            "NomNom only downloads media from the site's own servers.".into(),
        ))
    }
}

/// The same media with a different size or format parameter is still the same picture, so
/// duplicates are detected on host and path only.
pub fn media_key(url: &Url) -> String {
    format!("media:{}{}", url.host_str().unwrap_or(""), url.path())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub path: PathBuf,
    pub content_type: String,
}

impl Fetched {
    pub fn is_video(&self) -> bool {
        self.content_type.starts_with("video/")
    }
}

pub fn extension_for(content_type: &str, url: &Url) -> &'static str {
    match content_type.split(';').next().unwrap_or("").trim() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/avif" => "avif",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        _ => match url
            .path()
            .rsplit('.')
            .next()
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => "png",
            Some("webp") => "webp",
            Some("gif") => "gif",
            Some("mp4") => "mp4",
            Some("webm") => "webm",
            _ => "jpg",
        },
    }
}

#[async_trait]
pub trait FileFetcher: Send + Sync {
    /// Downloads `url` into `staging` and returns the file. `progress` gets (done, total).
    async fn fetch(
        &self,
        url: &Url,
        staging: &Path,
        stem: &str,
        progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Result<Fetched, DlError>;
}

pub struct HttpFetcher;

#[async_trait]
impl FileFetcher for HttpFetcher {
    async fn fetch(
        &self,
        url: &Url,
        staging: &Path,
        stem: &str,
        progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Result<Fetched, DlError> {
        use std::io::Write;
        let policy = reqwest::redirect::Policy::custom(|attempt| {
            let ok = attempt.url().scheme() == "https"
                && attempt.url().host_str().is_some_and(is_allowed_media_host);
            if ok && attempt.previous().len() < 5 {
                attempt.follow()
            } else {
                attempt.error("redirected somewhere NomNom does not download from")
            }
        });
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) NomNom")
            .redirect(policy)
            .connect_timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| DlError::Io(e.to_string()))?;
        let mut resp = client
            .get(url.clone())
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| DlError::Failed("The site didn't give the file.".into()))?;
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if !(content_type.starts_with("image/") || content_type.starts_with("video/")) {
            return Err(DlError::Failed(
                "That link isn't a picture or a video.".into(),
            ));
        }
        let total = resp.content_length();
        if total.is_some_and(|t| t > MAX_BYTES) {
            return Err(DlError::Failed("That file is too big (over 2 GB).".into()));
        }
        std::fs::create_dir_all(staging).map_err(|e| DlError::Io(e.to_string()))?;
        let path = staging.join(format!("{stem}.{}", extension_for(&content_type, url)));
        let mut file = std::fs::File::create(&path).map_err(|e| DlError::Io(e.to_string()))?;
        let mut done = 0u64;
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|_| DlError::Failed("The download was interrupted.".into()))?
        {
            done += chunk.len() as u64;
            if done > MAX_BYTES {
                return Err(DlError::Failed("That file is too big (over 2 GB).".into()));
            }
            file.write_all(&chunk)
                .map_err(|e| DlError::Io(e.to_string()))?;
            progress(done, total);
        }
        file.flush().map_err(|e| DlError::Io(e.to_string()))?;
        Ok(Fetched { path, content_type })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_sites_own_media_hosts_pass() {
        for ok in [
            "https://pbs.twimg.com/media/abc?format=jpg&name=orig",
            "https://video.twimg.com/ext_tw_video/1/vid/a.mp4",
            "https://i.redd.it/abc.jpg",
            "https://scontent-lhr8-1.cdninstagram.com/v/t51/x.jpg",
            "https://scontent.xx.fbcdn.net/v/x.jpg",
        ] {
            assert!(validate_media_url(ok).is_ok(), "{ok}");
        }
        for bad in [
            "http://pbs.twimg.com/media/abc.jpg",
            "https://evil.example/pbs.twimg.com/a.jpg",
            "https://pbs.twimg.com.evil.example/a.jpg",
            "https://notcdninstagram.com/a.jpg",
            "https://user:pw@pbs.twimg.com/a.jpg",
            "file:///C:/Windows/win.ini",
            "https://127.0.0.1/a.jpg",
            "https://twimg.com/a.jpg",
            "not a url",
        ] {
            assert!(validate_media_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn same_picture_in_different_sizes_has_one_key() {
        let a = Url::parse("https://pbs.twimg.com/media/abc?format=jpg&name=small").unwrap();
        let b = Url::parse("https://pbs.twimg.com/media/abc?format=jpg&name=orig").unwrap();
        let c = Url::parse("https://pbs.twimg.com/media/other?format=jpg").unwrap();
        assert_eq!(media_key(&a), media_key(&b));
        assert_ne!(media_key(&a), media_key(&c));
    }

    #[test]
    fn extension_comes_from_the_content_type_first() {
        let u = Url::parse("https://pbs.twimg.com/media/abc?format=png").unwrap();
        assert_eq!(extension_for("image/jpeg", &u), "jpg");
        assert_eq!(extension_for("image/webp; charset=x", &u), "webp");
        assert_eq!(extension_for("video/mp4", &u), "mp4");
        let by_path = Url::parse("https://i.redd.it/abc.gif").unwrap();
        assert_eq!(extension_for("application/octet-stream", &by_path), "gif");
        assert_eq!(extension_for("", &u), "jpg");
    }

    #[test]
    fn video_content_is_recognised() {
        assert!(Fetched {
            path: "a".into(),
            content_type: "video/mp4".into()
        }
        .is_video());
        assert!(!Fetched {
            path: "a".into(),
            content_type: "image/gif".into()
        }
        .is_video());
    }
}
