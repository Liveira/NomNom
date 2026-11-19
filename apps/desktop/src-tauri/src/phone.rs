//! Sending files between this computer and a phone. A small web page is served on the home network
//! for a while: the phone opens it from a QR code, picks files to send here, and downloads the
//! files this computer offers. Nothing goes through the internet.
//!
//! It is only on while the person asks for it, it listens on the computer's home-network address
//! only, a secret in the link is needed for everything, only local addresses may connect, and it
//! turns itself off after a quiet while.

use crate::state::AppState;
use crate::toast::{self, Toast};
use nomnom_core::phone as rules;
use serde::Serialize;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

type CmdResult<T> = Result<T, String>;

const PAGE: &str = include_str!("phone_page.html");

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize)]
pub struct Offered {
    pub id: String,
    pub name: String,
    pub size: u64,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct Received {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub at: i64,
}

#[derive(Default)]
pub struct PhoneState {
    running: Mutex<Option<Running>>,
}

struct Running {
    stop: Arc<AtomicBool>,
    url: String,
    dest: PathBuf,
    offered: Arc<Mutex<Vec<Offered>>>,
    received: Arc<Mutex<Vec<Received>>>,
    last_activity: Arc<AtomicI64>,
}

#[derive(Serialize)]
pub struct Status {
    pub running: bool,
    pub url: Option<String>,
    /// The QR code of the link, as an SVG picture.
    pub qr: Option<String>,
    pub dest: String,
    pub idle_minutes: u32,
    pub offered: Vec<Offered>,
    pub received: Vec<Received>,
}

fn default_dest(state: &AppState) -> PathBuf {
    state
        .settings
        .get()
        .phone
        .save_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| state.paths.downloads.join("From phone"))
}

/// This computer's address on the home network: the one used to reach the router.
fn lan_address() -> Option<Ipv4Addr> {
    // Connecting a UDP socket sends nothing; it only makes the system choose the outgoing address.
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("192.168.0.1:80").ok()?;
    match sock.local_addr().ok()?.ip() {
        IpAddr::V4(v4) if rules::is_shareable_address(v4) => Some(v4),
        _ => {
            sock.connect("10.0.0.1:80").ok()?;
            match sock.local_addr().ok()?.ip() {
                IpAddr::V4(v4) if rules::is_shareable_address(v4) => Some(v4),
                _ => None,
            }
        }
    }
}

fn qr_svg(url: &str) -> Option<String> {
    let code = qrcode::QrCode::new(url.as_bytes()).ok()?;
    Some(
        code.render::<qrcode::render::svg::Color>()
            .min_dimensions(220, 220)
            .quiet_zone(true)
            .dark_color(qrcode::render::svg::Color("#000000"))
            .light_color(qrcode::render::svg::Color("#ffffff"))
            .build(),
    )
}

