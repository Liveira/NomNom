use crate::{Tool, ToolError};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

/// Receives (bytes downloaded, total bytes when the server says).
pub type InstallProgress = dyn Fn(u64, Option<u64>) + Send + Sync;

const YTDLP_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const FFMPEG_BASE: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download";
const SCRCPY_LATEST_API: &str = "https://api.github.com/repos/Genymobile/scrcpy/releases/latest";

fn ytdlp_release_asset() -> &'static str {
    #[cfg(windows)]
    {
        "yt-dlp.exe"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "yt-dlp_linux_aarch64"
    }
    #[cfg(all(target_os = "linux", not(target_arch = "aarch64")))]
    {
        "yt-dlp_linux"
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        "yt-dlp"
    }
}

fn ffmpeg_release_archive() -> &'static str {
    #[cfg(windows)]
    {
        "ffmpeg-master-latest-win64-gpl.zip"
    }
    #[cfg(all(not(windows), target_arch = "aarch64"))]
    {
        "ffmpeg-master-latest-linuxarm64-gpl.tar.xz"
    }
    #[cfg(all(not(windows), not(target_arch = "aarch64")))]
    {
        "ffmpeg-master-latest-linux64-gpl.tar.xz"
    }
}

fn io(e: impl std::fmt::Display) -> ToolError {
    ToolError::Io(e.to_string())
}

fn net(e: impl std::fmt::Display) -> ToolError {
    ToolError::Network(e.to_string())
}

fn mark_executable(_path: &Path) -> Result<(), ToolError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(_path, std::fs::Permissions::from_mode(0o755)).map_err(io)?;
    }
    Ok(())
}

/// Finds the hash for `file_name` in a `sha256sum`-style listing ("<hex>  <name>" or "<hex> *<name>").
pub fn parse_checksum(listing: &str, file_name: &str) -> Option<String> {
    listing.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        let is_hex = hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit());
        (is_hex && name == file_name).then(|| hash.to_ascii_lowercase())
    })
}

pub fn verify_sha256(path: &Path, expected_hex: &str) -> Result<bool, ToolError> {
    let mut file = std::fs::File::open(path).map_err(io)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(actual.eq_ignore_ascii_case(expected_hex))
}

fn client() -> Result<reqwest::Client, ToolError> {
    reqwest::Client::builder()
        .user_agent("NOMNOM")
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(net)
}

pub(crate) async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, ToolError> {
    client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(net)?
        .text()
        .await
        .map_err(net)
}

pub(crate) async fn download_to(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    progress: &InstallProgress,
) -> Result<(), ToolError> {
    let mut resp = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(net)?;
    let total = resp.content_length();
    let mut file = std::fs::File::create(dest).map_err(io)?;
    let mut done = 0u64;
    while let Some(chunk) = resp.chunk().await.map_err(net)? {
        file.write_all(&chunk).map_err(io)?;
        done += chunk.len() as u64;
        progress(done, total);
    }
    file.flush().map_err(io)
}

pub(crate) async fn install_ytdlp(dir: &Path, progress: &InstallProgress) -> Result<(), ToolError> {
    std::fs::create_dir_all(dir).map_err(io)?;
    let client = client()?;
    let asset = ytdlp_release_asset();
    let local_name = Tool::YtDlp.exe_name();
    let sums = fetch_text(&client, &format!("{YTDLP_BASE}/SHA2-256SUMS")).await?;
    let expected = parse_checksum(&sums, asset)
        .ok_or_else(|| ToolError::Network(format!("the release has no checksum for {asset}")))?;

    let tmp = dir.join(format!("{local_name}.part"));
    download_to(&client, &format!("{YTDLP_BASE}/{asset}"), &tmp, progress).await?;
    if !verify_sha256(&tmp, &expected)? {
        let _ = std::fs::remove_file(&tmp);
        return Err(ToolError::ChecksumMismatch);
    }
    mark_executable(&tmp)?;
    std::fs::rename(&tmp, dir.join(local_name)).map_err(io)
}

pub(crate) async fn install_ffmpeg(
    dir: &Path,
    progress: &InstallProgress,
) -> Result<(), ToolError> {
    std::fs::create_dir_all(dir).map_err(io)?;
    let client = client()?;
    let archive_name = ffmpeg_release_archive();
    let sums = fetch_text(&client, &format!("{FFMPEG_BASE}/checksums.sha256")).await?;
    let expected = parse_checksum(&sums, archive_name).ok_or_else(|| {
        ToolError::Network("the release has no checksum for the ffmpeg archive".into())
    })?;

    let archive_path = dir.join(format!("{archive_name}.part"));
    download_to(
        &client,
        &format!("{FFMPEG_BASE}/{archive_name}"),
        &archive_path,
        progress,
    )
    .await?;
    let result = (|| {
        if !verify_sha256(&archive_path, &expected)? {
            return Err(ToolError::ChecksumMismatch);
        }
        extract_binaries(&archive_path, dir)
    })();
    let _ = std::fs::remove_file(&archive_path);
    result
}

