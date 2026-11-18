//! The local IPC server the Firefox native host talks to.
//!
//! - On Windows: a Named Pipe restricted by SDDL to the current user's SID, rejecting remote clients.
//! - On Linux: a `0600` Unix domain socket in `$XDG_RUNTIME_DIR` (or `0700` `/tmp/nomnom-<uid>`),
//!   checking `peer_cred().uid()` against `geteuid()`.

use crate::browser;
use nomnom_ipc::framing::MAX_MESSAGE;
use nomnom_ipc::{parse_request, ErrorCode, Response};
use std::time::Duration;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

async fn serve_io<R, W>(reader: R, mut writer: W, app: AppHandle) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(reader.take(MAX_MESSAGE as u64 + 1));
    let mut line = Vec::new();
    let read =
        tokio::time::timeout(Duration::from_secs(5), reader.read_until(b'\n', &mut line)).await;
    let answer = match read {
        Ok(Ok(n)) if n > 0 && n <= MAX_MESSAGE => {
            while matches!(line.last(), Some(b'\n' | b'\r')) {
                line.pop();
            }
            match parse_request(&line) {
                Ok(request) => {
                    let app = app.clone();
                    tokio::task::spawn_blocking(move || browser::handle(&app, request))
                        .await
                        .unwrap_or_else(|_| {
                            Response::err("", ErrorCode::Internal, "Something went wrong.")
                        })
                }
                Err(e) => Response::err("", ErrorCode::Invalid, e.to_string()),
            }
        }
        _ => Response::err("", ErrorCode::Invalid, "NomNom couldn't read that request."),
    };
    let mut bytes = serde_json::to_vec(&answer).unwrap_or_default();
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}

#[cfg(windows)]
pub fn spawn(app: AppHandle) {
    use std::ffi::c_void;
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};

    const SDDL_REVISION_1: u32 = 1;

    fn create_instance(name: &str, sid: &str, first: bool) -> std::io::Result<NamedPipeServer> {
        let sddl = HSTRING::from(format!("D:P(A;;GA;;;{sid})"));
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                &sddl,
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
            .map_err(|e| std::io::Error::other(e.to_string()))?;
            let mut attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor.0,
                bInheritHandle: false.into(),
            };
            let result = ServerOptions::new()
                .first_pipe_instance(first)
                .reject_remote_clients(true)
                .max_instances(8)
                .create_with_security_attributes_raw(
                    name,
                    (&mut attributes as *mut SECURITY_ATTRIBUTES).cast::<c_void>(),
                );
            let _ = LocalFree(HLOCAL(descriptor.0));
            result
        }
    }

    tauri::async_runtime::spawn(async move {
        let name = nomnom_ipc::pipe::pipe_path();
        let Ok(sid) = nomnom_ipc::pipe::current_user_sid() else {
            tracing::error!("couldn't find the current user, so the browser connection is off");
            return;
        };
        let mut first = true;
        loop {
            let server = match create_instance(&name, &sid, first) {
                Ok(s) => s,
                Err(e) if first => {
                    tracing::error!("browser pipe unavailable: {e}");
                    return;
                }
                Err(e) => {
                    tracing::warn!("couldn't open another pipe instance: {e}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };
            first = false;
            if server.connect().await.is_err() {
                continue;
            }
            let app = app.clone();
            tokio::spawn(async move {
                let (r, w) = tokio::io::split(server);
                if let Err(e) = serve_io(r, w, app).await {
                    tracing::debug!("browser connection ended: {e}");
                }
            });
        }
    });
}

#[cfg(unix)]
pub fn spawn(app: AppHandle) {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::path::Path;

    tauri::async_runtime::spawn(async move {
        let sock_path = nomnom_ipc::pipe::pipe_path();
        let path = Path::new(&sock_path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(parent)
                {
                    tracing::error!("couldn't create socket directory {}: {e}", parent.display());
                    return;
                }
            }
        }
        if path.exists() {
            // Only remove a stale socket file; never steal one from a live instance.
            if std::os::unix::net::UnixStream::connect(path).is_ok() {
                tracing::error!(
                    "browser socket {} is already owned by another instance",
                    sock_path
                );
                return;
            }
            let _ = std::fs::remove_file(path);
        }
        let listener = match tokio::net::UnixListener::bind(path) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("couldn't bind browser socket {}: {e}", sock_path);
                return;
            }
        };
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        let expected_uid = unsafe { libc::geteuid() };

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!("browser socket accept error: {e}");
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    continue;
                }
            };
            match stream.peer_cred() {
                Ok(cred) if cred.uid() == expected_uid => {}
                _ => {
                    tracing::warn!("rejected Unix socket connection from foreign uid");
                    continue;
                }
            }
            let app = app.clone();
            tokio::spawn(async move {
                let (r, w) = stream.into_split();
                if let Err(e) = serve_io(r, w, app).await {
                    tracing::debug!("browser connection ended: {e}");
                }
            });
        }
    });
}
