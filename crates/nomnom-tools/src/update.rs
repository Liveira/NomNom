//! NomNom's own updates. NomNom looks at the latest published release on its GitHub page, and only
//! when asked (or once a day if the user left that on). Nothing is downloaded or installed until the
//! user presses the button. The installer is checked against the SHA-256 listed in the same release.

use crate::install::{download_to, fetch_text};
use crate::{parse_checksum, verify_sha256, InstallProgress, ToolError};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Where releases are published.
pub const REPO: &str = "Liveira/NomNom";

fn net(e: impl std::fmt::Display) -> ToolError {
    ToolError::Network(e.to_string())
}

/// A version number like `1.2.3`, with an optional `-beta.1` part that sorts before the plain release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    parts: [u64; 3],
    pre: Option<String>,
}

impl Version {
    pub fn parse(text: &str) -> Option<Version> {
        let t = text.trim().trim_start_matches(['v', 'V']);
        let t = t.split('+').next()?;
        let (core, pre) = match t.split_once('-') {
            Some((c, p)) => (c, Some(p.to_string())),
            None => (t, None),
        };
        let mut parts = [0u64; 3];
        let mut n = 0;
        for piece in core.split('.') {
            if n == 3 {
                return None;
            }
            parts[n] = piece.parse().ok()?;
            n += 1;
        }
        (n == 3).then_some(Version { parts, pre })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        match self.parts.cmp(&other.parts) {
            Equal => match (&self.pre, &other.pre) {
                (None, None) => Equal,
                (Some(_), None) => Less,
                (None, Some(_)) => Greater,
                (Some(a), Some(b)) => a.cmp(b),
            },
            o => o,
        }
    }
}

/// An update that can be installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Available {
    pub version: String,
    pub notes: String,
    pub page: String,
    pub file_name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Check {
    UpToDate { current: String, latest: String },
    Available(Available),
}

fn is_release_asset(url: &str) -> bool {
    let prefix = format!("https://github.com/{REPO}/releases/download/");
    url.starts_with(&prefix) && !url.contains("..")
}

/// Reads the answer of GitHub's "latest release" call. Split from the network call so it can be tested.
pub fn evaluate(
    current: &str,
    release: &str,
    sums: impl Fn(&str) -> Option<String>,
) -> Result<Check, ToolError> {
    let cur = Version::parse(current).ok_or_else(|| {
        net(format!(
            "this copy's version \"{current}\" isn't a version number"
        ))
    })?;
    let v: serde_json::Value = serde_json::from_str(release)
        .map_err(|e| net(format!("GitHub's answer couldn't be read: {e}")))?;
    let tag = v["tag_name"].as_str().unwrap_or_default();
    let latest = Version::parse(tag).ok_or_else(|| {
        net(format!(
            "the latest release is named \"{tag}\", which isn't a version number"
        ))
    })?;
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) {
        return Ok(Check::UpToDate {
            current: current.into(),
            latest: current.into(),
        });
    }
    if latest <= cur {
        return Ok(Check::UpToDate {
            current: current.into(),
            latest: tag.trim_start_matches(['v', 'V']).into(),
        });
    }
    let assets = v["assets"].as_array().cloned().unwrap_or_default();
    let installer = assets.iter().find(|a| {
        let n = a["name"].as_str().unwrap_or_default().to_ascii_lowercase();
        n.ends_with("-setup.exe") || n.ends_with("_setup.exe")
    });
    let Some(installer) = installer else {
        return Err(net(format!("version {tag} has no installer attached yet")));
    };
    let name = installer["name"].as_str().unwrap_or_default().to_string();
    let url = installer["browser_download_url"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if !is_release_asset(&url) {
        return Err(net(
            "the installer isn't hosted on NomNom's release page, so it was not used",
        ));
    }
    let sha256 = sums(&name).ok_or_else(|| {
        net(format!(
            "version {tag} has no checksum for {name}, so it was not offered"
        ))
    })?;
    Ok(Check::Available(Available {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        notes: v["body"].as_str().unwrap_or_default().trim().to_string(),
        page: v["html_url"].as_str().unwrap_or_default().to_string(),
        file_name: name,
        url,
        size: installer["size"].as_u64().unwrap_or(0),
        sha256,
    }))
}