pub(crate) async fn install_scrcpy(
    dir: &Path,
    progress: &InstallProgress,
) -> Result<(), ToolError> {
    std::fs::create_dir_all(dir).map_err(io)?;
    let client = client()?;
    let release_json = fetch_text(&client, SCRCPY_LATEST_API).await?;
    let release: serde_json::Value = serde_json::from_str(&release_json)
        .map_err(|e| ToolError::Network(format!("couldn't read scrcpy release info: {e}")))?;
    let assets = release
        .get("assets")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ToolError::Network("scrcpy release had no files listed".into()))?;

    #[cfg(windows)]
    let (prefix, suffix) = ("scrcpy-win64-", ".zip");
    #[cfg(not(windows))]
    let (prefix, suffix) = ("scrcpy-linux-x86_64-", ".tar.gz");

    let asset = assets
        .iter()
        .find(|a| {
            a.get("name")
                .and_then(|n| n.as_str())
                .is_some_and(|n| n.starts_with(prefix) && n.ends_with(suffix))
        })
        .ok_or_else(|| {
            ToolError::Network("couldn't find a matching scrcpy package for this system".into())
        })?;

    let asset_name = asset
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("scrcpy.zip");
    let download_url = asset
        .get("browser_download_url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| ToolError::Network("scrcpy package had no download link".into()))?;

    // Check optional published SHA256SUMS.txt if present in the same release.
    let expected_hash = if let Some(sum_url) = assets.iter().find_map(|a| {
        let n = a.get("name")?.as_str()?;
        (n == "SHA256SUMS.txt" || n == "SHA256SUMS")
            .then(|| a.get("browser_download_url")?.as_str())
            .flatten()
    }) {
        fetch_text(&client, sum_url)
            .await
            .ok()
            .and_then(|txt| parse_checksum(&txt, asset_name))
    } else {
        None
    };

    let archive_path = dir.join(format!("{asset_name}.part"));
    download_to(&client, download_url, &archive_path, progress).await?;

    let result = (|| {
        if let Some(ref hex) = expected_hash {
            if !verify_sha256(&archive_path, hex)? {
                return Err(ToolError::ChecksumMismatch);
            }
        }
        extract_scrcpy(&archive_path, dir)
    })();
    let _ = std::fs::remove_file(&archive_path);
    result
}

