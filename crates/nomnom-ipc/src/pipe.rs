//! The local IPC channel between the native host and the app.
//!
//! On Windows this is a per-SID Named Pipe (`\\.\pipe\nomnom-<SID>`) restricted by SDDL to the
//! current user. On Linux it is a `0600` Unix domain socket inside `$XDG_RUNTIME_DIR` (or a `0700`
//! `/tmp/nomnom-<uid>` fallback) verified by `SO_PEERCRED` / peer UID.

use crate::framing::{read_line_limited, write_line};
use std::io::{self, BufReader};
use std::time::Duration;

#[cfg(windows)]
pub fn current_user_sid() -> io::Result<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
    use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let fail = |e: windows::core::Error| io::Error::other(e.to_string());
    // SAFETY: standard token query. The buffer is 8-byte aligned and sized by the first call; the
    // string returned by ConvertSidToStringSidW is copied and then freed exactly once.
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).map_err(fail)?;
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        let mut buf = vec![0u64; (needed as usize).div_ceil(8).max(1)];
        let result = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            needed,
            &mut needed,
        );
        let sid = result.map_err(fail).and_then(|_| {
            let user = &*(buf.as_ptr() as *const TOKEN_USER);
            let mut text = PWSTR::null();
            ConvertSidToStringSidW(user.User.Sid, &mut text).map_err(fail)?;
            let s = text
                .to_string()
                .map_err(|e| io::Error::other(e.to_string()));
            let _ = LocalFree(HLOCAL(text.0.cast()));
            s
        });
        let _ = CloseHandle(token);
        sid
    }
}

#[cfg(unix)]
pub fn current_user_sid() -> io::Result<String> {
    // SAFETY: geteuid is a thread-safe syscall with no arguments and cannot fail.
    Ok(unsafe { libc::geteuid() }.to_string())
}

#[cfg(not(any(windows, unix)))]
pub fn current_user_sid() -> io::Result<String> {
    Err(io::Error::other("unsupported platform"))
}

/// Returns the IPC endpoint path for the current user:
/// - Windows: `\\.\pipe\nomnom-<user SID>`
/// - Linux: `$XDG_RUNTIME_DIR/nomnom.sock` (or `/tmp/nomnom-<uid>/nomnom.sock`)
#[cfg(windows)]
pub fn pipe_path() -> String {
    let id = current_user_sid().unwrap_or_else(|_| "unknown".to_string());
    // A debug test copy with its own data folder gets its own pipe, so it never answers the
    // browser extension of the real app running next to it.
    #[cfg(debug_assertions)]
    if std::env::var_os("NOMNOM_TEST_DATA").is_some() {
        return format!(r"\\.\pipe\nomnom-test-{id}");
    }
    format!(r"\\.\pipe\nomnom-{id}")
}

#[cfg(not(windows))]
pub fn pipe_path() -> String {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        return dir.join("nomnom.sock").to_string_lossy().into_owned();
    }
    let id = current_user_sid().unwrap_or_else(|_| "unknown".to_string());
    format!("/tmp/nomnom-{id}/nomnom.sock")
}

/// Sends one request line and returns the response line.
///
/// `NotFound` means nothing is listening (the app isn't running). A busy pipe is retried until
/// `timeout`. The whole exchange is bounded by `timeout`.
#[cfg(windows)]
pub fn exchange(path: &str, request: &[u8], timeout: Duration) -> io::Result<Vec<u8>> {
    use std::fs::OpenOptions;
    use std::sync::mpsc;
    use std::time::Instant;

    const ERROR_PIPE_BUSY: i32 = 231;
    let started = Instant::now();
    let file = loop {
        match OpenOptions::new().read(true).write(true).open(path) {
            Ok(f) => break f,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(e) => return Err(e),
        }
    };

    let request = request.to_vec();
    let (tx, rx) = mpsc::channel();
    // Pipes have no read timeout in std, so the exchange runs on a thread we can stop waiting for.
    std::thread::spawn(move || {
        let result = (|| {
            let mut writer = &file;
            write_line(&mut writer, &request)?;
            let mut reader = BufReader::new(&file);
            read_line_limited(&mut reader)
        })();
        let _ = tx.send(result);
    });
    let remaining = timeout
        .saturating_sub(started.elapsed())
        .max(Duration::from_millis(200));
    rx.recv_timeout(remaining)
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "NOMNOM did not answer in time"))?
}

#[cfg(unix)]
pub fn exchange(path: &str, request: &[u8], timeout: Duration) -> io::Result<Vec<u8>> {
    use std::os::unix::net::UnixStream;
    use std::time::Instant;

    let started = Instant::now();
    let mut stream = match UnixStream::connect(path) {
        Ok(s) => s,
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "NOMNOM is not running",
            ));
        }
        Err(e) => return Err(e),
    };

    let map_timeout = |e: io::Error| {
        if matches!(
            e.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
        ) {
            io::Error::new(io::ErrorKind::TimedOut, "NOMNOM did not answer in time")
        } else {
            e
        }
    };

    stream.set_write_timeout(Some(timeout))?;
    write_line(&mut stream, request).map_err(map_timeout)?;

    let remaining = timeout
        .saturating_sub(started.elapsed())
        .max(Duration::from_millis(200));
    stream.set_read_timeout(Some(remaining))?;
    let mut reader = BufReader::new(stream);
    read_line_limited(&mut reader).map_err(map_timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    #[cfg(windows)]
    fn sid_looks_like_a_sid_and_the_path_contains_it() {
        let sid = current_user_sid().unwrap();
        assert!(sid.starts_with("S-1-5-"), "{sid}");
        assert!(pipe_path().ends_with(&sid));
        assert!(pipe_path().starts_with(r"\\.\pipe\nomnom-"));
    }

    #[test]
    #[cfg(unix)]
    fn uid_is_numeric_and_socket_path_ends_with_sock() {
        let uid = current_user_sid().unwrap();
        assert!(uid.chars().all(|c| c.is_ascii_digit()), "{uid}");
        assert!(pipe_path().ends_with("nomnom.sock"), "{}", pipe_path());
    }

    #[test]
    fn no_listener_means_not_found_quickly() {
        let started = Instant::now();
        let missing = if cfg!(windows) {
            r"\\.\pipe\nomnom-test-nobody-home".to_string()
        } else {
            format!("/tmp/nomnom-test-nobody-home-{}.sock", std::process::id())
        };
        let err = exchange(&missing, b"{}", Duration::from_secs(5)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
