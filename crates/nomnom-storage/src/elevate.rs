//! Elevated scan helper (`--elevated-scan`) and full-scan file save/load.
//!
//! Keeping the main window un-elevated preserves Explorer drag-and-drop, per-user settings and
//! single-instance behaviour. When the person asks to scan as Administrator, Windows shows its
//! standard UAC prompt for a short-lived helper copy of `nomnom.exe`, which enables
//! `SeBackupPrivilege`, scans the requested drive or folder, and streams progress and the finished
//! index back over a private Named Pipe.

use crate::index::Index;
use crate::scan::{SavedScan, ScanOptions, Session, State};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::FromRawHandle;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_NONE,
    OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

#[derive(Serialize, Deserialize)]
enum WireMsg {
    Progress {
        files: u64,
        dirs: u64,
        bytes: u64,
        errors: u64,
        mft_bytes: u64,
    },
    Finished(SavedScan),
    Failed(String),
}

fn wide_nul(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Starts an elevated helper process via Windows UAC (`runas`) and returns a live `Session` whose
/// counters and final index are populated from the helper.
pub fn spawn_elevated_scan(
    exe: &Path,
    id: u64,
    root: PathBuf,
    opts: ScanOptions,
) -> Result<Arc<Session>, String> {
    let nonce = format!(
        "{}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let pipe_name = format!(r"\\.\pipe\nomnom-elev-scan-{nonce}");
    let wpipe = wide_nul(&pipe_name);
    // SAFETY: `wpipe` is NUL-terminated; 1 instance only so no second client can attach.
    let pipe_handle = unsafe {
        CreateNamedPipeW(
            PCWSTR(wpipe.as_ptr()),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            1.min(PIPE_UNLIMITED_INSTANCES),
            1 << 20,
            1 << 20,
            15_000,
            None,
        )
    };
    if pipe_handle.is_invalid() || pipe_handle == INVALID_HANDLE_VALUE {
        return Err("Couldn't prepare the communication pipe for the elevated scan.".into());
    }

    let opts_json = serde_json::to_string(&opts).unwrap_or_else(|_| "{}".into());
    let params = format!(
        "--elevated-scan {} {} {}",
        quote(&pipe_name),
        quote(&root.to_string_lossy()),
        quote(&opts_json),
    );
    let wverb = wide_nul("runas");
    let wfile = wide_nul(&exe.to_string_lossy());
    let wparams = wide_nul(&params);
    // SAFETY: `wverb`, `wfile` and `wparams` are valid NUL-terminated wide buffers.
    let hinst = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(wverb.as_ptr()),
            PCWSTR(wfile.as_ptr()),
            PCWSTR(wparams.as_ptr()),
            PCWSTR::null(),
            SW_HIDE,
        )
    };
    let status = hinst.0 as isize;
    if status <= 32 {
        unsafe {
            let _ = CloseHandle(pipe_handle);
        }
        if status == 5 {
            return Err("Administrator permission was declined.".into());
        }
        return Err(format!("Couldn't start the elevated scan (code {status})."));
    }

    let session = Session::new_shell(id, root, opts, true);
    let session_bg = session.clone();
    // Raw HANDLE is Send-safe inside the dedicated pipe reader thread.
    let raw_pipe = pipe_handle.0 as usize;
    std::thread::Builder::new()
        .name("storage-elev-recv".into())
        .spawn(move || {
            let handle = windows::Win32::Foundation::HANDLE(raw_pipe as *mut _);
            // SAFETY: `handle` is the server end created by CreateNamedPipeW above.
            let connected = unsafe { ConnectNamedPipe(handle, None) };
            let ok = connected.is_ok() || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
            if !ok {
                unsafe {
                    let _ = CloseHandle(handle);
                }
                session_bg.finish(true);
                return;
            }
            // SAFETY: exclusive ownership of `handle` transferred to `File`.
            let file = unsafe { File::from_raw_handle(handle.0) };
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if session_bg.is_cancelled() {
                    // Dropping `reader` closes the pipe, signalling the helper to abort.
                    session_bg.finish(true);
                    return;
                }
                let Ok(line) = line else {
                    break;
                };
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<WireMsg>(&line) {
                    Ok(WireMsg::Progress {
                        files,
                        dirs,
                        bytes,
                        errors,
                        mft_bytes,
                    }) => {
                        session_bg.stats.files.store(files, Ordering::Relaxed);
                        session_bg.stats.dirs.store(dirs, Ordering::Relaxed);
                        session_bg.stats.bytes.store(bytes, Ordering::Relaxed);
                        session_bg.stats.errors.store(errors, Ordering::Relaxed);
                        session_bg.mft_bytes.store(mft_bytes, Ordering::Relaxed);
                    }
                    Ok(WireMsg::Finished(saved)) => {
                        session_bg
                            .mft_bytes
                            .store(saved.mft_bytes, Ordering::Relaxed);
                        session_bg
                            .elapsed_ms
                            .store(saved.elapsed_ms, Ordering::SeqCst);
                        session_bg.set_errors(saved.errors);
                        if let Ok(idx) = Index::from_archive(saved.index) {
                            if let Some(r) = idx.node(0) {
                                session_bg
                                    .stats
                                    .files
                                    .store(u64::from(r.files), Ordering::Relaxed);
                                session_bg
                                    .stats
                                    .dirs
                                    .store(u64::from(r.dirs), Ordering::Relaxed);
                                session_bg.stats.bytes.store(r.size, Ordering::Relaxed);
                            }
                            *session_bg.index.write().unwrap() = idx;
                            session_bg.finish(false);
                            return;
                        } else {
                            session_bg.finish(true);
                            return;
                        }
                    }
                    Ok(WireMsg::Failed(_)) | Err(_) => {
                        session_bg.finish(true);
                        return;
                    }
                }
            }
            if !session_bg.is_finished() {
                session_bg.finish(true);
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(session)
}

/// Entrypoint run inside the elevated helper (`nomnom.exe --elevated-scan <pipe> <root> <opts>`).
pub fn run_elevated_helper(pipe_name: &str, root: &str, opts_json: &str) -> i32 {
    if !pipe_name.starts_with(r"\\.\pipe\nomnom-elev-scan-") {
        return 2;
    }
    let wpipe = wide_nul(pipe_name);
    // SAFETY: `wpipe` is NUL-terminated.
    let Ok(handle) = (unsafe {
        CreateFileW(
            PCWSTR(wpipe.as_ptr()),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }) else {
        return 3;
    };
    // SAFETY: exclusive ownership of `handle` transferred to `File`.
    let file = unsafe { File::from_raw_handle(handle.0) };
    let mut writer = BufWriter::new(file);

    let mut opts: ScanOptions = serde_json::from_str(opts_json).unwrap_or_default();
    if opts.workers < 8 {
        opts.workers = 8;
    }
    let path = PathBuf::from(root);
    let session = Session::start(1, path, opts);
    loop {
        let msg = WireMsg::Progress {
            files: session.stats.files.load(Ordering::Relaxed),
            dirs: session.stats.dirs.load(Ordering::Relaxed),
            bytes: session.stats.bytes.load(Ordering::Relaxed),
            errors: session.stats.errors.load(Ordering::Relaxed),
            mft_bytes: session.mft_bytes.load(Ordering::Relaxed),
        };
        if send_line(&mut writer, &msg).is_err() {
            session.cancel();
            return 1;
        }
        if session.is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    if session.state() == State::Done {
        let saved = session.to_saved();
        let _ = send_line(&mut writer, &WireMsg::Finished(saved));
        0
    } else {
        let _ = send_line(&mut writer, &WireMsg::Failed("Scan stopped.".into()));
        1
    }
}

fn send_line(w: &mut BufWriter<File>, msg: &WireMsg) -> std::io::Result<()> {
    serde_json::to_writer(&mut *w, msg).map_err(std::io::Error::other)?;
    w.write_all(b"\n")?;
    w.flush()
}

fn quote(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"', '\n']) {
        return arg.to_string();
    }
    let mut out = String::from("\"");
    let mut bs = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => bs += 1,
            '"' => {
                out.push_str(&"\\".repeat(bs * 2 + 1));
                out.push('"');
                bs = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(bs));
                out.push(c);
                bs = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(bs * 2));
    out.push('"');
    out
}

/// Saves a finished scan to a `.json` file on disk so it can be opened and browsed later.
pub fn save_scan_file(session: &Session, path: &Path) -> Result<u64, String> {
    if !session.is_finished() {
        return Err("Wait for the scan to finish before saving it.".into());
    }
    let saved = session.to_saved();
    let bytes = serde_json::to_vec(&saved).map_err(|e| e.to_string())?;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                "A file with that name already exists. Choose another name.".to_string()
            } else {
                e.to_string()
            }
        })?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(bytes.len() as u64)
}

/// Loads a saved `.json` scan file into a finished `Session`.
pub fn load_scan_file(id: u64, path: &Path) -> Result<Arc<Session>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read that scan file: {e}"))?;
    let saved: SavedScan = serde_json::from_slice(&bytes)
        .map_err(|e| format!("That isn't a valid NomNom scan file: {e}"))?;
    Session::from_saved(id, saved)
}