#[cfg(windows)]
fn extract_scrcpy(zip_path: &Path, dir: &Path) -> Result<(), ToolError> {
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(zip_path).map_err(io)?).map_err(io)?;
    let staging = dir.join(format!(".scrcpy-staging-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(io)?;

    let unpack = (|| -> Result<(), ToolError> {
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(io)?;
            if entry.is_dir() {
                continue;
            }
            let raw_name = entry.name().replace('\\', "/");
            // Strip leading folder (e.g. `scrcpy-win64-v3.1/adb.exe` -> `adb.exe`).
            let rel = match raw_name.split_once('/') {
                Some((_, rest)) if !rest.is_empty() => rest,
                _ => raw_name.as_str(),
            };
            if rel.contains("..") || rel.starts_with('/') || rel.contains(':') {
                continue;
            }
            let dest = staging.join(rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            let mut out = std::fs::File::create(&dest).map_err(io)?;
            std::io::copy(&mut entry, &mut out).map_err(io)?;
        }
        if !staging.join("scrcpy.exe").is_file() {
            return Err(ToolError::MissingInArchive("scrcpy.exe"));
        }
        if !staging.join("adb.exe").is_file() {
            return Err(ToolError::MissingInArchive("adb.exe"));
        }
        let target = dir.join("scrcpy");
        let _ = std::fs::remove_dir_all(&target);
        std::fs::rename(&staging, &target).map_err(io)?;
        Ok(())
    })();

    if unpack.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    unpack
}

#[cfg(not(windows))]
fn extract_scrcpy(tar_path: &Path, dir: &Path) -> Result<(), ToolError> {
    let staging = dir.join(format!(".scrcpy-staging-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(io)?;

    let unpack = (|| -> Result<(), ToolError> {
        let status = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(tar_path)
            .arg("--strip-components=1")
            .arg("-C")
            .arg(&staging)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| ToolError::Io(format!("couldn't run tar to unpack scrcpy: {e}")))?;
        if !status.success() {
            return Err(ToolError::Io(
                "tar couldn't unpack the scrcpy archive (or install scrcpy and android-tools with your package manager)."
                    .into(),
            ));
        }
        let scrcpy = staging.join("scrcpy");
        let adb = staging.join("adb");
        if !scrcpy.is_file() {
            return Err(ToolError::MissingInArchive("scrcpy"));
        }
        mark_executable(&scrcpy)?;
        if adb.is_file() {
            mark_executable(&adb)?;
        }
        let target = dir.join("scrcpy");
        let _ = std::fs::remove_dir_all(&target);
        std::fs::rename(&staging, &target).map_err(io)?;
        Ok(())
    })();

    if unpack.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    unpack
}

/// Pulls only `bin/ffmpeg.exe` and `bin/ffprobe.exe` out of the Windows zip archive.
#[cfg(windows)]
fn extract_binaries(zip_path: &Path, dir: &Path) -> Result<(), ToolError> {
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(zip_path).map_err(io)?).map_err(io)?;
    for (suffix, name) in [
        ("/bin/ffmpeg.exe", "ffmpeg.exe"),
        ("/bin/ffprobe.exe", "ffprobe.exe"),
    ] {
        let index = (0..archive.len())
            .find(|&i| {
                archive
                    .by_index(i)
                    .is_ok_and(|f| f.name().ends_with(suffix))
            })
            .ok_or(ToolError::MissingInArchive(name))?;
        let mut entry = archive.by_index(index).map_err(io)?;
        let tmp = dir.join(format!("{name}.part"));
        let mut out = std::fs::File::create(&tmp).map_err(io)?;
        std::io::copy(&mut entry, &mut out).map_err(io)?;
        drop(out);
        std::fs::rename(&tmp, dir.join(name)).map_err(io)?;
    }
    Ok(())
}

/// Extracts `bin/ffmpeg` and `bin/ffprobe` from the Linux `.tar.xz` static archive using `tar`.
#[cfg(not(windows))]
fn extract_binaries(tar_path: &Path, dir: &Path) -> Result<(), ToolError> {
    let scratch = dir.join(format!(".ffmpeg-extract-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).map_err(io)?;

    let unpack = (|| -> Result<(), ToolError> {
        let status = std::process::Command::new("tar")
            .arg("-xJf")
            .arg(tar_path)
            .arg("-C")
            .arg(&scratch)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| {
                ToolError::Io(format!(
                    "couldn't run tar to unpack FFmpeg (install tar and xz, or install ffmpeg with your package manager): {e}"
                ))
            })?;
        if !status.success() {
            return Err(ToolError::Io(
                "tar couldn't unpack the FFmpeg archive (make sure xz is installed, or install ffmpeg with pacman/apt)."
                    .into(),
            ));
        }

        for name in ["ffmpeg", "ffprobe"] {
            let found =
                find_extracted_bin(&scratch, name).ok_or(ToolError::MissingInArchive(name))?;
            let tmp = dir.join(format!("{name}.part"));
            std::fs::copy(&found, &tmp).map_err(io)?;
            mark_executable(&tmp)?;
            std::fs::rename(&tmp, dir.join(name)).map_err(io)?;
        }
        Ok(())
    })();

    let _ = std::fs::remove_dir_all(&scratch);
    unpack
}

#[cfg(not(windows))]
fn find_extracted_bin(root: &Path, wanted: &str) -> Option<std::path::PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&cur) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            let Ok(md) = std::fs::symlink_metadata(&p) else {
                continue;
            };
            if md.file_type().is_symlink() {
                continue;
            }
            if md.is_dir() {
                stack.push(p);
            } else if md.is_file()
                && p.file_name().and_then(|n| n.to_str()) == Some(wanted)
                && p.parent()
                    .and_then(|par| par.file_name())
                    .and_then(|n| n.to_str())
                    == Some("bin")
            {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_listing_styles() {
        let h = "a".repeat(64);
        let g = "B".repeat(64);
        let text = format!("{h}  yt-dlp_linux\n{g} *other.zip\nnot a hash line\n");
        assert_eq!(parse_checksum(&text, "yt-dlp_linux"), Some(h));
        assert_eq!(parse_checksum(&text, "other.zip"), Some("b".repeat(64)));
        assert_eq!(parse_checksum(&text, "missing.zip"), None);
    }

    #[test]
    fn rejects_short_or_non_hex_hashes() {
        assert_eq!(parse_checksum("abc  yt-dlp.exe", "yt-dlp.exe"), None);
        assert_eq!(
            parse_checksum(&format!("{}  yt-dlp.exe", "z".repeat(64)), "yt-dlp.exe"),
            None
        );
    }

    #[test]
    fn verifies_known_hash_and_detects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x");
        std::fs::write(&f, b"abc").unwrap();
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_sha256(&f, abc).unwrap());
        std::fs::write(&f, b"abd").unwrap();
        assert!(!verify_sha256(&f, abc).unwrap());
    }
}
