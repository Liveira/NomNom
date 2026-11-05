//! The small program Firefox starts for each message from the NOMNOM extension. It passes the
//! message to the running app over the pipe and returns the answer. If the app is not running it
//! can start it in the background (never with a window), when the user allowed that.

use nomnom_ipc::{parse_request, ErrorCode, Response};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct Options {
    /// How long to wait for the app to answer once connected.
    pub exchange_timeout: Duration,
    /// How long to keep trying to connect after starting the app.
    pub launch_wait: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            exchange_timeout: Duration::from_secs(10),
            launch_wait: Duration::from_secs(8),
        }
    }
}

fn error_bytes(id: &str, code: ErrorCode, message: &str) -> Vec<u8> {
    serde_json::to_vec(&Response::err(id, code, message)).unwrap_or_else(|_| b"{}".to_vec())
}

/// Pulls the request id out of a message even if the rest is invalid, so errors can echo it.
fn best_effort_id(bytes: &[u8]) -> String {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|v| {
            v.get("id")
                .and_then(|i| i.as_str())
                .map(|s| s.chars().take(64).collect())
        })
        .unwrap_or_default()
}

/// Validates `request`, delivers it to the app at `pipe`, and returns the bytes to send back to
/// Firefox. Never panics and always returns a well-formed response.
pub fn relay(
    request: &[u8],
    pipe: &str,
    launch: &mut dyn FnMut() -> bool,
    opts: &Options,
) -> Vec<u8> {
    let id = best_effort_id(request);
    if let Err(e) = parse_request(request) {
        return error_bytes(
            &id,
            ErrorCode::Invalid,
            &format!("NOMNOM could not read this request: {e}."),
        );
    }
    // The app gets the validated request re-serialised without newlines, never raw bytes.
    let compact = match serde_json::from_slice::<serde_json::Value>(request)
        .and_then(|v| serde_json::to_vec(&v))
    {
        Ok(c) => c,
        Err(_) => {
            return error_bytes(
                &id,
                ErrorCode::Invalid,
                "NOMNOM could not read this request.",
            )
        }
    };

    match nomnom_ipc::pipe::exchange(pipe, &compact, opts.exchange_timeout) {
        Ok(answer) => answer,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if !launch() {
                return error_bytes(&id, ErrorCode::NotRunning, "NOMNOM isn't running.");
            }
            let started = Instant::now();
            loop {
                std::thread::sleep(Duration::from_millis(150));
                match nomnom_ipc::pipe::exchange(pipe, &compact, opts.exchange_timeout) {
                    Ok(answer) => return answer,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::NotFound
                            && started.elapsed() < opts.launch_wait =>
                    {
                        continue
                    }
                    Err(_) => {
                        return error_bytes(
                            &id,
                            ErrorCode::NotRunning,
                            "NOMNOM started but didn't answer.",
                        );
                    }
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
            error_bytes(&id, ErrorCode::Internal, "NOMNOM didn't answer in time.")
        }
        Err(_) => error_bytes(&id, ErrorCode::Internal, "Couldn't talk to NOMNOM."),
    }
}

/// What the app tells the host through `%LOCALAPPDATA%\NOMNOM\integration.json`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Integration {
    pub exe: Option<PathBuf>,
    pub launch_from_browser: bool,
}

pub fn read_integration(path: &Path) -> Integration {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Integration::default();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text.trim_start_matches('\u{feff}'))
    else {
        return Integration::default();
    };
    Integration {
        exe: v.get("exe").and_then(|e| e.as_str()).map(PathBuf::from),
        launch_from_browser: v
            .get("launchFromBrowser")
            .and_then(|b| b.as_bool())
            .unwrap_or(false),
    }
}

/// Only an existing file literally named nomnom.exe is ever started.
pub fn launchable(integration: &Integration) -> Option<&Path> {
    let exe = integration.exe.as_deref()?;
    let name_ok = exe
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("nomnom.exe"));
    (integration.launch_from_browser && name_ok && exe.is_file()).then_some(exe)
}

