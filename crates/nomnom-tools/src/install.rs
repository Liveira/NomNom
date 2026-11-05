use crate::ToolError;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

/// Receives (bytes downloaded, total bytes when the server says).
pub type InstallProgress = dyn Fn(u64, Option<u64>) + Send + Sync;

const YTDLP_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const FFMPEG_BASE: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download";
const FFMPEG_ARCHIVE: &str = "ffmpeg-master-latest-win64-gpl.zip";

fn io(e: impl std::fmt::Display) -> ToolError {
    ToolError::Io(e.to_string())
}

fn net(e: impl std::fmt::Display) -> ToolError {
    ToolError::Network(e.to_string())
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

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, ToolError> {
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

async fn download_to(
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
    let sums = fetch_text(&client, &format!("{YTDLP_BASE}/SHA2-256SUMS")).await?;
    let expected = parse_checksum(&sums, "yt-dlp.exe")
        .ok_or_else(|| ToolError::Network("the release has no checksum for yt-dlp.exe".into()))?;

    let tmp = dir.join("yt-dlp.exe.part");
    download_to(&client, &format!("{YTDLP_BASE}/yt-dlp.exe"), &tmp, progress).await?;
    if !verify_sha256(&tmp, &expected)? {
        let _ = std::fs::remove_file(&tmp);
        return Err(ToolError::ChecksumMismatch);
    }
    std::fs::rename(&tmp, dir.join("yt-dlp.exe")).map_err(io)
}

pub(crate) async fn install_ffmpeg(
    dir: &Path,
    progress: &InstallProgress,
) -> Result<(), ToolError> {
    std::fs::create_dir_all(dir).map_err(io)?;
    let client = client()?;
    let sums = fetch_text(&client, &format!("{FFMPEG_BASE}/checksums.sha256")).await?;
    let expected = parse_checksum(&sums, FFMPEG_ARCHIVE).ok_or_else(|| {
        ToolError::Network("the release has no checksum for the ffmpeg archive".into())
    })?;

    let zip_path = dir.join("ffmpeg-download.zip.part");
    download_to(
        &client,
        &format!("{FFMPEG_BASE}/{FFMPEG_ARCHIVE}"),
        &zip_path,
        progress,
    )
    .await?;
    let result = (|| {
        if !verify_sha256(&zip_path, &expected)? {
            return Err(ToolError::ChecksumMismatch);
        }
        extract_binaries(&zip_path, dir)
    })();
    let _ = std::fs::remove_file(&zip_path);
    result
}

/// Pulls only `bin/ffmpeg.exe` and `bin/ffprobe.exe` out of the archive.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_listing_styles() {
        let h = "a".repeat(64);
        let g = "B".repeat(64);
        let text = format!("{h}  yt-dlp.exe\n{g} *other.zip\nnot a hash line\n");
        assert_eq!(parse_checksum(&text, "yt-dlp.exe"), Some(h));
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