fn status(state: &AppState) -> Status {
    let idle_minutes = state.settings.get().phone.idle_minutes;
    let guard = state.phone.running.lock().unwrap();
    match guard.as_ref() {
        Some(r) => Status {
            running: true,
            url: Some(r.url.clone()),
            qr: qr_svg(&r.url),
            dest: r.dest.to_string_lossy().into_owned(),
            idle_minutes,
            offered: r.offered.lock().unwrap().clone(),
            received: r.received.lock().unwrap().clone(),
        },
        None => Status {
            running: false,
            url: None,
            qr: None,
            dest: default_dest(state).to_string_lossy().into_owned(),
            idle_minutes,
            offered: Vec::new(),
            received: Vec::new(),
        },
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("a valid header")
}

fn plain(code: u16, text: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(text.to_string())
        .with_status_code(StatusCode(code))
        .with_header(header("Content-Type", "text/plain; charset=utf-8"))
}

struct Ctx {
    app: AppHandle,
    token: String,
    dest: PathBuf,
    offered: Arc<Mutex<Vec<Offered>>>,
    received: Arc<Mutex<Vec<Received>>>,
    last_activity: Arc<AtomicI64>,
}

fn handle(ctx: &Ctx, request: Request) {
    // Only the home network, and never anything else.
    let local = request
        .remote_addr()
        .map(|a| rules::is_local_client(a.ip()))
        .unwrap_or(false);
    if !local {
        let _ = request.respond(plain(403, "Not allowed."));
        return;
    }
    let url = request.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
    let mut parts = path.trim_matches('/').splitn(3, '/');
    let first = parts.next().unwrap_or("");
    if !rules::same_secret(first, &ctx.token) {
        let _ = request.respond(plain(404, "Not found."));
        return;
    }
    ctx.last_activity.store(now(), Ordering::Relaxed);
    let action = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    let method = request.method().clone();

    match (method, action) {
        (Method::Get, "") => {
            let _ = request.respond(
                Response::from_string(PAGE)
                    .with_header(header("Content-Type", "text/html; charset=utf-8"))
                    .with_header(header("Cache-Control", "no-store"))
                    .with_header(header("Referrer-Policy", "no-referrer")),
            );
        }
        (Method::Get, "list") => {
            let offered: Vec<serde_json::Value> = ctx
                .offered
                .lock()
                .unwrap()
                .iter()
                .map(|o| serde_json::json!({ "id": o.id, "name": o.name, "size": o.size }))
                .collect();
            let body = serde_json::json!({ "offered": offered }).to_string();
            let _ = request.respond(
                Response::from_string(body)
                    .with_header(header("Content-Type", "application/json"))
                    .with_header(header("Cache-Control", "no-store")),
            );
        }
        (Method::Get, "get") => {
            let found = ctx
                .offered
                .lock()
                .unwrap()
                .iter()
                .find(|o| o.id == rest)
                .cloned();
            match found.and_then(|o| std::fs::File::open(&o.path).ok().map(|f| (o, f))) {
                Some((o, file)) => {
                    let len = file.metadata().map(|m| m.len()).ok();
                    let response = Response::new(
                        StatusCode(200),
                        vec![
                            header("Content-Type", "application/octet-stream"),
                            header("Content-Disposition", &rules::disposition(&o.name)),
                            header("Cache-Control", "no-store"),
                        ],
                        file,
                        len.map(|l| l as usize),
                        None,
                    );
                    let _ = request.respond(response);
                }
                None => {
                    let _ = request.respond(plain(404, "That file isn't shared any more."));
                }
            }
        }
        (Method::Put, "put") => receive(ctx, request, query),
        (Method::Post, "done") => {
            let n: usize = rules::query_value(query, "count")
                .and_then(|c| c.parse().ok())
                .unwrap_or(0)
                .min(10_000);
            if n > 0 {
                let lang = crate::i18n::lang_of(&ctx.app.state::<AppState>().settings.get());
                let (title, body) = crate::i18n::phone_text(lang, n, &ctx.dest.to_string_lossy());
                toast::show(&ctx.app, Toast::new("success", title, body).opens("phone"));
            }
            let _ = request.respond(plain(200, "ok"));
        }
        _ => {
            let _ = request.respond(plain(404, "Not found."));
        }
    }
}

/// A file from the phone, saved under a name that is not taken. Never overwrites anything.
fn receive(ctx: &Ctx, mut request: Request, query: &str) {
    let name = rules::safe_name(&rules::query_value(query, "name").unwrap_or_default());
    let length = request.body_length().unwrap_or(0) as u64;
    if length > rules::MAX_MB * 1024 * 1024 {
        let _ = request.respond(plain(413, "That file is too big."));
        return;
    }
    if std::fs::create_dir_all(&ctx.dest).is_err() {
        let _ = request.respond(plain(500, "The computer couldn't make the folder."));
        return;
    }
    let target = rules::free_path(&ctx.dest, &name);
    let temp = target.with_extension("nomnom-part");
    let result = (|| -> std::io::Result<u64> {
        let mut file = std::fs::File::create(&temp)?;
        let mut reader = request.as_reader().take(rules::MAX_MB * 1024 * 1024);
        let n = std::io::copy(&mut reader, &mut file)?;
        file.sync_all()?;
        Ok(n)
    })();
    match result {
        Ok(size) if length == 0 || size == length => {
            if std::fs::rename(&temp, &target).is_err() {
                let _ = std::fs::remove_file(&temp);
                let _ = request.respond(plain(500, "The computer couldn't save it."));
                return;
            }
            ctx.received.lock().unwrap().insert(
                0,
                Received {
                    name: target
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or(name),
                    path: target.to_string_lossy().into_owned(),
                    size,
                    at: now(),
                },
            );
            let _ = ctx.app.emit("phone://changed", ());
            let _ = request.respond(plain(200, "ok"));
        }
        _ => {
            let _ = std::fs::remove_file(&temp);
            let _ = request.respond(plain(500, "It didn't arrive in full."));
        }
    }
}

fn start_server(app: &AppHandle, state: &AppState) -> CmdResult<Running> {
    let ip = lan_address()
        .ok_or("NomNom couldn't find your Wi-Fi or network address. Connect to the same network as your phone.")?;
    let server = Server::http((ip, 0)).map_err(|e| format!("Couldn't start sharing: {e}"))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or("Couldn't start sharing.")?;
    let token = uuid::Uuid::new_v4().simple().to_string();
    let url = format!("http://{ip}:{port}/{token}/");
    let dest = default_dest(state);
    let stop = Arc::new(AtomicBool::new(false));
    let offered = Arc::new(Mutex::new(Vec::new()));
    let received = Arc::new(Mutex::new(Vec::new()));
    let last_activity = Arc::new(AtomicI64::new(now()));
    let ctx = Arc::new(Ctx {
        app: app.clone(),
        token,
        dest: dest.clone(),
        offered: offered.clone(),
        received: received.clone(),
        last_activity: last_activity.clone(),
    });
    let stop_flag = stop.clone();
    std::thread::spawn(move || {
        while !stop_flag.load(Ordering::Relaxed) {
            match server.recv_timeout(Duration::from_millis(400)) {
                Ok(Some(request)) => {
                    let ctx = ctx.clone();
                    std::thread::spawn(move || handle(&ctx, request));
                }
                Ok(None) => {}
                Err(_) => break,
            }
        }
    });
    Ok(Running {
        stop,
        url,
        dest,
        offered,
        received,
        last_activity,
    })
}

fn stop_running(state: &AppState) -> bool {
    match state.phone.running.lock().unwrap().take() {
        Some(r) => {
            r.stop.store(true, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

/// Turns sharing off after a quiet while.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(20)).await;
            let state = app.state::<AppState>();
            let idle = i64::from(state.settings.get().phone.idle_minutes.max(1)) * 60;
            let expired = state
                .phone
                .running
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|r| now() - r.last_activity.load(Ordering::Relaxed) > idle);
            if expired && stop_running(&state) {
                let _ = app.emit("phone://changed", ());
                let lang = crate::i18n::lang_of(&state.settings.get());
                toast::show(
                    &app,
                    Toast::new(
                        "info",
                        crate::i18n::tr(lang, "Sharing with your phone stopped"),
                        crate::i18n::tr(
                            lang,
                            "Nobody used it for a while, so NomNom turned it off.",
                        ),
                    ),
                );
            }
        }
    });
}

