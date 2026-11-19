//! Sending files between this computer and a phone over the home network: the rules that keep it
//! safe (who may connect, what a received file may be called, where it may go) and the small
//! helpers the page needs. Nothing here opens a network connection.

use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

/// The biggest single file accepted from the phone, in megabytes.
pub const MAX_MB: u64 = 8 * 1024;

/// A home or office network address (192.168.x.x, 10.x.x.x, 172.16-31.x.x) or this computer itself.
/// Anything else, such as an address on the internet, is never let in.
pub fn is_local_client(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique local
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link local
        }
    }
}

/// Is this an address worth showing on a QR code: a private IPv4 of this computer?
pub fn is_shareable_address(ip: Ipv4Addr) -> bool {
    ip.is_private() && !ip.is_loopback()
}

/// The two strings are equal, compared in a way that doesn't reveal how many leading characters
/// matched (the secret in the link is checked with this).
pub fn same_secret(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// A file name that is safe to create on this computer: no folders, no odd characters, never empty.
pub fn safe_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("").trim();
    let mut name: String = base
        .chars()
        .map(|c| match c {
            ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    name = name.trim().trim_end_matches('.').to_string();
    if name.is_empty() || name == ".." {
        name = "file".into();
    }
    if name.chars().count() > 150 {
        // Keep the extension when shortening.
        let ext: String = Path::new(&name)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .filter(|e| e.chars().count() <= 12)
            .unwrap_or_default();
        let stem: String = name.chars().take(140).collect();
        name = format!("{stem}{ext}");
    }
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "LPT1", "LPT2", "LPT3",
    ];
    let stem_upper = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED.contains(&stem_upper.as_str()) {
        name.insert(0, '_');
    }
    name
}

/// A path in `dir` for `name` that doesn't exist yet: `a.jpg`, then `a (2).jpg`, `a (3).jpg`…
pub fn free_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..10_000)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|c| !c.exists())
        .unwrap_or_else(|| dir.join(format!("{stem} ({}){ext}", std::process::id())))
}

/// The value of `key` in a query string such as `name=a%20b.jpg&x=1`, decoded.
pub fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| percent_decode(v))
    })
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A size such as "3.4 MB", for the page.
pub fn size_text(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

/// `Content-Disposition` for a download, with the name encoded so any name works.
pub fn disposition(name: &str) -> String {
    let ascii: String = name
        .chars()
        .map(|c| {
            if c.is_ascii() && c != '"' && c != '\\' && !c.is_control() {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encoded: String = name
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_devices_on_the_local_network_may_connect() {
        for ok in [
            "192.168.1.20",
            "10.0.0.5",
            "172.20.1.1",
            "127.0.0.1",
            "169.254.3.3",
            "::1",
            "fd12::1",
            "fe80::1",
        ] {
            assert!(is_local_client(ok.parse().unwrap()), "{ok}");
        }
        for bad in [
            "8.8.8.8",
            "93.184.216.34",
            "2001:4860:4860::8888",
            "172.32.0.1",
        ] {
            assert!(!is_local_client(bad.parse().unwrap()), "{bad}");
        }
        assert!(is_shareable_address("192.168.3.49".parse().unwrap()));
        assert!(!is_shareable_address("127.0.0.1".parse().unwrap()));
        assert!(!is_shareable_address("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn the_secret_is_compared_whole() {
        assert!(same_secret("abc123", "abc123"));
        assert!(!same_secret("abc123", "abc124"));
        assert!(!same_secret("abc123", "abc12"));
        assert!(!same_secret("", "x"));
    }

    #[test]
    fn a_received_name_cannot_escape_the_folder_or_break_the_system() {
        assert_eq!(safe_name("../../Windows/evil.exe"), "evil.exe");
        assert_eq!(safe_name("C:\\Users\\me\\photo.jpg"), "photo.jpg");
        assert_eq!(safe_name("a:b*c?.txt"), "a_b_c_.txt");
        assert_eq!(safe_name(""), "file");
        assert_eq!(safe_name(".."), "file");
        assert_eq!(safe_name("  holiday.png.  "), "holiday.png");
        assert_eq!(safe_name("con.txt"), "_con.txt");
        let long = format!("{}.mp4", "x".repeat(400));
        let short = safe_name(&long);
        assert!(short.chars().count() <= 160 && short.ends_with(".mp4"));
    }

    #[test]
    fn a_name_that_is_taken_gets_a_number() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(free_path(dir.path(), "a.jpg"), dir.path().join("a.jpg"));
        std::fs::write(dir.path().join("a.jpg"), "1").unwrap();
        assert_eq!(free_path(dir.path(), "a.jpg"), dir.path().join("a (2).jpg"));
        std::fs::write(dir.path().join("a (2).jpg"), "2").unwrap();
        assert_eq!(free_path(dir.path(), "a.jpg"), dir.path().join("a (3).jpg"));
        std::fs::write(dir.path().join("noext"), "1").unwrap();
        assert_eq!(free_path(dir.path(), "noext"), dir.path().join("noext (2)"));
    }

    #[test]
    fn the_query_is_read_and_decoded() {
        assert_eq!(
            query_value("name=a%20b%C3%A9.jpg&x=1", "name").as_deref(),
            Some("a bé.jpg")
        );
        assert_eq!(query_value("x=1", "name"), None);
        assert_eq!(query_value("name=a+b", "name").as_deref(), Some("a b"));
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn sizes_and_download_names_read_well() {
        assert_eq!(size_text(500), "500 B");
        assert_eq!(size_text(1536), "1.5 KB");
        assert_eq!(size_text(5 * 1024 * 1024), "5.0 MB");
        let d = disposition("fé \"x\".jpg");
        assert!(
            d.starts_with("attachment; filename=\"f_ _x_.jpg\"")
                || d.contains("filename*=UTF-8''f%C3%A9%20%22x%22.jpg")
        );
        assert!(d.contains("UTF-8''"));
    }
}
