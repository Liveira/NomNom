//! The named pipe between the native host and the app. The name contains the user's SID so each
//! Windows user has their own, and the server only accepts that user (see the app's pipe server).

use crate::framing::{read_line_limited, write_line};
use std::fs::OpenOptions;
use std::io::{self, BufReader};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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

#[cfg(not(windows))]
pub fn current_user_sid() -> io::Result<String> {
    Err(io::Error::other("Windows only"))
}

/// `\\.\pipe\nomnom-<user SID>`.
pub fn pipe_path() -> String {
    let id = current_user_sid().unwrap_or_else(|_| "unknown".to_string());
    format!(r"\\.\pipe\nomnom-{id}")
}

/// Sends one request line and returns the response line.
///
/// `NotFound` means nothing is listening (the app isn't running). A busy pipe is retried until
/// `timeout`. The whole exchange is bounded by `timeout`.
pub fn exchange(path: &str, request: &[u8], timeout: Duration) -> io::Result<Vec<u8>> {
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn sid_looks_like_a_sid_and_the_path_contains_it() {
        let sid = current_user_sid().unwrap();
        assert!(sid.starts_with("S-1-5-"), "{sid}");
        assert!(pipe_path().ends_with(&sid));
        assert!(pipe_path().starts_with(r"\\.\pipe\nomnom-"));
    }

    #[test]
    fn no_listener_means_not_found_quickly() {
        let started = Instant::now();
        let err = exchange(
            r"\\.\pipe\nomnom-test-nobody-home",
            b"{}",
            Duration::from_secs(5),
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
