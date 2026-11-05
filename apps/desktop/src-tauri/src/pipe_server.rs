//! The named pipe the native host talks to. Only the current Windows user can connect, remote
//! clients are rejected by the OS, and every message is parsed strictly before anything happens.

use crate::browser;
use nomnom_ipc::framing::MAX_MESSAGE;
use nomnom_ipc::{parse_request, ErrorCode, Response};
use std::ffi::c_void;
use std::time::Duration;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows::core::HSTRING;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};

const SDDL_REVISION_1: u32 = 1;

/// Creates one pipe instance that only `sid` (the current user) may open.
fn create_instance(name: &str, sid: &str, first: bool) -> std::io::Result<NamedPipeServer> {
    let sddl = HSTRING::from(format!("D:P(A;;GA;;;{sid})"));
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: the SDDL string is valid; the descriptor it allocates is freed after the pipe has
    // been created (the pipe keeps its own copy of the security information).
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

pub fn spawn(app: AppHandle) {
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
                    // Another process already owns the pipe name. Don't talk to an impostor.
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
                if let Err(e) = serve(server, app).await {
                    tracing::debug!("browser connection ended: {e}");
                }
            });
        }
    });
}

async fn serve(server: NamedPipeServer, app: AppHandle) -> std::io::Result<()> {
    let (reader, mut writer) = tokio::io::split(server);
    let mut reader = BufReader::new(reader.take(MAX_MESSAGE as u64 + 1));
    let mut line = Vec::new();
    // A client that connects and says nothing must not hold a pipe instance forever.
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
                    // Handling touches the queue and may open a window, so keep it off the reader.
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