fn client() -> Result<reqwest::Client, ToolError> {
    reqwest::Client::builder()
        .user_agent(concat!("NomNom/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(60 * 30))
        .build()
        .map_err(net)
}

/// Looks at the latest published release. Drafts and pre-releases are never offered.
pub async fn check(current: &str) -> Result<Check, ToolError> {
    let client = client()?;
    let body = fetch_text(
        &client,
        &format!("https://api.github.com/repos/{REPO}/releases/latest"),
    )
    .await
    .map_err(|e| match e {
        ToolError::Network(m) if m.contains("404") => net("no release has been published yet"),
        other => other,
    })?;
    // The checksum file is only fetched when there is something newer.
    let probe = evaluate(current, &body, |_| Some("0".repeat(64)));
    match probe {
        Ok(Check::Available(_)) => {
            let sums_url = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| {
                    v["assets"].as_array()?.iter().find_map(|a| {
                        if a["name"].as_str()? == "SHA256SUMS.txt" {
                            a["browser_download_url"].as_str().map(str::to_string)
                        } else {
                            None
                        }
                    })
                });
            let listing = match sums_url.filter(|u| is_release_asset(u)) {
                Some(u) => fetch_text(&client, &u).await?,
                None => String::new(),
            };
            evaluate(current, &body, |n| parse_checksum(&listing, n))
        }
        other => other,
    }
}

/// Downloads the installer into `dir` and checks it. Returns the path of the verified file.
pub async fn download(
    update: &Available,
    dir: &Path,
    progress: &InstallProgress,
) -> Result<PathBuf, ToolError> {
    if !is_release_asset(&update.url) {
        return Err(net(
            "the installer isn't hosted on NomNom's release page, so it was not used",
        ));
    }
    let name = Path::new(&update.file_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if name.is_empty() || name != update.file_name || !name.to_ascii_lowercase().ends_with(".exe") {
        return Err(net(
            "the installer has an unexpected name, so it was not used",
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| ToolError::Io(e.to_string()))?;
    let part = dir.join(format!("{name}.part"));
    let done = dir.join(name);
    let _ = std::fs::remove_file(&done);
    let client = client()?;
    download_to(&client, &update.url, &part, progress).await?;
    match verify_sha256(&part, &update.sha256) {
        Ok(true) => {
            std::fs::rename(&part, &done).map_err(|e| ToolError::Io(e.to_string()))?;
            Ok(done)
        }
        Ok(false) => {
            let _ = std::fs::remove_file(&part);
            Err(ToolError::ChecksumMismatch)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn versions_compare_by_number_not_text() {
        assert!(v("0.10.0") > v("0.9.9"));
        assert!(v("v1.0.0") > v("0.99.99"));
        assert!(v("1.2.3") == v("1.2.3"));
        assert!(v("1.0.0-beta.1") < v("1.0.0"));
        assert!(Version::parse("1.2").is_none());
        assert!(Version::parse("1.2.3.4").is_none());
        assert!(Version::parse("latest").is_none());
    }

    fn release(tag: &str, url: &str) -> String {
        serde_json::json!({
            "tag_name": tag, "draft": false, "prerelease": false, "body": "Fixes things.",
            "html_url": format!("https://github.com/{REPO}/releases/tag/{tag}"),
            "assets": [{"name": "NOMNOM_0.2.0_x64-setup.exe", "browser_download_url": url, "size": 1234}]
        })
        .to_string()
    }
    const GOOD: &str =
        "https://github.com/Liveira/NomNom/releases/download/v0.2.0/NOMNOM_0.2.0_x64-setup.exe";

    #[test]
    fn newer_release_is_offered_with_its_checksum() {
        let sum = "a".repeat(64);
        let r = evaluate("0.1.0", &release("v0.2.0", GOOD), |_| Some(sum.clone())).unwrap();
        let Check::Available(a) = r else {
            panic!("expected an update")
        };
        assert_eq!(a.version, "0.2.0");
        assert_eq!(a.sha256, sum);
        assert_eq!(a.size, 1234);
    }

    #[test]
    fn same_or_older_release_is_not_offered() {
        let same = evaluate("0.2.0", &release("v0.2.0", GOOD), |_| Some("a".repeat(64))).unwrap();
        assert!(matches!(same, Check::UpToDate { .. }));
        let older = evaluate("0.3.0", &release("v0.2.0", GOOD), |_| Some("a".repeat(64))).unwrap();
        assert!(matches!(older, Check::UpToDate { .. }));
    }

    #[test]
    fn no_checksum_means_no_offer() {
        assert!(evaluate("0.1.0", &release("v0.2.0", GOOD), |_| None).is_err());
    }

    #[test]
    fn installer_from_somewhere_else_is_refused() {
        let bad = "https://example.com/NOMNOM_0.2.0_x64-setup.exe";
        assert!(evaluate("0.1.0", &release("v0.2.0", bad), |_| Some("a".repeat(64))).is_err());
        let sneaky = "https://github.com/Liveira/NomNom/releases/download/../../evil/x-setup.exe";
        assert!(evaluate("0.1.0", &release("v0.2.0", sneaky), |_| Some(
            "a".repeat(64)
        ))
        .is_err());
    }

    #[test]
    fn drafts_and_prereleases_are_ignored() {
        let mut j: serde_json::Value = serde_json::from_str(&release("v0.2.0", GOOD)).unwrap();
        j["prerelease"] = true.into();
        let r = evaluate("0.1.0", &j.to_string(), |_| Some("a".repeat(64))).unwrap();
        assert!(matches!(r, Check::UpToDate { .. }));
    }

    #[test]
    fn release_without_installer_is_an_error() {
        let j = serde_json::json!({"tag_name": "v0.2.0", "assets": []}).to_string();
        assert!(evaluate("0.1.0", &j, |_| Some("a".repeat(64))).is_err());
    }

    #[test]
    fn download_refuses_odd_names_and_hosts() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut a = Available {
            version: "0.2.0".into(),
            notes: String::new(),
            page: String::new(),
            file_name: "..\\evil.exe".into(),
            url: GOOD.into(),
            size: 0,
            sha256: "a".repeat(64),
        };
        assert!(rt.block_on(download(&a, dir.path(), &|_, _| {})).is_err());
        a.file_name = "ok-setup.exe".into();
        a.url = "https://example.com/ok-setup.exe".into();
        assert!(rt.block_on(download(&a, dir.path(), &|_, _| {})).is_err());
    }
}