#[tauri::command]
pub fn ph_status(state: State<'_, AppState>) -> Status {
    status(&state)
}

#[tauri::command]
pub fn ph_start(app: AppHandle, state: State<'_, AppState>) -> CmdResult<Status> {
    if state.phone.running.lock().unwrap().is_none() {
        let running = start_server(&app, &state)?;
        *state.phone.running.lock().unwrap() = Some(running);
    }
    Ok(status(&state))
}

#[tauri::command]
pub fn ph_stop(app: AppHandle, state: State<'_, AppState>) -> Status {
    stop_running(&state);
    let _ = app.emit("phone://changed", ());
    status(&state)
}

/// Offers files to the phone. Only regular files; the phone can fetch only what is in this list.
#[tauri::command]
pub fn ph_offer(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> CmdResult<Status> {
    {
        let guard = state.phone.running.lock().unwrap();
        let running = guard.as_ref().ok_or("Start sharing first.")?;
        let mut list = running.offered.lock().unwrap();
        for p in paths.iter().take(500) {
            let path = PathBuf::from(p);
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            if !path.is_absolute() || !meta.is_file() || list.iter().any(|o| o.path == path) {
                continue;
            }
            list.push(Offered {
                id: uuid::Uuid::new_v4().simple().to_string(),
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "file".into()),
                size: meta.len(),
                path,
            });
        }
    }
    let _ = app.emit("phone://changed", ());
    Ok(status(&state))
}

#[tauri::command]
pub fn ph_unoffer(app: AppHandle, state: State<'_, AppState>, id: String) -> Status {
    if let Some(r) = state.phone.running.lock().unwrap().as_ref() {
        r.offered.lock().unwrap().retain(|o| o.id != id);
    }
    let _ = app.emit("phone://changed", ());
    status(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phone_page_never_talks_to_anyone_but_this_computer() {
        // Every address in the page is relative to where it came from.
        assert!(!PAGE.contains("http://") && !PAGE.contains("https://"));
        assert!(
            PAGE.contains("encodeURIComponent"),
            "names are encoded in the link"
        );
    }

    #[test]
    fn the_qr_code_is_a_picture() {
        let svg = qr_svg("http://192.168.1.2:5000/abcdef/").unwrap();
        assert!(svg.starts_with("<?xml") || svg.contains("<svg"));
    }
}