/// Stops this process's standard handles (Firefox's pipes) from being inherited by anything we start.
/// Without this, a long-lived child keeps Firefox's pipe open and whoever waits for the host to
/// finish never sees the end of its output.
#[cfg(windows)]
fn make_std_handles_private() {
    use windows::Win32::Foundation::{SetHandleInformation, HANDLE_FLAGS, HANDLE_FLAG_INHERIT};
    use windows::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };
    for which in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: plain handle queries/flag changes on this process's own standard handles.
        unsafe {
            if let Ok(h) = GetStdHandle(which) {
                if !h.is_invalid() {
                    let _ = SetHandleInformation(h, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0));
                }
            }
        }
    }
}

/// Starts NOMNOM without a window and without touching this process's stdin/stdout, which belong
/// to Firefox.
#[cfg(windows)]
pub fn launch_in_background(exe: &Path) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    make_std_handles_private();
    Command::new(exe)
        .arg("--background")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .is_ok()
}

#[cfg(not(windows))]
pub fn launch_in_background(_exe: &Path) -> bool {
    false
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ServerOptions;

    const PING: &[u8] = br#"{"v":1,"id":"p1","type":"ping"}"#;

    fn name(tag: &str) -> String {
        format!(r"\\.\pipe\nomnom-hosttest-{tag}-{}", std::process::id())
    }

    /// A stand-in app: answers one request with `reply` (or never answers if `None`).
    async fn serve_once(pipe: String, reply: Option<Vec<u8>>) -> Vec<u8> {
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe)
            .unwrap();
        server.connect().await.unwrap();
        let mut reader = BufReader::new(server);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        if let Some(mut r) = reply {
            r.push(b'\n');
            reader.get_mut().write_all(&r).await.unwrap();
            reader.get_mut().flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        } else {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        line.trim().as_bytes().to_vec()
    }

    fn fast() -> Options {
        Options {
            exchange_timeout: Duration::from_millis(800),
            launch_wait: Duration::from_secs(3),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn passes_the_request_through_and_returns_the_apps_answer() {
        let pipe = name("ok");
        let server = tokio::spawn(serve_once(
            pipe.clone(),
            Some(br#"{"v":1,"id":"p1","ok":true,"app":"0.1.0"}"#.to_vec()),
        ));
        tokio::time::sleep(Duration::from_millis(100)).await;
        let p = pipe.clone();
        let answer = tokio::task::spawn_blocking(move || relay(PING, &p, &mut || false, &fast()))
            .await
            .unwrap();
        assert_eq!(answer, br#"{"v":1,"id":"p1","ok":true,"app":"0.1.0"}"#);
        let received: serde_json::Value = serde_json::from_slice(&server.await.unwrap()).unwrap();
        let sent: serde_json::Value = serde_json::from_slice(PING).unwrap();
        assert_eq!(
            received, sent,
            "the app received the same request, on one line"
        );
    }

    #[test]
    fn app_not_running_and_launching_not_allowed_says_so_quickly() {
        let started = Instant::now();
        let answer = relay(PING, &name("none"), &mut || false, &fast());
        let r: Response = serde_json::from_slice(&answer).unwrap();
        assert!(!r.ok && r.id == "p1");
        assert_eq!(r.error.unwrap().code, ErrorCode::NotRunning);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn starts_the_app_when_allowed_and_then_delivers_the_request() {
        let pipe = name("launch");
        let p = pipe.clone();
        let launched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = launched.clone();
        let pipe_for_launch = pipe.clone();
        let answer = tokio::task::spawn_blocking(move || {
            relay(
                PING,
                &p,
                &mut || {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    // The "app" takes a moment to open its pipe after being started.
                    let pipe = pipe_for_launch.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(500));
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();
                        rt.block_on(serve_once(
                            pipe,
                            Some(br#"{"v":1,"id":"p1","ok":true}"#.to_vec()),
                        ));
                    });
                    true
                },
                &fast(),
            )
        })
        .await
        .unwrap();
        assert!(launched.load(std::sync::atomic::Ordering::SeqCst));
        let r: Response = serde_json::from_slice(&answer).unwrap();
        assert!(r.ok, "{r:?}");
    }

    #[test]
    fn app_that_never_starts_gives_up_with_not_running() {
        let opts = Options {
            exchange_timeout: Duration::from_millis(300),
            launch_wait: Duration::from_millis(600),
        };
        let answer = relay(PING, &name("never"), &mut || true, &opts);
        let r: Response = serde_json::from_slice(&answer).unwrap();
        assert_eq!(r.error.unwrap().code, ErrorCode::NotRunning);
    }

    #[test]
    fn bad_requests_are_refused_without_touching_the_pipe_or_launching() {
        let mut launched = false;
        for bad in [
            &b"nope"[..],
            br#"{"v":1,"id":"z","type":"format_disk"}"#,
            br#"{"v":1,"id":"z","type":"ping","x":1}"#,
        ] {
            let answer = relay(
                bad,
                &name("bad"),
                &mut || {
                    launched = true;
                    true
                },
                &fast(),
            );
            let r: Response = serde_json::from_slice(&answer).unwrap();
            assert_eq!(r.error.unwrap().code, ErrorCode::Invalid);
        }
        assert!(!launched);
    }

    #[test]
    fn invalid_request_keeps_the_callers_id_when_it_has_one() {
        let answer = relay(
            br#"{"v":1,"id":"keepme","type":"bogus"}"#,
            &name("id"),
            &mut || false,
            &fast(),
        );
        assert_eq!(
            serde_json::from_slice::<Response>(&answer).unwrap().id,
            "keepme"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_app_that_never_answers_times_out_instead_of_hanging() {
        let pipe = name("silent");
        tokio::spawn(serve_once(pipe.clone(), None));
        tokio::time::sleep(Duration::from_millis(100)).await;
        let started = Instant::now();
        let p = pipe.clone();
        let answer = tokio::task::spawn_blocking(move || relay(PING, &p, &mut || false, &fast()))
            .await
            .unwrap();
        let r: Response = serde_json::from_slice(&answer).unwrap();
        assert_eq!(r.error.unwrap().code, ErrorCode::Internal);
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn only_an_existing_nomnom_exe_with_permission_is_ever_launched() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("nomnom.exe");
        let other = dir.path().join("calc.exe");
        std::fs::write(&good, b"x").unwrap();
        std::fs::write(&other, b"x").unwrap();
        let mk = |exe: Option<&Path>, allow| Integration {
            exe: exe.map(Path::to_path_buf),
            launch_from_browser: allow,
        };
        assert_eq!(launchable(&mk(Some(&good), true)), Some(good.as_path()));
        assert_eq!(
            launchable(&mk(Some(&good), false)),
            None,
            "needs the user's permission"
        );
        assert_eq!(launchable(&mk(Some(&other), true)), None, "wrong file name");
        assert_eq!(
            launchable(&mk(Some(&dir.path().join("nomnom.exe.missing")), true)),
            None
        );
        assert_eq!(launchable(&mk(None, true)), None);
    }

    #[test]
    fn integration_file_is_read_defensively() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("integration.json");
        assert_eq!(read_integration(&f), Integration::default());
        std::fs::write(
            &f,
            "\u{feff}{\"exe\":\"C:\\\\x\\\\nomnom.exe\",\"launchFromBrowser\":true}",
        )
        .unwrap();
        let i = read_integration(&f);
        assert!(i.launch_from_browser && i.exe.is_some());
        std::fs::write(&f, "garbage").unwrap();
        assert_eq!(read_integration(&f), Integration::default());
    }
}
