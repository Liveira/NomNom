//! NomNom Mobile Manager: Android device hub over ADB + scrcpy (USB and Wi-Fi).
//!
//! Runs ADB and scrcpy commands off the main UI thread with hidden consoles on Windows,
//! caches pulled photo previews locally, and persists timeline events, deck actions, and
//! muted notification apps in SQLite.

use crate::state::AppState;
use crate::toast::{self, Toast};
use nomnom_tools::{InstallProgress, Tool};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

type CmdResult<T> = Result<T, String>;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Default)]
pub struct MobileState {
    installing: AtomicBool,
    audio_proc: Mutex<Option<(String, Child)>>,
    mic_proc: Mutex<Option<(String, Child)>>,
    webcam_proc: Mutex<Option<(String, Child)>>,
    rec_proc: Mutex<Option<(String, PathBuf, Child)>>,
    qr_cancel: Mutex<Option<Arc<AtomicBool>>>,
    torch_state: Mutex<HashMap<String, bool>>,
    /// Locally dismissed notification keys (some Android versions disallow shell cancel).
    dismissed_keys: Mutex<HashSet<String>>,
}

impl MobileState {
    pub fn shutdown(&self) {
        if let Ok(mut g) = self.qr_cancel.lock() {
            if let Some(flag) = g.take() {
                flag.store(true, Ordering::SeqCst);
            }
        }
        for slot in [&self.audio_proc, &self.mic_proc, &self.webcam_proc] {
            if let Ok(mut g) = slot.lock() {
                if let Some((_, mut c)) = g.take() {
                    let _ = c.kill();
                }
            }
        }
        if let Ok(mut g) = self.rec_proc.lock() {
            if let Some((_, _, mut c)) = g.take() {
                let _ = c.kill();
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileToolsStatus {
    pub adb_found: bool,
    pub scrcpy_found: bool,
    pub adb_path: Option<String>,
    pub scrcpy_path: Option<String>,
    pub adb_version: Option<String>,
    pub scrcpy_version: Option<String>,
    pub installing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MobileDevice {
    pub serial: String,
    pub state: String,
    pub transport: String,
    pub model: String,
    pub product: String,
    pub wifi_ip: Option<String>,
    pub latency_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BatteryInfo {
    pub level: u8,
    pub charging: bool,
    pub charge_type: String,
    pub temp_c: f32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ControlsState {
    pub dnd: bool,
    pub torch: bool,
    pub wifi: bool,
    pub bluetooth: bool,
    pub hotspot: bool,
    pub audio_forwarding: bool,
    pub mic_active: bool,
    pub webcam_active: bool,
    pub recording_active: bool,
    pub dark_mode: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SoundDisplayState {
    pub media_vol: u8,
    pub media_max: u8,
    pub ring_vol: u8,
    pub ring_max: u8,
    pub alarm_vol: u8,
    pub alarm_max: u8,
    pub notif_vol: u8,
    pub notif_max: u8,
    pub brightness: u8,
    pub auto_brightness: bool,
    pub screen_timeout_ms: u32,
}

impl Default for SoundDisplayState {
    fn default() -> Self {
        Self {
            media_vol: 7,
            media_max: 15,
            ring_vol: 5,
            ring_max: 7,
            alarm_vol: 5,
            alarm_max: 7,
            notif_vol: 5,
            notif_max: 7,
            brightness: 128,
            auto_brightness: true,
            screen_timeout_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SoundDisplayPatch {
    pub media_vol: Option<u8>,
    pub ring_vol: Option<u8>,
    pub alarm_vol: Option<u8>,
    pub notif_vol: Option<u8>,
    pub brightness: Option<u8>,
    pub auto_brightness: Option<bool>,
    pub screen_timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaInfo {
    pub app: String,
    pub package: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub position_ms: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhoneNotification {
    pub key: String,
    pub package: String,
    pub app_name: String,
    pub title: String,
    pub text: String,
    pub post_time_ms: i64,
    pub can_reply: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceSnapshot {
    pub serial: String,
    pub name: String,
    pub manufacturer: String,
    pub android_version: String,
    pub sdk_level: u32,
    pub screen_size: String,
    pub transport: String,
    pub wifi_ip: Option<String>,
    pub latency_ms: Option<u32>,
    pub battery: BatteryInfo,
    pub controls: ControlsState,
    pub sound_display: SoundDisplayState,
    pub media: Option<MediaInfo>,
    pub notifications: Vec<PhoneNotification>,
    pub muted_packages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MirrorOptions {
    pub max_size: Option<u32>,
    pub bit_rate_mbps: Option<u32>,
    pub max_fps: Option<u32>,
    pub video_codec: Option<String>,
    pub turn_screen_off: Option<bool>,
    pub stay_awake: Option<bool>,
    pub audio: Option<bool>,
    pub start_app: Option<String>,
    pub new_display: Option<bool>,
    pub display_size: Option<String>,
    pub display_dpi: Option<u32>,
    pub display_id: Option<u32>,
    pub keep_alive_on_close: Option<bool>,
    pub no_decorations: Option<bool>,
    pub keyboard_uhid: Option<bool>,
    pub mouse_uhid: Option<bool>,
    pub gamepad_uhid: Option<bool>,
    pub borderless: Option<bool>,
    pub always_on_top: Option<bool>,
    pub video_buffer_ms: Option<u32>,
    pub audio_buffer_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraInfo {
    pub id: String,
    pub facing: String,
    pub label: String,
    pub sizes: Vec<String>,
    pub fps: Vec<u32>,
    pub high_speed: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraCatalog {
    pub cameras: Vec<CameraInfo>,
    pub v4l2_devices: Vec<String>,
    pub obs_path: Option<String>,
    pub supports_usb_uvc: bool,
    pub sdk_level: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WebcamProOptions {
    pub camera_id: Option<String>,
    pub facing: Option<String>,
    pub size: Option<String>,
    pub aspect_ratio: Option<String>,
    pub fps: Option<u32>,
    pub high_speed: Option<bool>,
    pub video_codec: Option<String>,
    pub bit_rate_mbps: Option<u32>,
    pub orientation: Option<u32>,
    pub flip: Option<bool>,
    pub lock_orientation: Option<bool>,
    pub audio_mic: Option<bool>,
    pub borderless: Option<bool>,
    pub always_on_top: Option<bool>,
    pub record: Option<bool>,
    pub video_buffer_ms: Option<u32>,
    pub audio_buffer_ms: Option<u32>,
    #[allow(dead_code)]
    pub v4l2_sink: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MdnsService {
    pub name: String,
    pub service_type: String,
    pub endpoint: String,
    pub ip: String,
    pub port: u16,
    /// `"connect"` for `_adb-tls-connect._tcp`, `"pair"` for `_adb-tls-pairing._tcp`.
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedWifiDevice {
    pub endpoint: String,
    pub name: String,
    pub ip: String,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WifiDiscovery {
    pub services: Vec<MdnsService>,
    pub saved: Vec<SavedWifiDevice>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WifiQrSession {
    pub service: String,
    pub password: String,
    pub payload: String,
    pub qr_svg: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub id: u32,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhoneContact {
    pub name: String,
    pub number: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEntry {
    pub id: i64,
    pub device_id: String,
    pub device_name: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckItem {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub kind: String,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmsMessage {
    pub id: String,
    pub address: String,
    pub body: String,
    pub date_ms: i64,
    pub incoming: bool,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallLogEntry {
    pub id: String,
    pub number: String,
    pub name: String,
    pub date_ms: i64,
    pub duration_secs: u64,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhonePhoto {
    pub path: String,
    pub name: String,
    pub folder: String,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhoneApp {
    pub package: String,
    pub name: String,
    pub system: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FsListing {
    pub dir: String,
    pub entries: Vec<FsEntry>,
    pub storage_total_kb: u64,
    pub storage_used_kb: u64,
    pub storage_avail_kb: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub model: String,
    pub manufacturer: String,
    pub brand: String,
    pub device: String,
    pub android_version: String,
    pub sdk_level: u32,
    pub security_patch: String,
    pub build_id: String,
    pub kernel: String,
    pub abi: String,
    pub hardware: String,
    pub cpu_cores: u32,
    pub ram_total_kb: u64,
    pub ram_avail_kb: u64,
    pub storage_total_kb: u64,
    pub storage_used_kb: u64,
    pub storage_avail_kb: u64,
    pub battery_level: u8,
    pub battery_temp_c: f32,
    pub battery_voltage_mv: u32,
    pub battery_health: String,
    pub battery_tech: String,
    pub battery_charging: bool,
    pub wifi_ssid: String,
    pub wifi_ip: String,
    pub carrier: String,
    pub airplane_mode: bool,
    pub screen_size: String,
    pub screen_density: String,
    pub dark_mode: bool,
    pub anim_scale: String,
    pub show_touches: bool,
    pub stay_awake_plugged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortTunnel {
    pub direction: String,
    pub local: String,
    pub remote: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapturedImage {
    pub path: String,
    pub name: String,
    pub data_url: String,
}

fn hidden_cmd(bin: &Path) -> Command {
    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn locate_adb(state: &AppState) -> CmdResult<PathBuf> {
    state.tools.locate(Tool::Adb).ok_or_else(|| {
        "ADB is not installed yet. Click Install Android tools to download ADB and scrcpy.".into()
    })
}

fn locate_scrcpy(state: &AppState) -> CmdResult<PathBuf> {
    state.tools.locate(Tool::Scrcpy).ok_or_else(|| {
        "scrcpy is not installed yet. Click Install Android tools to download ADB and scrcpy."
            .into()
    })
}

fn run_adb_raw(adb: &Path, args: &[&str], timeout_secs: u64) -> CmdResult<Vec<u8>> {
    let mut child = hidden_cmd(adb)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't start adb: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("Timed out waiting for the phone to answer.".into());
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(format!("adb failed: {e}")),
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("Couldn't read adb output: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let msg = if !err.is_empty() { err } else { stdout };
        return Err(if msg.is_empty() {
            "The phone command failed.".into()
        } else {
            msg
        });
    }
    Ok(out.stdout)
}

fn run_adb(adb: &Path, args: &[&str], timeout_secs: u64) -> CmdResult<String> {
    let bytes = run_adb_raw(adb, args, timeout_secs)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn adb_shell(adb: &Path, serial: &str, shell_cmd: &str, timeout_secs: u64) -> CmdResult<String> {
    run_adb(adb, &["-s", serial, "shell", shell_cmd], timeout_secs)
}

fn measure_tcp_latency(addr: &str) -> Option<u32> {
    let parsed: SocketAddr = addr.parse().ok()?;
    let start = Instant::now();
    TcpStream::connect_timeout(&parsed, Duration::from_millis(600)).ok()?;
    Some(start.elapsed().as_millis().max(1) as u32)
}

fn is_wifi_serial(serial: &str) -> bool {
    serial.contains(':') || serial.contains("._adb")
}

fn extract_ip_from_serial(serial: &str) -> Option<String> {
    let (host, _) = serial.split_once(':')?;
    host.parse::<std::net::Ipv4Addr>().ok()?;
    Some(host.to_string())
}

fn parse_devices_output(text: &str) -> Vec<MobileDevice> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('*') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(serial) = parts.next() else {
            continue;
        };
        let Some(state) = parts.next() else {
            continue;
        };
        let mut model = String::new();
        let mut product = String::new();
        for kv in parts {
            if let Some(v) = kv.strip_prefix("model:") {
                model = v.replace('_', " ");
            } else if let Some(v) = kv.strip_prefix("product:") {
                product = v.replace('_', " ");
            }
        }
        let wifi = is_wifi_serial(serial);
        let wifi_ip = extract_ip_from_serial(serial);
        let latency_ms = if wifi && state == "device" {
            measure_tcp_latency(serial)
        } else {
            None
        };
        out.push(MobileDevice {
            serial: serial.to_string(),
            state: state.to_string(),
            transport: if wifi { "wifi".into() } else { "usb".into() },
            model: if model.is_empty() {
                serial.to_string()
            } else {
                model
            },
            product,
            wifi_ip,
            latency_ms,
        });
    }
    out
}

fn parse_battery(text: &str) -> BatteryInfo {
    let mut info = BatteryInfo::default();
    let mut ac = false;
    let mut usb = false;
    let mut wireless = false;
    let mut status_code = 0i32;
    for line in text.lines() {
        let l = line.trim();
        if let Some(v) = l.strip_prefix("level:") {
            info.level = v.trim().parse::<u8>().unwrap_or(0).min(100);
        } else if let Some(v) = l.strip_prefix("AC powered:") {
            ac = v.trim().eq_ignore_ascii_case("true");
        } else if let Some(v) = l.strip_prefix("USB powered:") {
            usb = v.trim().eq_ignore_ascii_case("true");
        } else if let Some(v) = l.strip_prefix("Wireless powered:") {
            wireless = v.trim().eq_ignore_ascii_case("true");
        } else if let Some(v) = l.strip_prefix("status:") {
            status_code = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = l.strip_prefix("temperature:") {
            if let Ok(t) = v.trim().parse::<f32>() {
                info.temp_c = (t / 10.0 * 10.0).round() / 10.0;
            }
        }
    }
    // Android BatteryManager: 2 = CHARGING, 5 = FULL
    info.charging = ac || usb || wireless || status_code == 2;
    info.charge_type = if wireless {
        "Wireless charging".into()
    } else if ac {
        "Charging".into()
    } else if usb {
        "USB charging".into()
    } else if status_code == 5 {
        "Full".into()
    } else {
        "On battery".into()
    };
    info
}

fn friendly_app_name(pkg: &str) -> String {
    let known = match pkg {
        "com.whatsapp" | "com.whatsapp.w4b" => Some("WhatsApp"),
        "org.telegram.messenger" | "org.thunderdog.challegram" => Some("Telegram"),
        "com.instagram.android" => Some("Instagram"),
        "com.discord" => Some("Discord"),
        "com.spotify.music" => Some("Spotify"),
        "com.google.android.youtube" => Some("YouTube"),
        "com.google.android.apps.youtube.music" => Some("YouTube Music"),
        "com.google.android.gm" => Some("Gmail"),
        "com.google.android.apps.messaging" | "com.samsung.android.messaging" => Some("Messages"),
        "com.twitter.android" => Some("X"),
        "com.facebook.orca" => Some("Messenger"),
        "com.facebook.katana" => Some("Facebook"),
        "com.zhiliaoapp.musically" | "com.ss.android.ugc.trill" => Some("TikTok"),
        "com.netflix.mediaclient" => Some("Netflix"),
        "com.reddit.frontpage" => Some("Reddit"),
        "ai.x.grok" => Some("Grok"),
        "com.openai.chatgpt" => Some("ChatGPT"),
        "com.android.chrome" => Some("Chrome"),
        "org.mozilla.firefox" => Some("Firefox"),
        "com.sec.android.app.camera" | "com.google.android.GoogleCamera" => Some("Camera"),
        "com.sec.android.gallery3d" | "com.google.android.apps.photos" => Some("Photos"),
        "com.samsung.android.dialer" | "com.google.android.dialer" => Some("Phone"),
        "com.google.android.apps.maps" => Some("Maps"),
        "com.amazon.mp3" => Some("Amazon Music"),
        "com.apple.android.music" => Some("Apple Music"),
        "com.soundcloud.android" => Some("SoundCloud"),
        _ => None,
    };
    if let Some(name) = known {
        return name.to_string();
    }
    let last = pkg
        .rsplit('.')
        .find(|seg| !matches!(*seg, "android" | "app" | "mobile" | "client" | "release"))
        .unwrap_or(pkg);
    let mut chars = last.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => pkg.to_string(),
    }
}

fn parse_media_session(text: &str) -> Option<MediaInfo> {
    let mut best: Option<MediaInfo> = None;
    let mut cur_pkg = String::new();
    let mut cur_active = false;
    let mut cur_playing = false;
    let mut cur_pos = 0i64;
    let mut cur_title = String::new();
    let mut cur_artist = String::new();
    let mut cur_dur = 0i64;

    let flush = |best: &mut Option<MediaInfo>,
                 pkg: &str,
                 active: bool,
                 playing: bool,
                 pos: i64,
                 title: &str,
                 artist: &str,
                 dur: i64| {
        if pkg.is_empty() || title.is_empty() {
            return;
        }
        let candidate = MediaInfo {
            app: friendly_app_name(pkg),
            package: pkg.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            playing,
            position_ms: pos.max(0),
            duration_ms: dur.max(0),
        };
        let replace = match best {
            None => true,
            Some(prev) => (playing && !prev.playing) || (active && !prev.playing),
        };
        if replace {
            *best = Some(candidate);
        }
    };

    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("package=") {
            flush(
                &mut best,
                &cur_pkg,
                cur_active,
                cur_playing,
                cur_pos,
                &cur_title,
                &cur_artist,
                cur_dur,
            );
            cur_pkg = rest.split_whitespace().next().unwrap_or("").to_string();
            cur_active = false;
            cur_playing = false;
            cur_pos = 0;
            cur_title.clear();
            cur_artist.clear();
            cur_dur = 0;
        } else if line.starts_with("active=true") {
            cur_active = true;
        } else if line.starts_with("state=PlaybackState") {
            // state=PlaybackState {state=3, position=215000, ...} or state=PLAYING(3)
            cur_playing = line.contains("state=3") || line.contains("PLAYING");
            if let Some(pos_part) = line.split("position=").nth(1) {
                let num: String = pos_part
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '-')
                    .collect();
                cur_pos = num.parse().unwrap_or(0);
            }
        } else if let Some(meta) = line.strip_prefix("metadata:") {
            // metadata: size=5, description=Intro (Infected), Sickick, null
            if let Some(desc) = meta.split("description=").nth(1) {
                let parts: Vec<&str> = desc.split(", ").collect();
                let t = parts.first().copied().unwrap_or("").trim();
                let a = parts.get(1).copied().unwrap_or("").trim();
                if !t.is_empty() && t != "null" {
                    cur_title = t.to_string();
                }
                if !a.is_empty() && a != "null" {
                    cur_artist = a.to_string();
                }
            }
        } else if let Some(v) = line.strip_prefix("android.media.metadata.DURATION=") {
            cur_dur = v.trim().parse().unwrap_or(0);
        }
    }
    flush(
        &mut best,
        &cur_pkg,
        cur_active,
        cur_playing,
        cur_pos,
        &cur_title,
        &cur_artist,
        cur_dur,
    );
    best
}

fn is_system_noise_package(pkg: &str) -> bool {
    matches!(
        pkg,
        "android"
            | "com.android.systemui"
            | "com.android.providers.downloads"
            | "com.android.vending"
            | "com.google.android.gms"
            | "com.samsung.android.app.smartcapture"
            | "com.sec.android.app.launcher"
    )
}

fn parse_notifications(
    text: &str,
    muted: &HashSet<String>,
    dismissed: &HashSet<String>,
) -> Vec<PhoneNotification> {
    let mut out = Vec::new();
    let mut seen_keys = HashSet::new();

    let mut cur_key = String::new();
    let mut cur_pkg = String::new();
    let mut cur_title = String::new();
    let mut cur_text = String::new();
    let mut cur_time = 0i64;
    let mut cur_ongoing = false;
    let mut cur_can_reply = false;
    let mut in_record = false;

    let flush = |out: &mut Vec<PhoneNotification>,
                 seen: &mut HashSet<String>,
                 key: &str,
                 pkg: &str,
                 title: &str,
                 body: &str,
                 post_time: i64,
                 ongoing: bool,
                 can_reply: bool| {
        if pkg.is_empty() || key.is_empty() || ongoing || is_system_noise_package(pkg) {
            return;
        }
        if muted.contains(pkg) || dismissed.contains(key) {
            return;
        }
        let clean_title = title.trim();
        let clean_body = body.trim();
        if clean_title.is_empty() && clean_body.is_empty() {
            return;
        }
        if !seen.insert(key.to_string()) {
            return;
        }
        out.push(PhoneNotification {
            key: key.to_string(),
            package: pkg.to_string(),
            app_name: friendly_app_name(pkg),
            title: if clean_title.is_empty() {
                friendly_app_name(pkg)
            } else {
                clean_title.to_string()
            },
            text: clean_body.to_string(),
            post_time_ms: post_time,
            can_reply,
        });
    };

    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("NotificationRecord(") {
            if in_record {
                flush(
                    &mut out,
                    &mut seen_keys,
                    &cur_key,
                    &cur_pkg,
                    &cur_title,
                    &cur_text,
                    cur_time,
                    cur_ongoing,
                    cur_can_reply,
                );
            }
            in_record = true;
            cur_key.clear();
            cur_pkg.clear();
            cur_title.clear();
            cur_text.clear();
            cur_time = 0;
            cur_ongoing = false;
            cur_can_reply = false;

            if let Some(pkg_part) = line.split("pkg=").nth(1) {
                cur_pkg = pkg_part
                    .split(|c: char| c.is_whitespace() || c == ')')
                    .next()
                    .unwrap_or("")
                    .to_string();
            }
            if let Some(key_part) = line.split("key=").nth(1) {
                cur_key = key_part
                    .split(|c: char| c.is_whitespace() || c == ')')
                    .next()
                    .unwrap_or("")
                    .to_string();
            }
            continue;
        }
        if !in_record {
            continue;
        }
        if line.contains("FLAG_ONGOING_EVENT") || line.contains("FLAG_FOREGROUND_SERVICE") {
            cur_ongoing = true;
        }
        if line.contains("RemoteInput") || line.contains("action=Reply") {
            cur_can_reply = true;
        }
        if let Some(rest) = line.strip_prefix("postTime=") {
            cur_time = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if let Some(val) = extract_extra_value(line, "android.title=") {
            if cur_title.is_empty() {
                cur_title = val;
            }
        } else if let Some(val) = extract_extra_value(line, "android.bigText=") {
            cur_text = val;
        } else if let Some(val) = extract_extra_value(line, "android.text=") {
            if cur_text.is_empty() {
                cur_text = val;
            }
        }
    }
    if in_record {
        flush(
            &mut out,
            &mut seen_keys,
            &cur_key,
            &cur_pkg,
            &cur_title,
            &cur_text,
            cur_time,
            cur_ongoing,
            cur_can_reply,
        );
    }
    out.sort_by(|a, b| b.post_time_ms.cmp(&a.post_time_ms));
    out.truncate(30);
    out
}

fn extract_extra_value(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    // Format is usually: `String (Hello world)` or `SpannableString (Hello)` or plain value
    let cleaned = if let Some(open) = rest.find('(') {
        let inside = &rest[open + 1..];
        inside.strip_suffix(')').unwrap_or(inside)
    } else {
        rest
    };
    let trimmed = cleaned.trim();
    if trimmed.is_empty() || trimmed == "null" {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_content_rows(text: &str) -> Vec<HashMap<String, String>> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("Row:") else {
            continue;
        };
        // Skip row index number
        let Some((_, body)) = rest.trim_start().split_once(' ') else {
            continue;
        };
        let mut map = HashMap::new();
        // Split by `, ` when followed by `key=`
        let mut current_key = String::new();
        let mut current_val = String::new();
        for token in body.split(", ") {
            if let Some((k, v)) = token.split_once('=') {
                if k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    if !current_key.is_empty() {
                        map.insert(current_key, current_val);
                    }
                    current_key = k.to_string();
                    current_val = v.to_string();
                    continue;
                }
            }
            if !current_key.is_empty() {
                current_val.push_str(", ");
                current_val.push_str(token);
            }
        }
        if !current_key.is_empty() {
            map.insert(current_key, current_val);
        }
        if !map.is_empty() {
            rows.push(map);
        }
    }
    rows
}

fn parse_stream_volume(audio_dump: &str, stream_header: &str) -> Option<(u8, u8)> {
    let idx = audio_dump.find(stream_header)?;
    let slice = &audio_dump[idx..audio_dump.len().min(idx + 500)];
    let mut max_v = 15u8;
    let mut cur_v = 7u8;
    for line in slice.lines().skip(1).take(12) {
        let l = line.trim();
        if l.starts_with("- STREAM_") {
            break;
        }
        if let Some(rest) = l.strip_prefix("Max:") {
            if let Ok(n) = rest.trim().parse::<u8>() {
                max_v = n.max(1);
            }
        } else if l.starts_with("Current:") {
            // Current: 2 (speaker): 8, 40000000 (default): 8
            if let Some(part) = l.split("speaker):").nth(1).or_else(|| l.split("default):").nth(1)) {
                let digits: String = part
                    .trim()
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(n) = digits.parse::<u8>() {
                    cur_v = n;
                }
            } else if let Some(part) = l.split(':').last() {
                let digits: String = part
                    .trim()
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(n) = digits.parse::<u8>() {
                    cur_v = n;
                }
            }
        }
    }
    Some((cur_v.min(max_v), max_v))
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Escapes user text for `adb shell input text <escaped>`.
fn escape_adb_input_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        match ch {
            ' ' => out.push_str("%s"),
            '\\' | '"' | '\'' | '&' | '|' | ';' | '(' | ')' | '<' | '>' | '$' | '`' | '*'
            | '?' | '~' | '#' | '[' | ']' | '{' | '}' | '!' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' | '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn record_timeline(
    state: &AppState,
    app: &AppHandle,
    device_id: &str,
    device_name: &str,
    kind: &str,
    title: &str,
    detail: &str,
) {
    let _ = state.db.with(|c| {
        c.execute(
            "INSERT INTO mobile_timeline(device_id, device_name, kind, title, detail, at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![device_id, device_name, kind, title, detail, now()],
        )?;
        c.execute(
            "DELETE FROM mobile_timeline WHERE id NOT IN (SELECT id FROM mobile_timeline ORDER BY at DESC LIMIT 120)",
            [],
        )
    });
    let _ = app.emit("mobile://timeline", ());
}

fn device_display_name(adb: &Path, serial: &str) -> String {
    let script = "for p in ro.product.marketname ro.config.marketing_name ro.product.vendor.marketname ro.product.model; do v=$(getprop $p); if [ -n \"$v\" ]; then echo \"$v\"; break; fi; done";
    adb_shell(adb, serial, script, 3)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| serial.to_string())
}

fn phone_save_dir(state: &AppState) -> PathBuf {
    state
        .settings
        .get()
        .phone
        .save_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| state.paths.downloads.join("From phone"))
}

fn is_child_alive(slot: &Mutex<Option<(String, Child)>>, serial: &str) -> bool {
    let Ok(mut g) = slot.lock() else {
        return false;
    };
    if let Some((s, child)) = g.as_mut() {
        match child.try_wait() {
            Ok(None) => return s == serial,
            _ => {
                *g = None;
            }
        }
    }
    false
}

fn muted_set(state: &AppState) -> HashSet<String> {
    state
        .db
        .with(|c| {
            let mut st = c.prepare("SELECT package_name FROM mobile_muted")?;
            let rows = st.query_map([], |r| r.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<HashSet<_>>>()
        })
        .unwrap_or_default()
}

fn format_duration_short(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("{m}m")
        } else {
            format!("{m}m {s}s")
        }
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{h}h {m}m")
    }
}

#[tauri::command]
pub async fn mob_tools_status(state: State<'_, AppState>) -> CmdResult<MobileToolsStatus> {
    let adb = state.tools.status(Tool::Adb).await;
    let scrcpy = state.tools.status(Tool::Scrcpy).await;
    Ok(MobileToolsStatus {
        adb_found: adb.path.is_some(),
        scrcpy_found: scrcpy.path.is_some(),
        adb_path: adb.path.map(|p| p.to_string_lossy().into_owned()),
        scrcpy_path: scrcpy.path.map(|p| p.to_string_lossy().into_owned()),
        adb_version: adb.version,
        scrcpy_version: scrcpy.version,
        installing: state.mobile.installing.load(Ordering::Relaxed),
    })
}

#[tauri::command]
pub async fn mob_install_tools(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<MobileToolsStatus> {
    if state
        .mobile
        .installing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Android tools are already being installed.".into());
    }
    let tools = state.tools.clone();
    let app_clone = app.clone();
    let progress: Arc<InstallProgress> = Arc::new(move |done, total| {
        let _ = app_clone.emit(
            "mobile://install-progress",
            serde_json::json!({ "done": done, "total": total }),
        );
    });
    let res = tools.install_scrcpy(progress.as_ref()).await;
    state.mobile.installing.store(false, Ordering::SeqCst);
    res.map_err(|e| e.to_string())?;
    toast::show(
        &app,
        Toast::new(
            "success",
            "Android tools ready",
            "ADB and scrcpy are installed and ready for Mobile Manager.",
        )
        .opens("mobile"),
    );
    mob_tools_status(state).await
}

#[tauri::command]
pub async fn mob_devices(state: State<'_, AppState>) -> CmdResult<Vec<MobileDevice>> {
    let Some(adb) = state.tools.locate(Tool::Adb) else {
        return Ok(Vec::new());
    };
    tokio::task::spawn_blocking(move || {
        let out = run_adb(&adb, &["devices", "-l"], 8)?;
        Ok(parse_devices_output(&out))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_pair_wifi(
    app: AppHandle,
    state: State<'_, AppState>,
    address: String,
    code: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let addr = address.trim().to_string();
    let pin = code.trim().to_string();
    if addr.is_empty() || pin.is_empty() {
        return Err("Enter both the IP:port and the 6-digit pairing code from your phone.".into());
    }
    let out = tokio::task::spawn_blocking(move || run_adb(&adb, &["pair", &addr, &pin], 20))
        .await
        .map_err(|e| e.to_string())??;
    record_timeline(
        &state,
        &app,
        &address,
        "Wi-Fi device",
        "pair",
        "Paired over Wi-Fi",
        &address,
    );
    Ok(out.trim().to_string())
}

#[tauri::command]
pub async fn mob_connect_wifi(
    app: AppHandle,
    state: State<'_, AppState>,
    address: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let addr = address.trim().to_string();
    if addr.is_empty() {
        return Err("Enter the IP:port shown in Wireless debugging.".into());
    }
    let addr_for_adb = addr.clone();
    let adb_for_name = adb.clone();
    let (out, dev_name) = tokio::task::spawn_blocking(move || -> CmdResult<(String, String)> {
        let out = run_adb(&adb, &["connect", &addr_for_adb], 12)?;
        if out.to_lowercase().contains("cannot connect") || out.to_lowercase().contains("failed") {
            return Err(out.trim().to_string());
        }
        let name = device_display_name(&adb_for_name, &addr_for_adb);
        Ok((out, name))
    })
    .await
    .map_err(|e| e.to_string())??;
    save_wifi_device(&state, &addr, &dev_name);
    record_timeline(
        &state,
        &app,
        &addr,
        &dev_name,
        "connect",
        "Connected over Wi-Fi",
        &addr,
    );
    Ok(out.trim().to_string())
}

#[tauri::command]
pub async fn mob_disconnect_wifi(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || run_adb(&adb, &["disconnect", &serial], 8))
        .await
        .map_err(|e| e.to_string())?
        .map(|s| s.trim().to_string())
}

#[tauri::command]
pub async fn mob_enable_wifi(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let s = serial.clone();
    let (target, dev_name) = tokio::task::spawn_blocking(move || -> CmdResult<(String, String)> {
        let name = device_display_name(&adb, &s);
        let ip_raw = adb_shell(
            &adb,
            &s,
            "ip route | awk '/wlan0/ && /src/ {print $9; exit}'",
            5,
        )?;
        let ip = ip_raw.trim().to_string();
        if ip.parse::<std::net::Ipv4Addr>().is_err() {
            return Err("Connect your phone to the same Wi-Fi network first.".into());
        }
        let _ = run_adb(&adb, &["-s", &s, "tcpip", "5555"], 8)?;
        std::thread::sleep(Duration::from_millis(900));
        let endpoint = format!("{ip}:5555");
        let res = run_adb(&adb, &["connect", &endpoint], 10)?;
        if res.to_lowercase().contains("cannot connect") || res.to_lowercase().contains("failed") {
            return Err(res.trim().to_string());
        }
        Ok((endpoint, name))
    })
    .await
    .map_err(|e| e.to_string())??;

    save_wifi_device(&state, &target, &dev_name);
    record_timeline(
        &state,
        &app,
        &target,
        &dev_name,
        "connect",
        "Switched to Wi-Fi",
        &format!("Connected at {target} — you can unplug USB"),
    );
    Ok(target)
}

#[tauri::command]
pub async fn mob_snapshot(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<DeviceSnapshot> {
    let adb = locate_adb(&state)?;
    let muted = muted_set(&state);
    let dismissed = state
        .mobile
        .dismissed_keys
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let audio_forwarding = is_child_alive(&state.mobile.audio_proc, &serial);
    let mic_active = is_child_alive(&state.mobile.mic_proc, &serial);
    let webcam_active = is_child_alive(&state.mobile.webcam_proc, &serial);
    let recording_active = state
        .mobile
        .rec_proc
        .lock()
        .ok()
        .and_then(|mut g| {
            if let Some((s, _, c)) = g.as_mut() {
                if matches!(c.try_wait(), Ok(None)) {
                    return Some(s == &serial);
                }
                *g = None;
            }
            None
        })
        .unwrap_or(false);
    let torch_cached = state
        .mobile
        .torch_state
        .lock()
        .ok()
        .and_then(|m| m.get(&serial).copied())
        .unwrap_or(false);

    tokio::task::spawn_blocking(move || -> CmdResult<DeviceSnapshot> {
        // Gather core properties & settings in a single fast shell round-trip.
        let probe_script = concat!(
            "echo \"===NAME===\"; ",
            "for p in ro.product.marketname ro.config.marketing_name ro.product.vendor.marketname ro.product.model; do ",
            "v=$(getprop $p); if [ -n \"$v\" ]; then echo \"$v\"; break; fi; done; ",
            "echo \"===MFR===\"; getprop ro.product.manufacturer; ",
            "echo \"===REL===\"; getprop ro.build.version.release; ",
            "echo \"===SDK===\"; getprop ro.build.version.sdk; ",
            "echo \"===SIZE===\"; wm size; ",
            "echo \"===IP===\"; ip route | awk '/wlan0/ && /src/ {print $9; exit}'; ",
            "echo \"===SET===\"; ",
            "echo \"zen=$(settings get global zen_mode)\"; ",
            "echo \"wifi=$(settings get global wifi_on)\"; ",
            "echo \"bt=$(settings get global bluetooth_on)\"; ",
            "echo \"bright=$(settings get system screen_brightness)\"; ",
            "echo \"autobright=$(settings get system screen_brightness_mode)\"; ",
            "echo \"timeout=$(settings get system screen_off_timeout)\"; ",
            "echo \"night=$(cmd uimode night 2>/dev/null)\"; ",
            "echo \"===AP===\"; ip addr show 2>/dev/null | grep -E 'ap0|swlan0|wlan1' | head -n 2; ",
            "echo \"===BAT===\"; dumpsys battery; ",
            "echo \"===MEDIA===\"; dumpsys media_session; ",
            "echo \"===AUDIO===\"; dumpsys audio | grep -A 6 -E 'STREAM_MUSIC|STREAM_RING|STREAM_ALARM|STREAM_NOTIFICATION' | head -n 60; ",
            "echo \"===NOTIF===\"; dumpsys notification --noredact"
        );

        let raw = adb_shell(&adb, &serial, probe_script, 10)?;
        let section = |tag: &str, next: Option<&str>| -> &str {
            let marker = format!("==={tag}===");
            let Some(start) = raw.find(&marker) else {
                return "";
            };
            let rest = &raw[start + marker.len()..];
            if let Some(n) = next {
                let end_marker = format!("==={n}===");
                if let Some(end) = rest.find(&end_marker) {
                    return rest[..end].trim();
                }
            }
            rest.trim()
        };

        let name = {
            let s = section("NAME", Some("MFR"));
            if s.is_empty() {
                serial.clone()
            } else {
                s.lines().next().unwrap_or(&serial).trim().to_string()
            }
        };
        let manufacturer = section("MFR", Some("REL"))
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let android_version = section("REL", Some("SDK"))
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let sdk_level = section("SDK", Some("SIZE"))
            .trim()
            .parse::<u32>()
            .unwrap_or(0);
        let screen_size = section("SIZE", Some("IP"))
            .lines()
            .rev()
            .find_map(|l| l.split(':').nth(1))
            .map(|s| s.trim().replace('x', "×"))
            .unwrap_or_default();
        let wifi_ip = {
            let ip = section("IP", Some("SET")).trim();
            if ip.parse::<std::net::Ipv4Addr>().is_ok() {
                Some(ip.to_string())
            } else {
                extract_ip_from_serial(&serial)
            }
        };

        let mut dnd = false;
        let mut wifi = false;
        let mut bluetooth = false;
        let mut dark_mode = false;
        let mut sd = SoundDisplayState::default();
        for line in section("SET", Some("AP")).lines() {
            if let Some(v) = line.strip_prefix("zen=") {
                dnd = v.trim() != "0" && v.trim() != "null" && !v.trim().is_empty();
            } else if let Some(v) = line.strip_prefix("wifi=") {
                wifi = v.trim() == "1" || v.trim() == "2";
            } else if let Some(v) = line.strip_prefix("bt=") {
                bluetooth = v.trim() == "1";
            } else if let Some(v) = line.strip_prefix("night=") {
                dark_mode = v.to_lowercase().contains("yes");
            } else if let Some(v) = line.strip_prefix("bright=") {
                if let Ok(b) = v.trim().parse::<u8>() {
                    sd.brightness = b;
                }
            } else if let Some(v) = line.strip_prefix("autobright=") {
                sd.auto_brightness = v.trim() == "1";
            } else if let Some(v) = line.strip_prefix("timeout=") {
                if let Ok(ms) = v.trim().parse::<u32>() {
                    sd.screen_timeout_ms = ms;
                }
            }
        }
        let ap_text = section("AP", Some("BAT"));
        let hotspot = ap_text.contains("UP");
        let battery = parse_battery(section("BAT", Some("MEDIA")));
        let media = parse_media_session(section("MEDIA", Some("AUDIO")));

        let audio_text = section("AUDIO", Some("NOTIF"));
        if let Some((c, m)) = parse_stream_volume(audio_text, "STREAM_MUSIC") {
            sd.media_vol = c;
            sd.media_max = m;
        }
        if let Some((c, m)) = parse_stream_volume(audio_text, "STREAM_RING") {
            sd.ring_vol = c;
            sd.ring_max = m;
        }
        if let Some((c, m)) = parse_stream_volume(audio_text, "STREAM_ALARM") {
            sd.alarm_vol = c;
            sd.alarm_max = m;
        }
        if let Some((c, m)) = parse_stream_volume(audio_text, "STREAM_NOTIFICATION") {
            sd.notif_vol = c;
            sd.notif_max = m;
        }

        let notifications = parse_notifications(section("NOTIF", None), &muted, &dismissed);
        let is_wifi = is_wifi_serial(&serial);
        let latency_ms = if is_wifi {
            measure_tcp_latency(&serial)
        } else {
            None
        };

        let mut muted_packages: Vec<String> = muted.into_iter().collect();
        muted_packages.sort();

        Ok(DeviceSnapshot {
            serial,
            name,
            manufacturer,
            android_version,
            sdk_level,
            screen_size,
            transport: if is_wifi { "wifi".into() } else { "usb".into() },
            wifi_ip,
            latency_ms,
            battery,
            controls: ControlsState {
                dnd,
                torch: torch_cached,
                wifi,
                bluetooth,
                hotspot,
                audio_forwarding,
                mic_active,
                webcam_active,
                recording_active,
                dark_mode,
            },
            sound_display: sd,
            media,
            notifications,
            muted_packages,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_media_action(
    state: State<'_, AppState>,
    serial: String,
    action: String,
) -> CmdResult<()> {
    let adb = locate_adb(&state)?;
    let keycode = match action.as_str() {
        "play_pause" => "85",
        "next" => "87",
        "prev" => "88",
        "stop" => "86",
        "vol_up" => "24",
        "vol_down" => "25",
        _ => return Err("Unknown media action.".into()),
    };
    tokio::task::spawn_blocking(move || {
        adb_shell(&adb, &serial, &format!("input keyevent {keycode}"), 4)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_toggle_control(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    control: String,
    enabled: bool,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let dev_serial = serial.clone();
    let ctrl = control.clone();
    let note = tokio::task::spawn_blocking(move || -> CmdResult<String> {
        match ctrl.as_str() {
            "dnd" => {
                let mode = if enabled { "priority" } else { "off" };
                let _ = adb_shell(&adb, &dev_serial, &format!("cmd notification set_dnd {mode}"), 4);
                let val = if enabled { "1" } else { "0" };
                let _ = adb_shell(&adb, &dev_serial, &format!("settings put global zen_mode {val}"), 4);
                Ok(if enabled { "Do Not Disturb on".into() } else { "Do Not Disturb off".into() })
            }
            "wifi" => {
                let arg = if enabled { "enable" } else { "disable" };
                adb_shell(&adb, &dev_serial, &format!("svc wifi {arg}"), 5)?;
                Ok(if enabled { "Wi-Fi turned on".into() } else { "Wi-Fi turned off".into() })
            }
            "bluetooth" => {
                let arg = if enabled { "enable" } else { "disable" };
                let _ = adb_shell(&adb, &dev_serial, &format!("cmd bluetooth_manager {arg}"), 5);
                let _ = adb_shell(&adb, &dev_serial, &format!("svc bluetooth {arg}"), 5);
                Ok(if enabled { "Bluetooth turned on".into() } else { "Bluetooth turned off".into() })
            }
            "hotspot" => {
                if enabled {
                    let res = adb_shell(&adb, &dev_serial, "cmd wifi start-softap NomNomSpot open", 5);
                    if res.is_err() {
                        let _ = adb_shell(
                            &adb,
                            &dev_serial,
                            "am start -a android.settings.TETHER_SETTINGS",
                            5,
                        );
                        return Ok("Opened hotspot settings on your phone".into());
                    }
                    Ok("Hotspot turned on".into())
                } else {
                    let _ = adb_shell(&adb, &dev_serial, "cmd wifi stop-softap", 5);
                    Ok("Hotspot turned off".into())
                }
            }
            "torch" => {
                // Expand quick settings or click flashlight tile on supported Android builds.
                let clicked = adb_shell(
                    &adb,
                    &dev_serial,
                    "cmd statusbar click-tile com.android.systemui/.qs.tiles.FlashlightTile",
                    4,
                )
                .is_ok();
                if !clicked {
                    let _ = adb_shell(
                        &adb,
                        &dev_serial,
                        "cmd statusbar click-tile Flashlight",
                        4,
                    );
                }
                Ok(if enabled { "Torch toggled on".into() } else { "Torch toggled off".into() })
            }
            _ => Err("Unknown control.".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())??;

    if control == "torch" {
        if let Ok(mut m) = state.mobile.torch_state.lock() {
            m.insert(serial.clone(), enabled);
        }
    }
    record_timeline(&state, &app, &serial, &serial, "control", &note, &control);
    Ok(note)
}

#[tauri::command]
pub async fn mob_set_sound_display(
    state: State<'_, AppState>,
    serial: String,
    patch: SoundDisplayPatch,
) -> CmdResult<()> {
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        let mut cmds = Vec::new();
        if let Some(v) = patch.media_vol {
            cmds.push(format!("cmd media_session volume --stream 3 --set {v}"));
        }
        if let Some(v) = patch.ring_vol {
            cmds.push(format!("cmd media_session volume --stream 2 --set {v}"));
        }
        if let Some(v) = patch.alarm_vol {
            cmds.push(format!("cmd media_session volume --stream 4 --set {v}"));
        }
        if let Some(v) = patch.notif_vol {
            cmds.push(format!("cmd media_session volume --stream 5 --set {v}"));
        }
        if let Some(auto) = patch.auto_brightness {
            cmds.push(format!(
                "settings put system screen_brightness_mode {}",
                if auto { 1 } else { 0 }
            ));
        }
        if let Some(b) = patch.brightness {
            cmds.push("settings put system screen_brightness_mode 0".into());
            cmds.push(format!("settings put system screen_brightness {b}"));
        }
        if let Some(ms) = patch.screen_timeout_ms {
            cmds.push(format!("settings put system screen_off_timeout {ms}"));
        }
        if !cmds.is_empty() {
            let joined = cmds.join("; ");
            adb_shell(&adb, &serial, &joined, 6)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_mirror(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    options: Option<MirrorOptions>,
) -> CmdResult<()> {
    let scrcpy = locate_scrcpy(&state)?;
    let adb = locate_adb(&state)?;
    let opts = options.unwrap_or_default();
    let app_pkg = opts.start_app.clone();

    let s = serial.clone();
    let adb_for_name = adb.clone();
    let (dev_name, size_label) = tokio::task::spawn_blocking(move || {
        let name = device_display_name(&adb_for_name, &s);
        let sz = adb_shell(&adb_for_name, &s, "wm size", 3)
            .ok()
            .and_then(|t| t.lines().rev().find_map(|l| l.split(':').nth(1)).map(|v| v.trim().replace('x', "×")))
            .unwrap_or_default();
        (name, sz)
    })
    .await
    .unwrap_or_else(|_| (serial.clone(), String::new()));

    let mut cmd = hidden_cmd(&scrcpy);
    if let Some(parent) = scrcpy.parent() {
        cmd.current_dir(parent);
    }
    cmd.env("ADB", &adb);
    cmd.arg("-s").arg(&serial);
    let title = if let Some(ref pkg) = app_pkg {
        format!("{} · NomNom", friendly_app_name(pkg))
    } else {
        format!("{dev_name} · NomNom")
    };
    cmd.arg("--window-title").arg(title);

    if let Some(max_size) = opts.max_size.filter(|&v| v > 0) {
        cmd.arg("--max-size").arg(max_size.to_string());
    }
    if let Some(mbps) = opts.bit_rate_mbps.filter(|&v| v > 0) {
        cmd.arg("--video-bit-rate").arg(format!("{mbps}M"));
    }
    if let Some(fps) = opts.max_fps.filter(|&v| v > 0) {
        cmd.arg("--max-fps").arg(fps.to_string());
    }
    if let Some(codec) = opts.video_codec.as_deref() {
        if matches!(codec, "h264" | "h265" | "av1") {
            cmd.arg(format!("--video-codec={codec}"));
        }
    }
    if opts.turn_screen_off.unwrap_or(false) {
        cmd.arg("--turn-screen-off");
    }
    if opts.stay_awake.unwrap_or(true) {
        cmd.arg("--stay-awake");
    }
    if opts.audio == Some(false) {
        cmd.arg("--no-audio");
    }
    if opts.keyboard_uhid.unwrap_or(false) {
        cmd.arg("--keyboard=uhid");
    }
    if opts.mouse_uhid.unwrap_or(false) {
        cmd.arg("--mouse=uhid");
    }
    if opts.gamepad_uhid.unwrap_or(false) {
        cmd.arg("--gamepad=uhid");
    }
    if opts.borderless.unwrap_or(false) {
        cmd.arg("--window-borderless");
    }
    if opts.always_on_top.unwrap_or(false) {
        cmd.arg("--always-on-top");
    }
    if let Some(buf) = opts.video_buffer_ms.filter(|&b| b > 0 && b <= 500) {
        cmd.arg(format!("--video-buffer={buf}"));
    }
    if let Some(buf) = opts.audio_buffer_ms.filter(|&b| b > 0 && b <= 500) {
        cmd.arg(format!("--audio-buffer={buf}"));
    }
    if let Some(did) = opts.display_id {
        cmd.arg(format!("--display-id={did}"));
    }
    if opts.new_display.unwrap_or(false) || app_pkg.is_some() {
        if opts.new_display.unwrap_or(true) {
            let dim = opts
                .display_size
                .as_deref()
                .filter(|d| d.contains('x') && d.chars().all(|c| c.is_ascii_digit() || c == 'x'))
                .unwrap_or("880x1920");
            if let Some(dpi) = opts.display_dpi.filter(|&d| (120..=640).contains(&d)) {
                cmd.arg(format!("--new-display={dim}/{dpi}"));
            } else {
                cmd.arg(format!("--new-display={dim}"));
            }
            cmd.arg("--display-ime-policy=local");
            if opts.keep_alive_on_close.unwrap_or(false) {
                cmd.arg("--no-vd-destroy-content");
            }
            if opts.no_decorations.unwrap_or(false) {
                cmd.arg("--no-vd-system-decorations");
            }
        }
        if let Some(ref pkg) = app_pkg {
            cmd.arg(format!("--start-app={pkg}"));
        }
    }

    let mut child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Couldn't start screen mirroring: {e}"))?;

    let started = Instant::now();
    let app_watch = app.clone();
    let serial_watch = serial.clone();
    let name_watch = dev_name.clone();
    let title_kind = if let Some(ref pkg) = app_pkg {
        format!("App window · {}", friendly_app_name(pkg))
    } else {
        "Screen mirroring".into()
    };
    std::thread::spawn(move || {
        let _ = child.wait();
        let secs = started.elapsed().as_secs().max(1);
        let detail = if size_label.is_empty() {
            format!("With {name_watch} · {}", format_duration_short(secs))
        } else {
            format!("With {name_watch} · {size_label} · {}", format_duration_short(secs))
        };
        let st = app_watch.state::<AppState>();
        record_timeline(
            &st,
            &app_watch,
            &serial_watch,
            &name_watch,
            "mirror",
            &title_kind,
            &detail,
        );
    });

    Ok(())
}

#[tauri::command]
pub async fn mob_webcam(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    facing: Option<String>,
) -> CmdResult<bool> {
    let scrcpy = locate_scrcpy(&state)?;
    let adb = locate_adb(&state)?;
    if is_child_alive(&state.mobile.webcam_proc, &serial) {
        if let Ok(mut g) = state.mobile.webcam_proc.lock() {
            if let Some((_, mut c)) = g.take() {
                let _ = c.kill();
            }
        }
        return Ok(false);
    }

    let face = match facing.as_deref() {
        Some("front") => "front",
        _ => "back",
    };
    let mut cmd = hidden_cmd(&scrcpy);
    if let Some(parent) = scrcpy.parent() {
        cmd.current_dir(parent);
    }
    cmd.env("ADB", &adb)
        .arg("-s")
        .arg(&serial)
        .arg("--video-source=camera")
        .arg(format!("--camera-facing={face}"))
        .arg("--camera-size=1280x720")
        .arg("--no-audio")
        .arg("--window-title")
        .arg("NomNom Phone Webcam")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Couldn't start phone camera stream: {e}"))?;
    *state.mobile.webcam_proc.lock().unwrap() = Some((serial.clone(), child));
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "webcam",
        "Phone as webcam",
        &format!("Started {face} camera stream (Android 12+)"),
    );
    Ok(true)
}

#[tauri::command]
pub async fn mob_audio_toggle(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    enabled: bool,
) -> CmdResult<bool> {
    if !enabled {
        if let Ok(mut g) = state.mobile.audio_proc.lock() {
            if let Some((_, mut c)) = g.take() {
                let _ = c.kill();
            }
        }
        return Ok(false);
    }
    if is_child_alive(&state.mobile.audio_proc, &serial) {
        return Ok(true);
    }
    let scrcpy = locate_scrcpy(&state)?;
    let adb = locate_adb(&state)?;
    let mut cmd = hidden_cmd(&scrcpy);
    if let Some(parent) = scrcpy.parent() {
        cmd.current_dir(parent);
    }
    cmd.env("ADB", &adb)
        .arg("-s")
        .arg(&serial)
        .arg("--no-video")
        .arg("--no-control")
        .arg("--audio-source=output")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Couldn't start audio bridge (requires Android 11+): {e}"))?;
    *state.mobile.audio_proc.lock().unwrap() = Some((serial.clone(), child));
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "audio",
        "Audio bridge started",
        "Streaming phone audio to this computer",
    );
    Ok(true)
}

#[tauri::command]
pub async fn mob_continuity_camera(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<CapturedImage> {
    let adb = locate_adb(&state)?;
    let dest_dir = phone_save_dir(&state);
    let s = serial.clone();
    let img = tokio::task::spawn_blocking(move || -> CmdResult<CapturedImage> {
        let before = adb_shell(
            &adb,
            &s,
            "ls -1t /sdcard/DCIM/Camera/*.jpg /sdcard/DCIM/Camera/*.jpeg 2>/dev/null | head -n 1",
            4,
        )
        .unwrap_or_default()
        .trim()
        .to_string();

        // Launch camera on phone so the user can snap a photo.
        let _ = adb_shell(
            &adb,
            &s,
            "input keyevent 224; am start -a android.media.action.STILL_IMAGE_CAMERA",
            5,
        );

        // Wait up to 25 seconds for a new photo in DCIM/Camera.
        let deadline = Instant::now() + Duration::from_secs(25);
        let mut newest = String::new();
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(900));
            let cur = adb_shell(
                &adb,
                &s,
                "ls -1t /sdcard/DCIM/Camera/*.jpg /sdcard/DCIM/Camera/*.jpeg 2>/dev/null | head -n 1",
                4,
            )
            .unwrap_or_default()
            .trim()
            .to_string();
            if !cur.is_empty() && cur != before {
                newest = cur;
                break;
            }
        }
        if newest.is_empty() {
            return Err(
                "No new photo was taken within 25 seconds. Take a photo on your phone after pressing Continuity Camera."
                    .into(),
            );
        }

        let _ = std::fs::create_dir_all(&dest_dir);
        let name = newest
            .rsplit('/')
            .next()
            .unwrap_or("photo.jpg")
            .to_string();
        let local = dest_dir.join(&name);
        let local_str = local.to_string_lossy().into_owned();
        run_adb(&adb, &["-s", &s, "pull", &newest, &local_str], 20)?;
        let bytes = std::fs::read(&local).map_err(|e| format!("Couldn't read saved photo: {e}"))?;
        let data_url = format!("data:image/jpeg;base64,{}", encode_base64(&bytes));
        Ok(CapturedImage {
            path: local_str,
            name,
            data_url,
        })
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "camera",
        "Continuity Camera",
        &format!("Saved {}", img.name),
    );
    toast::show(
        &app,
        Toast::new(
            "success",
            "Photo received",
            format!("Saved {} to your computer.", img.name),
        )
        .opens("mobile"),
    );
    Ok(img)
}

#[tauri::command]
pub async fn mob_screenshot(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<CapturedImage> {
    let adb = locate_adb(&state)?;
    let dest_dir = phone_save_dir(&state);
    let s = serial.clone();
    let img = tokio::task::spawn_blocking(move || -> CmdResult<CapturedImage> {
        let bytes = run_adb_raw(&adb, &["-s", &s, "exec-out", "screencap", "-p"], 12)?;
        if bytes.len() < 16 {
            return Err("Couldn't capture the phone screen.".into());
        }
        let _ = std::fs::create_dir_all(&dest_dir);
        let name = format!("phone-screen-{}.png", now());
        let local = dest_dir.join(&name);
        std::fs::write(&local, &bytes).map_err(|e| format!("Couldn't save screenshot: {e}"))?;
        let data_url = format!("data:image/png;base64,{}", encode_base64(&bytes));
        Ok(CapturedImage {
            path: local.to_string_lossy().into_owned(),
            name,
            data_url,
        })
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "camera",
        "Phone screenshot",
        &format!("Saved {}", img.name),
    );
    Ok(img)
}

#[tauri::command]
pub async fn mob_send_files(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    paths: Vec<String>,
) -> CmdResult<usize> {
    if paths.is_empty() {
        return Ok(0);
    }
    let adb = locate_adb(&state)?;
    let s = serial.clone();
    let files = paths.clone();
    let (sent, dev_name) = tokio::task::spawn_blocking(move || -> CmdResult<(usize, String)> {
        let dev_name = device_display_name(&adb, &s);
        let _ = adb_shell(&adb, &s, "mkdir -p /sdcard/Download/NomNom", 5);
        let mut count = 0usize;
        for p in &files {
            let path = Path::new(p);
            if !path.is_file() {
                continue;
            }
            let fname = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "file".into());
            let remote = format!("/sdcard/Download/NomNom/{fname}");
            run_adb(&adb, &["-s", &s, "push", p, &remote], 120)?;
            let _ = adb_shell(
                &adb,
                &s,
                &format!(
                    "am broadcast -a android.intent.action.MEDIA_SCANNER_SCAN_FILE -d {}",
                    shell_single_quote(&format!("file://{remote}"))
                ),
                4,
            );
            count += 1;
        }
        Ok((count, dev_name))
    })
    .await
    .map_err(|e| e.to_string())??;

    if sent > 0 {
        let title = if sent == 1 {
            Path::new(&paths[0])
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "1 file".into())
        } else {
            format!("{sent} files sent")
        };
        record_timeline(
            &state,
            &app,
            &serial,
            &dev_name,
            "file",
            &title,
            &format!("Sent to {dev_name} · /Download/NomNom"),
        );
    }
    Ok(sent)
}

#[tauri::command]
pub async fn mob_send_clipboard(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    text: Option<String>,
    open_links: Option<bool>,
) -> CmdResult<String> {
    let content = match text {
        Some(t) if !t.trim().is_empty() => t,
        _ => app
            .clipboard()
            .read_text()
            .map_err(|_| "Your computer's clipboard is empty.".to_string())?,
    };
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() {
        return Err("Your computer's clipboard is empty.".into());
    }
    let adb = locate_adb(&state)?;
    let s = serial.clone();
    let payload = trimmed.clone();
    let should_open = open_links.unwrap_or(true)
        && (trimmed.starts_with("http://") || trimmed.starts_with("https://"));

    let dev_name = tokio::task::spawn_blocking(move || -> CmdResult<String> {
        let name = device_display_name(&adb, &s);
        if should_open {
            adb_shell(
                &adb,
                &s,
                &format!(
                    "am start -a android.intent.action.VIEW -d {}",
                    shell_single_quote(&payload)
                ),
                6,
            )?;
        } else {
            // Try Android 12+ `cmd clipboard`, then fallback to typing into the focused box.
            let quoted = shell_single_quote(&payload);
            let copied = adb_shell(&adb, &s, &format!("cmd clipboard set {quoted}"), 5).is_ok();
            if !copied {
                let escaped = escape_adb_input_text(&payload);
                adb_shell(&adb, &s, &format!("input text {escaped}"), 6)?;
            }
        }
        Ok(name)
    })
    .await
    .map_err(|e| e.to_string())??;

    if should_open {
        let host = trimmed
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or(&trimmed);
        record_timeline(
            &state,
            &app,
            &serial,
            &dev_name,
            "clipboard",
            &format!("Link · {host}"),
            &format!("Sent to {dev_name} · {trimmed}"),
        );
        Ok(format!("Opened {host} on {dev_name}"))
    } else {
        let preview: String = trimmed.chars().take(48).collect();
        record_timeline(
            &state,
            &app,
            &serial,
            &dev_name,
            "clipboard",
            "Clipboard sent",
            &format!("Sent to {dev_name} · {preview}"),
        );
        Ok(format!("Sent clipboard to {dev_name}"))
    }
}

#[tauri::command]
pub async fn mob_find_phone(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let s = serial.clone();
    let dev_name = tokio::task::spawn_blocking(move || -> CmdResult<String> {
        let name = device_display_name(&adb, &s);
        let script = concat!(
            "input keyevent 224; ",
            "cmd notification set_dnd off 2>/dev/null; ",
            "cmd media_session volume --stream 3 --set 15 2>/dev/null; ",
            "cmd media_session volume --stream 4 --set 7 2>/dev/null; ",
            "cmd vibrator_manager synced -f 0 600 200 600 200 600 2>/dev/null || cmd vibrator vibrate 1200 2>/dev/null; ",
            "am start -a android.intent.action.SET_TIMER --ei android.intent.extra.alarm.LENGTH 1 --es android.intent.extra.alarm.MESSAGE 'NomNom Find Phone' --ez android.intent.extra.alarm.SKIP_UI true 2>/dev/null || true"
        );
        adb_shell(&adb, &s, script, 8)?;
        Ok(name)
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(
        &state,
        &app,
        &serial,
        &dev_name,
        "find",
        "Find phone",
        &format!("Rang and vibrated {dev_name}"),
    );
    Ok(format!("Ringing {dev_name}…"))
}

#[tauri::command]
pub async fn mob_type_text(
    state: State<'_, AppState>,
    serial: String,
    text: String,
) -> CmdResult<()> {
    if text.is_empty() {
        return Ok(());
    }
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        let escaped = escape_adb_input_text(&text);
        if !escaped.is_empty() {
            adb_shell(&adb, &serial, &format!("input text {escaped}"), 5)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_keyevent(
    state: State<'_, AppState>,
    serial: String,
    key: String,
) -> CmdResult<()> {
    let adb = locate_adb(&state)?;
    let code = match key.as_str() {
        "enter" => "66",
        "backspace" => "67",
        "tab" => "61",
        "escape" => "111",
        "home" => "3",
        "back" => "4",
        "recents" => "187",
        "power" => "26",
        "wakeup" => "224",
        "sleep" => "223",
        "vol_up" => "24",
        "vol_down" => "25",
        "mute" => "164",
        "play_pause" => "85",
        "next" => "87",
        "prev" => "88",
        "answer" => "5",
        "end_call" => "6",
        other if other.chars().all(|c| c.is_ascii_digit()) && !other.is_empty() => other,
        _ => return Err("Unknown key.".into()),
    }
    .to_string();
    tokio::task::spawn_blocking(move || {
        adb_shell(&adb, &serial, &format!("input keyevent {code}"), 4)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_notification_action(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    package: String,
    key: String,
    action: String,
    reply_text: Option<String>,
) -> CmdResult<String> {
    match action.as_str() {
        "mute" => {
            let _ = state.db.with(|c| {
                c.execute(
                    "INSERT OR REPLACE INTO mobile_muted(package_name, muted_at) VALUES (?1, ?2)",
                    rusqlite::params![package, now()],
                )
            });
            return Ok(format!("Muted notifications from {}", friendly_app_name(&package)));
        }
        "unmute" => {
            let _ = state.db.with(|c| {
                c.execute(
                    "DELETE FROM mobile_muted WHERE package_name = ?1",
                    rusqlite::params![package],
                )
            });
            return Ok(format!("Unmuted {}", friendly_app_name(&package)));
        }
        "dismiss" => {
            if let Ok(mut g) = state.mobile.dismissed_keys.lock() {
                g.insert(key.clone());
            }
            if let Ok(adb) = locate_adb(&state) {
                let s = serial.clone();
                let k = key.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    let _ = adb_shell(
                        &adb,
                        &s,
                        &format!("cmd notification snooze --for 3600000 {}", shell_single_quote(&k)),
                        4,
                    );
                })
                .await;
            }
            return Ok("Dismissed".into());
        }
        "clear_all" => {
            if let Ok(mut g) = state.mobile.dismissed_keys.lock() {
                // Also try clearing on phone if allowed.
                g.clear();
            }
            if let Ok(adb) = locate_adb(&state) {
                let s = serial.clone();
                let current_keys = tokio::task::spawn_blocking(move || {
                    let raw = adb_shell(&adb, &s, "dumpsys notification --noredact", 6)
                        .unwrap_or_default();
                    let _ = adb_shell(&adb, &s, "service call notification 1", 4);
                    parse_notifications(&raw, &HashSet::new(), &HashSet::new())
                        .into_iter()
                        .map(|n| n.key)
                        .collect::<Vec<_>>()
                })
                .await
                .unwrap_or_default();
                if let Ok(mut g) = state.mobile.dismissed_keys.lock() {
                    for k in current_keys {
                        g.insert(k);
                    }
                }
            }
            return Ok("Cleared notifications".into());
        }
        "open" => {
            let adb = locate_adb(&state)?;
            let pkg = package.clone();
            let s = serial.clone();
            tokio::task::spawn_blocking(move || {
                adb_shell(
                    &adb,
                    &s,
                    &format!("monkey -p {} -c android.intent.category.LAUNCHER 1", shell_single_quote(&pkg)),
                    6,
                )
            })
            .await
            .map_err(|e| e.to_string())??;
            return Ok(format!("Opened {}", friendly_app_name(&package)));
        }
        "reply" => {
            let text = reply_text.unwrap_or_default();
            let adb = locate_adb(&state)?;
            let pkg = package.clone();
            let s = serial.clone();
            tokio::task::spawn_blocking(move || -> CmdResult<()> {
                let _ = adb_shell(
                    &adb,
                    &s,
                    &format!("monkey -p {} -c android.intent.category.LAUNCHER 1", shell_single_quote(&pkg)),
                    6,
                );
                if !text.trim().is_empty() {
                    std::thread::sleep(Duration::from_millis(650));
                    let escaped = escape_adb_input_text(text.trim());
                    let _ = adb_shell(&adb, &s, &format!("input text {escaped}"), 5);
                    let _ = adb_shell(&adb, &s, "input keyevent 66", 4);
                }
                Ok(())
            })
            .await
            .map_err(|e| e.to_string())??;
            record_timeline(
                &state,
                &app,
                &serial,
                &serial,
                "message",
                &format!("Replied in {}", friendly_app_name(&package)),
                "Opened conversation on phone",
            );
            return Ok(format!("Sent reply in {}", friendly_app_name(&package)));
        }
        _ => Err("Unknown notification action.".into()),
    }
}

#[tauri::command]
pub async fn mob_messages(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<Vec<SmsMessage>> {
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<Vec<SmsMessage>> {
        let out = adb_shell(
            &adb,
            &serial,
            "content query --uri content://sms --projection _id:address:body:date:type:read --sort 'date DESC'",
            10,
        )?;
        if out.contains("Error while accessing provider") || out.contains("Permission Denial") {
            return Err(
                "This phone restricts shell SMS reading. You can still compose messages or mirror Messages directly."
                    .into(),
            );
        }
        let mut list = Vec::new();
        for row in parse_content_rows(&out).into_iter().take(120) {
            let id = row.get("_id").cloned().unwrap_or_default();
            let address = row.get("address").cloned().unwrap_or_default();
            let body = row.get("body").cloned().unwrap_or_default();
            let date_ms = row
                .get("date")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let msg_type = row.get("type").map(String::as_str).unwrap_or("1");
            let read = row.get("read").map(String::as_str).unwrap_or("1") == "1";
            if !address.is_empty() {
                list.push(SmsMessage {
                    id,
                    address,
                    body,
                    date_ms,
                    incoming: msg_type == "1",
                    read,
                });
            }
        }
        Ok(list)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_send_sms(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    address: String,
    body: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let addr = address.trim().to_string();
    let msg = body.trim().to_string();
    if addr.is_empty() || msg.is_empty() {
        return Err("Enter both a phone number and a message.".into());
    }
    let s = serial.clone();
    let a = addr.clone();
    let m = msg.clone();
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        adb_shell(
            &adb,
            &s,
            &format!(
                "am start -a android.intent.action.SENDTO -d {} --es sms_body {} --ez exit_on_sent true",
                shell_single_quote(&format!("sms:{a}")),
                shell_single_quote(&m)
            ),
            6,
        )?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "message",
        &format!("SMS to {addr}"),
        &msg.chars().take(60).collect::<String>(),
    );
    Ok(format!("Prepared message to {addr} on your phone"))
}

#[tauri::command]
pub async fn mob_calls(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<Vec<CallLogEntry>> {
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<Vec<CallLogEntry>> {
        let out = adb_shell(
            &adb,
            &serial,
            "content query --uri content://call_log/calls --projection _id:number:name:date:duration:type --sort 'date DESC'",
            10,
        )?;
        if out.contains("Error while accessing provider") || out.contains("Permission Denial") {
            return Err(
                "This phone restricts shell call-log reading. You can still place and control calls from the dialer below."
                    .into(),
            );
        }
        let mut list = Vec::new();
        for row in parse_content_rows(&out).into_iter().take(80) {
            let id = row.get("_id").cloned().unwrap_or_default();
            let number = row.get("number").cloned().unwrap_or_default();
            let raw_name = row.get("name").cloned().unwrap_or_default();
            let name = if raw_name.is_empty() || raw_name == "NULL" {
                number.clone()
            } else {
                raw_name
            };
            let date_ms = row
                .get("date")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let duration_secs = row
                .get("duration")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let kind = match row.get("type").map(String::as_str) {
                Some("1") => "incoming",
                Some("2") => "outgoing",
                Some("3") => "missed",
                Some("5") => "rejected",
                _ => "other",
            }
            .to_string();
            if !number.is_empty() {
                list.push(CallLogEntry {
                    id,
                    number,
                    name,
                    date_ms,
                    duration_secs,
                    kind,
                });
            }
        }
        Ok(list)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_dial(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    number: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let num: String = number
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '+' | '*' | '#' | '-'))
        .collect();
    if num.is_empty() {
        return Err("Enter a phone number to call.".into());
    }
    let s = serial.clone();
    let n = num.clone();
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        let called = adb_shell(
            &adb,
            &s,
            &format!("am start -a android.intent.action.CALL -d tel:{n}"),
            6,
        )
        .is_ok();
        if !called {
            adb_shell(
                &adb,
                &s,
                &format!("am start -a android.intent.action.DIAL -d tel:{n}"),
                6,
            )?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "call",
        &format!("Call {num}"),
        "Dialed from NomNom",
    );
    Ok(format!("Calling {num}…"))
}

#[tauri::command]
pub async fn mob_photos(
    state: State<'_, AppState>,
    serial: String,
    limit: Option<usize>,
) -> CmdResult<Vec<PhonePhoto>> {
    let adb = locate_adb(&state)?;
    let max_items = limit.unwrap_or(60).min(200);
    tokio::task::spawn_blocking(move || -> CmdResult<Vec<PhonePhoto>> {
        let script = format!(
            "for d in /sdcard/DCIM/Camera /sdcard/Pictures/Screenshots /sdcard/Pictures /sdcard/Download; do \
             [ -d \"$d\" ] && ls -1t \"$d\" 2>/dev/null | head -n 60 | while IFS= read -r f; do \
               case \"$f\" in *.jpg|*.jpeg|*.png|*.webp|*.JPG|*.JPEG|*.PNG) \
                 p=\"$d/$f\"; sz=$(stat -c '%s %Y' \"$p\" 2>/dev/null || echo '0 0'); echo \"$sz|$p\";; \
               esac; \
             done; done | head -n {max_items}"
        );
        let out = adb_shell(&adb, &serial, &script, 12)?;
        let mut photos = Vec::new();
        for line in out.lines() {
            let Some((meta, path)) = line.split_once('|') else {
                continue;
            };
            let mut m = meta.split_whitespace();
            let size = m.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            let mtime = m.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            let path = path.trim().to_string();
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            let folder = path
                .rsplit('/')
                .nth(1)
                .unwrap_or("Phone")
                .to_string();
            photos.push(PhonePhoto {
                path,
                name,
                folder,
                size,
                mtime,
            });
        }
        photos.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        photos.truncate(max_items);
        Ok(photos)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_photo_preview(
    state: State<'_, AppState>,
    serial: String,
    remote_path: String,
) -> CmdResult<String> {
    if !remote_path.starts_with("/sdcard/") {
        return Err("Only photos inside /sdcard/ can be read.".into());
    }
    let adb = locate_adb(&state)?;
    let cache_dir = state.paths.data_dir.join("cache").join("mobile-photos");
    let hash = blake3::hash(format!("{serial}:{remote_path}").as_bytes()).to_hex();
    let ext = remote_path
        .rsplit('.')
        .next()
        .unwrap_or("jpg")
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    let cached = cache_dir.join(format!("{hash}.{ext}"));

    tokio::task::spawn_blocking(move || -> CmdResult<String> {
        let _ = std::fs::create_dir_all(&cache_dir);
        if !cached.is_file() {
            let dst = cached.to_string_lossy().into_owned();
            run_adb(&adb, &["-s", &serial, "pull", &remote_path, &dst], 20)?;
        }
        let bytes = std::fs::read(&cached).map_err(|e| format!("Couldn't read preview: {e}"))?;
        if bytes.len() > 14 * 1024 * 1024 {
            return Err("Image is too large for inline preview — download it to open.".into());
        }
        Ok(format!("data:{mime};base64,{}", encode_base64(&bytes)))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_pull_photo(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    remote_path: String,
) -> CmdResult<String> {
    if !remote_path.starts_with("/sdcard/") {
        return Err("Only files inside /sdcard/ can be downloaded.".into());
    }
    let adb = locate_adb(&state)?;
    let dest_dir = phone_save_dir(&state);
    let s = serial.clone();
    let rp = remote_path.clone();
    let saved = tokio::task::spawn_blocking(move || -> CmdResult<String> {
        let _ = std::fs::create_dir_all(&dest_dir);
        let name = rp.rsplit('/').next().unwrap_or("photo.jpg");
        let target = nomnom_core::phone::free_path(&dest_dir, name);
        let target_str = target.to_string_lossy().into_owned();
        run_adb(&adb, &["-s", &s, "pull", &rp, &target_str], 45)?;
        Ok(target_str)
    })
    .await
    .map_err(|e| e.to_string())??;

    let fname = Path::new(&saved)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "photo".into());
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "photo",
        &fname,
        &format!("Saved to {}", phone_save_dir(&state).display()),
    );
    Ok(saved)
}

#[tauri::command]
pub async fn mob_delete_photo(
    state: State<'_, AppState>,
    serial: String,
    remote_path: String,
) -> CmdResult<()> {
    if !remote_path.starts_with("/sdcard/") {
        return Err("Only files inside /sdcard/ can be deleted.".into());
    }
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        adb_shell(
            &adb,
            &serial,
            &format!("rm -f {}", shell_single_quote(&remote_path)),
            6,
        )?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_apps(
    state: State<'_, AppState>,
    serial: String,
    filter: Option<String>,
) -> CmdResult<Vec<PhoneApp>> {
    let adb = locate_adb(&state)?;
    let mode = filter.unwrap_or_else(|| "user".into());
    tokio::task::spawn_blocking(move || -> CmdResult<Vec<PhoneApp>> {
        let script = concat!(
            "echo '===USER==='; pm list packages -3; ",
            "echo '===DISABLED==='; pm list packages -d; ",
            "echo '===ALL==='; pm list packages"
        );
        let raw = adb_shell(&adb, &serial, script, 12)?;
        let mut user_set = HashSet::new();
        let mut disabled_set = HashSet::new();
        let mut all_pkgs = Vec::new();
        let mut cur = "";
        for line in raw.lines() {
            let l = line.trim();
            if l == "===USER===" {
                cur = "user";
                continue;
            } else if l == "===DISABLED===" {
                cur = "disabled";
                continue;
            } else if l == "===ALL===" {
                cur = "all";
                continue;
            }
            if let Some(pkg) = l.strip_prefix("package:") {
                if pkg.is_empty() {
                    continue;
                }
                match cur {
                    "user" => {
                        user_set.insert(pkg.to_string());
                    }
                    "disabled" => {
                        disabled_set.insert(pkg.to_string());
                    }
                    "all" => all_pkgs.push(pkg.to_string()),
                    _ => {}
                }
            }
        }
        let mut apps: Vec<PhoneApp> = all_pkgs
            .into_iter()
            .filter_map(|pkg| {
                let is_user = user_set.contains(&pkg);
                let is_disabled = disabled_set.contains(&pkg);
                let keep = match mode.as_str() {
                    "system" => !is_user,
                    "disabled" => is_disabled,
                    "all" => true,
                    _ => is_user,
                };
                keep.then(|| PhoneApp {
                    name: friendly_app_name(&pkg),
                    package: pkg,
                    system: !is_user,
                    disabled: is_disabled,
                })
            })
            .collect();
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(apps)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_launch_app(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    package: String,
    mirrored: bool,
) -> CmdResult<()> {
    if mirrored {
        return mob_mirror(
            app,
            state,
            serial,
            Some(MirrorOptions {
                start_app: Some(package),
                new_display: Some(true),
                stay_awake: Some(true),
                audio: Some(true),
                ..Default::default()
            }),
        )
        .await;
    }
    let adb = locate_adb(&state)?;
    let pkg = package.clone();
    let s = serial.clone();
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        adb_shell(
            &adb,
            &s,
            &format!(
                "monkey -p {} -c android.intent.category.LAUNCHER 1",
                shell_single_quote(&pkg)
            ),
            6,
        )?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "app",
        &format!("Opened {}", friendly_app_name(&package)),
        &package,
    );
    Ok(())
}

#[tauri::command]
pub async fn mob_install_apk(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    path: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let p = path.clone();
    let s = serial.clone();
    let out = tokio::task::spawn_blocking(move || run_adb(&adb, &["-s", &s, "install", "-r", &p], 90))
        .await
        .map_err(|e| e.to_string())??;
    let fname = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(path);
    record_timeline(&state, &app, &serial, &serial, "app", "Installed APK", &fname);
    Ok(out.trim().to_string())
}

#[tauri::command]
pub async fn mob_uninstall_app(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    package: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let pkg = package.clone();
    let s = serial.clone();
    let out = tokio::task::spawn_blocking(move || run_adb(&adb, &["-s", &s, "uninstall", &pkg], 30))
        .await
        .map_err(|e| e.to_string())??;
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "app",
        &format!("Uninstalled {}", friendly_app_name(&package)),
        &package,
    );
    Ok(out.trim().to_string())
}

#[tauri::command]
pub fn mob_timeline(state: State<'_, AppState>, limit: Option<usize>) -> Vec<TimelineEntry> {
    let lim = limit.unwrap_or(40).min(120) as i64;
    state
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT id, device_id, device_name, kind, title, detail, at
                 FROM mobile_timeline ORDER BY at DESC LIMIT ?1",
            )?;
            let rows = st.query_map([lim], |r| {
                Ok(TimelineEntry {
                    id: r.get(0)?,
                    device_id: r.get(1)?,
                    device_name: r.get(2)?,
                    kind: r.get(3)?,
                    title: r.get(4)?,
                    detail: r.get(5)?,
                    at: r.get(6)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn mob_clear_timeline(app: AppHandle, state: State<'_, AppState>) {
    let _ = state.db.with(|c| c.execute("DELETE FROM mobile_timeline", []));
    let _ = app.emit("mobile://timeline", ());
}

#[tauri::command]
pub fn mob_deck_list(state: State<'_, AppState>) -> Vec<DeckItem> {
    let saved = state
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT id, title, icon, kind, payload FROM mobile_deck ORDER BY created_at ASC",
            )?;
            let rows = st.query_map([], |r| {
                Ok(DeckItem {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    icon: r.get(2)?,
                    kind: r.get(3)?,
                    payload: r.get(4)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap_or_default();

    if !saved.is_empty() {
        return saved;
    }

    // Helpful starter Deck tiles on first visit.
    vec![
        DeckItem {
            id: "preset-screenshot".into(),
            title: "Phone screenshot".into(),
            icon: "image".into(),
            kind: "screenshot".into(),
            payload: String::new(),
        },
        DeckItem {
            id: "preset-wake".into(),
            title: "Wake screen".into(),
            icon: "sun".into(),
            kind: "key".into(),
            payload: "wakeup".into(),
        },
        DeckItem {
            id: "preset-lock".into(),
            title: "Lock screen".into(),
            icon: "lock".into(),
            kind: "key".into(),
            payload: "sleep".into(),
        },
        DeckItem {
            id: "preset-play".into(),
            title: "Play / Pause".into(),
            icon: "play".into(),
            kind: "key".into(),
            payload: "play_pause".into(),
        },
        DeckItem {
            id: "preset-next".into(),
            title: "Next track".into(),
            icon: "arrow".into(),
            kind: "key".into(),
            payload: "next".into(),
        },
        DeckItem {
            id: "preset-camera".into(),
            title: "Open camera".into(),
            icon: "camera".into(),
            kind: "shell".into(),
            payload: "am start -a android.media.action.STILL_IMAGE_CAMERA".into(),
        },
        DeckItem {
            id: "preset-home".into(),
            title: "Home button".into(),
            icon: "home".into(),
            kind: "key".into(),
            payload: "home".into(),
        },
        DeckItem {
            id: "preset-back".into(),
            title: "Back button".into(),
            icon: "arrow-left".into(),
            kind: "key".into(),
            payload: "back".into(),
        },
    ]
}

#[tauri::command]
pub fn mob_deck_save(state: State<'_, AppState>, mut item: DeckItem) -> CmdResult<Vec<DeckItem>> {
    if item.title.trim().is_empty() {
        return Err("Give this Deck button a name.".into());
    }
    if item.id.trim().is_empty() {
        item.id = uuid::Uuid::new_v4().simple().to_string();
    }
    state
        .db
        .with(|c| {
            c.execute(
                "INSERT OR REPLACE INTO mobile_deck(id, title, icon, kind, payload, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![item.id, item.title.trim(), item.icon, item.kind, item.payload, now()],
            )
        })
        .map_err(|e| e.to_string())?;
    Ok(mob_deck_list(state))
}

#[tauri::command]
pub fn mob_deck_delete(state: State<'_, AppState>, id: String) -> Vec<DeckItem> {
    let _ = state.db.with(|c| c.execute("DELETE FROM mobile_deck WHERE id = ?1", [&id]));
    mob_deck_list(state)
}

#[tauri::command]
pub async fn mob_deck_run(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    item: DeckItem,
) -> CmdResult<String> {
    match item.kind.as_str() {
        "screenshot" => {
            let img = mob_screenshot(app, state, serial).await?;
            Ok(format!("Saved {}", img.name))
        }
        "key" => {
            mob_keyevent(state, serial, item.payload).await?;
            Ok(format!("Ran {}", item.title))
        }
        "url" => {
            mob_send_clipboard(app, state, serial, Some(item.payload), Some(true)).await
        }
        "app" => {
            mob_launch_app(app, state, serial, item.payload, false).await?;
            Ok(format!("Opened {}", item.title))
        }
        "shell" => {
            let adb = locate_adb(&state)?;
            let s = serial.clone();
            let cmd = item.payload.clone();
            let out = tokio::task::spawn_blocking(move || adb_shell(&adb, &s, &cmd, 10))
                .await
                .map_err(|e| e.to_string())??;
            record_timeline(&state, &app, &serial, &serial, "deck", &item.title, &item.payload);
            let summary = out.trim();
            Ok(if summary.is_empty() {
                format!("Ran {}", item.title)
            } else {
                summary.chars().take(120).collect()
            })
        }
        _ => Err("Unknown Deck action type.".into()),
    }
}

#[tauri::command]
pub async fn mob_mic_toggle(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    enabled: bool,
) -> CmdResult<bool> {
    if !enabled {
        if let Ok(mut g) = state.mobile.mic_proc.lock() {
            if let Some((_, mut c)) = g.take() {
                let _ = c.kill();
            }
        }
        return Ok(false);
    }
    if is_child_alive(&state.mobile.mic_proc, &serial) {
        return Ok(true);
    }
    let scrcpy = locate_scrcpy(&state)?;
    let adb = locate_adb(&state)?;
    let mut cmd = hidden_cmd(&scrcpy);
    if let Some(parent) = scrcpy.parent() {
        cmd.current_dir(parent);
    }
    cmd.env("ADB", &adb)
        .arg("-s")
        .arg(&serial)
        .arg("--no-video")
        .arg("--no-control")
        .arg("--audio-source=mic")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Couldn't start phone microphone stream (Android 11+): {e}"))?;
    *state.mobile.mic_proc.lock().unwrap() = Some((serial.clone(), child));
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "audio",
        "Phone as microphone",
        "Streaming phone mic audio to this computer",
    );
    Ok(true)
}

#[tauri::command]
pub async fn mob_record_toggle(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    enabled: bool,
) -> CmdResult<Option<String>> {
    if !enabled {
        let saved = if let Ok(mut g) = state.mobile.rec_proc.lock() {
            if let Some((_, path, mut c)) = g.take() {
                let _ = c.kill();
                let _ = c.wait();
                Some(path)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(p) = saved {
            let p_str = p.to_string_lossy().into_owned();
            let fname = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "recording.mkv".into());
            record_timeline(
                &state,
                &app,
                &serial,
                &serial,
                "mirror",
                "Screen recording saved",
                &fname,
            );
            return Ok(Some(p_str));
        }
        return Ok(None);
    }

    let scrcpy = locate_scrcpy(&state)?;
    let adb = locate_adb(&state)?;
    let dest_dir = phone_save_dir(&state);
    let _ = std::fs::create_dir_all(&dest_dir);
    let out_path = dest_dir.join(format!("phone-recording-{}.mkv", now()));

    let mut cmd = hidden_cmd(&scrcpy);
    if let Some(parent) = scrcpy.parent() {
        cmd.current_dir(parent);
    }
    cmd.env("ADB", &adb)
        .arg("-s")
        .arg(&serial)
        .arg("--no-playback")
        .arg("--no-control")
        .arg("--record")
        .arg(&out_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Couldn't start phone screen recording: {e}"))?;
    *state.mobile.rec_proc.lock().unwrap() = Some((serial.clone(), out_path, child));
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "mirror",
        "Recording phone screen",
        "Saving MKV video to computer",
    );
    Ok(None)
}

#[tauri::command]
pub async fn mob_swipe(
    state: State<'_, AppState>,
    serial: String,
    direction: String,
) -> CmdResult<()> {
    let adb = locate_adb(&state)?;
    let args = match direction.as_str() {
        "up" => "input swipe 540 1500 540 500 220",
        "down" => "input swipe 540 500 540 1500 220",
        "left" => "input swipe 900 1000 180 1000 220",
        "right" => "input swipe 180 1000 900 1000 220",
        "notifications" => "cmd statusbar expand-notifications",
        "quicksettings" => "cmd statusbar expand-settings",
        "collapse" => "cmd statusbar collapse",
        _ => return Err("Unknown gesture.".into()),
    };
    tokio::task::spawn_blocking(move || {
        adb_shell(&adb, &serial, args, 5)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_pull_photos(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    remote_paths: Vec<String>,
) -> CmdResult<usize> {
    if remote_paths.is_empty() {
        return Ok(0);
    }
    let adb = locate_adb(&state)?;
    let dest_dir = phone_save_dir(&state);
    let s = serial.clone();
    let count = tokio::task::spawn_blocking(move || -> CmdResult<usize> {
        let _ = std::fs::create_dir_all(&dest_dir);
        let mut saved = 0usize;
        for rp in remote_paths.iter().take(200) {
            if !rp.starts_with("/sdcard/") {
                continue;
            }
            let name = rp.rsplit('/').next().unwrap_or("photo.jpg");
            let target = nomnom_core::phone::free_path(&dest_dir, name);
            let target_str = target.to_string_lossy().into_owned();
            if run_adb(&adb, &["-s", &s, "pull", rp, &target_str], 45).is_ok() {
                saved += 1;
            }
        }
        Ok(saved)
    })
    .await
    .map_err(|e| e.to_string())??;

    if count > 0 {
        record_timeline(
            &state,
            &app,
            &serial,
            &serial,
            "photo",
            &format!("Saved {count} photos"),
            &format!("Copied to {}", phone_save_dir(&state).display()),
        );
    }
    Ok(count)
}

#[tauri::command]
pub async fn mob_app_action(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    package: String,
    action: String,
) -> CmdResult<String> {
    if package.trim().is_empty() || !package.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        return Err("Invalid package name.".into());
    }
    let adb = locate_adb(&state)?;
    let dest_dir = phone_save_dir(&state);
    let s = serial.clone();
    let pkg = package.clone();
    let act = action.clone();
    let msg = tokio::task::spawn_blocking(move || -> CmdResult<String> {
        match act.as_str() {
            "force_stop" => {
                adb_shell(&adb, &s, &format!("am force-stop {pkg}"), 6)?;
                Ok(format!("Force-stopped {}", friendly_app_name(&pkg)))
            }
            "clear_data" => {
                adb_shell(&adb, &s, &format!("pm clear {pkg}"), 10)?;
                Ok(format!("Cleared cache and data for {}", friendly_app_name(&pkg)))
            }
            "disable" => {
                adb_shell(&adb, &s, &format!("pm disable-user --user 0 {pkg}"), 8)?;
                Ok(format!("Disabled {}", friendly_app_name(&pkg)))
            }
            "enable" => {
                adb_shell(&adb, &s, &format!("pm enable {pkg}"), 8)?;
                Ok(format!("Enabled {}", friendly_app_name(&pkg)))
            }
            "app_info" => {
                adb_shell(
                    &adb,
                    &s,
                    &format!("am start -a android.settings.APPLICATION_DETAILS_SETTINGS -d package:{pkg}"),
                    6,
                )?;
                Ok(format!("Opened App Info for {}", friendly_app_name(&pkg)))
            }
            "pull_apk" => {
                let paths_out = adb_shell(&adb, &s, &format!("pm path {pkg}"), 6)?;
                let apk_path = paths_out
                    .lines()
                    .filter_map(|l| l.trim().strip_prefix("package:"))
                    .next()
                    .ok_or_else(|| "Couldn't locate APK on phone.".to_string())?;
                let _ = std::fs::create_dir_all(&dest_dir);
                let target = nomnom_core::phone::free_path(&dest_dir, &format!("{pkg}.apk"));
                let target_str = target.to_string_lossy().into_owned();
                run_adb(&adb, &["-s", &s, "pull", apk_path, &target_str], 90)?;
                Ok(format!("Backed up APK to {target_str}"))
            }
            _ => Err("Unknown app action.".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(&state, &app, &serial, &serial, "app", &msg, &package);
    Ok(msg)
}

fn clean_sdcard_path(raw: &str) -> CmdResult<String> {
    let trimmed = raw.trim().replace('\\', "/");
    let normalized = if trimmed.is_empty() || trimmed == "/" {
        "/sdcard".to_string()
    } else if trimmed == "/sdcard" || trimmed.starts_with("/sdcard/") {
        trimmed.trim_end_matches('/').to_string()
    } else {
        return Err("Only paths inside /sdcard can be accessed.".into());
    };
    if normalized.split('/').any(|seg| seg == "..") {
        return Err("Invalid folder path.".into());
    }
    Ok(normalized)
}

fn parse_df_kb(df_out: &str) -> (u64, u64, u64) {
    for line in df_out.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let (Ok(total), Ok(used), Ok(avail)) = (
                parts[1].parse::<u64>(),
                parts[2].parse::<u64>(),
                parts[3].parse::<u64>(),
            ) {
                return (total, used, avail);
            }
        }
    }
    (0, 0, 0)
}

#[tauri::command]
pub async fn mob_fs_list(
    state: State<'_, AppState>,
    serial: String,
    dir: String,
) -> CmdResult<FsListing> {
    let adb = locate_adb(&state)?;
    let clean = clean_sdcard_path(&dir)?;
    tokio::task::spawn_blocking(move || -> CmdResult<FsListing> {
        let q = shell_single_quote(&clean);
        let script = format!(
            "echo '===DF==='; df -k /data 2>/dev/null || df -k /sdcard 2>/dev/null; \
             echo '===LS==='; \
             if [ -d {q} ]; then \
               for item in {q}/*; do \
                 [ -e \"$item\" ] || continue; \
                 if [ -d \"$item\" ]; then t=D; sz=0; else t=F; sz=$(stat -c '%s' \"$item\" 2>/dev/null || echo 0); fi; \
                 mt=$(stat -c '%Y' \"$item\" 2>/dev/null || echo 0); \
                 echo \"$t|$sz|$mt|$item\"; \
               done; \
             fi"
        );
        let raw = adb_shell(&adb, &serial, &script, 12)?;
        let (df_part, ls_part) = match raw.split_once("===LS===") {
            Some((a, b)) => (a, b),
            None => (raw.as_str(), ""),
        };
        let (storage_total_kb, storage_used_kb, storage_avail_kb) =
            parse_df_kb(df_part.replace("===DF===", "").trim());

        let mut entries = Vec::new();
        for line in ls_part.lines() {
            let mut parts = line.splitn(4, '|');
            let Some(kind) = parts.next() else { continue };
            let Some(sz_s) = parts.next() else { continue };
            let Some(mt_s) = parts.next() else { continue };
            let Some(path) = parts.next() else { continue };
            if kind != "D" && kind != "F" {
                continue;
            }
            let path = path.trim().to_string();
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            entries.push(FsEntry {
                name,
                path,
                is_dir: kind == "D",
                size: sz_s.parse().unwrap_or(0),
                mtime: mt_s.parse().unwrap_or(0),
            });
        }
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(FsListing {
            dir: clean,
            entries,
            storage_total_kb,
            storage_used_kb,
            storage_avail_kb,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_fs_push(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    dir: String,
    paths: Vec<String>,
) -> CmdResult<usize> {
    let adb = locate_adb(&state)?;
    let clean = clean_sdcard_path(&dir)?;
    let s = serial.clone();
    let target_dir = clean.clone();
    let sent = tokio::task::spawn_blocking(move || -> CmdResult<usize> {
        let mut n = 0usize;
        for p in paths {
            let lp = Path::new(&p);
            if !lp.is_file() {
                continue;
            }
            let fname = lp
                .file_name()
                .map(|x| x.to_string_lossy().into_owned())
                .unwrap_or_else(|| "file".into());
            let remote = format!("{target_dir}/{fname}");
            run_adb(&adb, &["-s", &s, "push", &p, &remote], 120)?;
            n += 1;
        }
        Ok(n)
    })
    .await
    .map_err(|e| e.to_string())??;

    if sent > 0 {
        record_timeline(
            &state,
            &app,
            &serial,
            &serial,
            "file",
            &format!("Uploaded {sent} {}", if sent == 1 { "file" } else { "files" }),
            &clean,
        );
    }
    Ok(sent)
}

#[tauri::command]
pub async fn mob_fs_pull(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    remote_path: String,
) -> CmdResult<String> {
    let clean = clean_sdcard_path(&remote_path)?;
    let adb = locate_adb(&state)?;
    let dest_dir = phone_save_dir(&state);
    let s = serial.clone();
    let rp = clean.clone();
    let saved = tokio::task::spawn_blocking(move || -> CmdResult<String> {
        let _ = std::fs::create_dir_all(&dest_dir);
        let name = rp.rsplit('/').next().unwrap_or("file");
        let target = nomnom_core::phone::free_path(&dest_dir, name);
        let target_str = target.to_string_lossy().into_owned();
        run_adb(&adb, &["-s", &s, "pull", &rp, &target_str], 120)?;
        Ok(target_str)
    })
    .await
    .map_err(|e| e.to_string())??;

    let name = clean.rsplit('/').next().unwrap_or(&clean);
    record_timeline(&state, &app, &serial, &serial, "file", &format!("Downloaded {name}"), &saved);
    Ok(saved)
}

#[tauri::command]
pub async fn mob_fs_mkdir(
    state: State<'_, AppState>,
    serial: String,
    path: String,
) -> CmdResult<()> {
    let clean = clean_sdcard_path(&path)?;
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || {
        adb_shell(&adb, &serial, &format!("mkdir -p {}", shell_single_quote(&clean)), 6)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_fs_rename(
    state: State<'_, AppState>,
    serial: String,
    from: String,
    to: String,
) -> CmdResult<()> {
    let a = clean_sdcard_path(&from)?;
    let b = clean_sdcard_path(&to)?;
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || {
        adb_shell(
            &adb,
            &serial,
            &format!("mv {} {}", shell_single_quote(&a), shell_single_quote(&b)),
            8,
        )?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_fs_delete(
    state: State<'_, AppState>,
    serial: String,
    path: String,
) -> CmdResult<()> {
    let clean = clean_sdcard_path(&path)?;
    if clean == "/sdcard" {
        return Err("Cannot delete the root storage folder.".into());
    }
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || {
        adb_shell(&adb, &serial, &format!("rm -rf {}", shell_single_quote(&clean)), 12)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn parse_meminfo_kb(text: &str) -> (u64, u64) {
    let mut total = 0u64;
    let mut avail = 0u64;
    for line in text.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("MemTotal:") {
            total = rest.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0);
        } else if let Some(rest) = l.strip_prefix("MemAvailable:") {
            avail = rest.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0);
        }
    }
    (total, avail)
}

#[tauri::command]
pub async fn mob_system_info(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<SystemInfo> {
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<SystemInfo> {
        let script = concat!(
            "echo '===PROPS==='; ",
            "echo \"model=$(getprop ro.product.model)\"; ",
            "echo \"mfr=$(getprop ro.product.manufacturer)\"; ",
            "echo \"brand=$(getprop ro.product.brand)\"; ",
            "echo \"device=$(getprop ro.product.device)\"; ",
            "echo \"rel=$(getprop ro.build.version.release)\"; ",
            "echo \"sdk=$(getprop ro.build.version.sdk)\"; ",
            "echo \"patch=$(getprop ro.build.version.security_patch)\"; ",
            "echo \"build=$(getprop ro.build.display.id)\"; ",
            "echo \"abi=$(getprop ro.product.cpu.abi)\"; ",
            "echo \"hw=$(getprop ro.board.platform) $(getprop ro.hardware)\"; ",
            "echo \"carrier=$(getprop gsm.operator.alpha)\"; ",
            "echo \"cores=$(nproc 2>/dev/null || echo 0)\"; ",
            "echo \"kernel=$(uname -r 2>/dev/null)\"; ",
            "echo \"ip=$(ip route | awk '/wlan0/ && /src/ {print $9; exit}')\"; ",
            "echo \"airplane=$(settings get global airplane_mode_on)\"; ",
            "echo \"anim=$(settings get global animator_duration_scale)\"; ",
            "echo \"touches=$(settings get system show_touches)\"; ",
            "echo \"stayon=$(settings get global stay_on_while_plugged_in)\"; ",
            "echo \"night=$(cmd uimode night 2>/dev/null)\"; ",
            "echo '===WM==='; wm size; wm density; ",
            "echo '===MEM==='; head -n 5 /proc/meminfo; ",
            "echo '===DF==='; df -k /data 2>/dev/null || df -k /sdcard 2>/dev/null; ",
            "echo '===BAT==='; dumpsys battery; ",
            "echo '===SSID==='; dumpsys wifi 2>/dev/null | grep -m 1 'SSID:'"
        );
        let raw = adb_shell(&adb, &serial, script, 10)?;
        let sec = |tag: &str, next: Option<&str>| -> &str {
            let m = format!("==={tag}===");
            let Some(start) = raw.find(&m) else { return "" };
            let rest = &raw[start + m.len()..];
            if let Some(n) = next {
                if let Some(end) = rest.find(&format!("==={n}===")) {
                    return rest[..end].trim();
                }
            }
            rest.trim()
        };
        let mut kv = HashMap::new();
        for l in sec("PROPS", Some("WM")).lines() {
            if let Some((k, v)) = l.split_once('=') {
                kv.insert(k.trim(), v.trim().to_string());
            }
        }
        let get = |k: &str| kv.get(k).cloned().unwrap_or_default();

        let mut screen_size = String::new();
        let mut screen_density = String::new();
        for l in sec("WM", Some("MEM")).lines() {
            if l.contains("size:") {
                if let Some((_, v)) = l.split_once(':') {
                    screen_size = v.trim().replace('x', "×");
                }
            } else if l.contains("density:") {
                if let Some((_, v)) = l.split_once(':') {
                    screen_density = format!("{} dpi", v.trim());
                }
            }
        }
        let (ram_total_kb, ram_avail_kb) = parse_meminfo_kb(sec("MEM", Some("DF")));
        let (storage_total_kb, storage_used_kb, storage_avail_kb) = parse_df_kb(sec("DF", Some("BAT")));

        let bat_raw = sec("BAT", Some("SSID"));
        let bat = parse_battery(bat_raw);
        let mut voltage_mv = 0u32;
        let mut health_code = 2u32;
        let mut tech = String::new();
        for l in bat_raw.lines() {
            let t = l.trim();
            if let Some(v) = t.strip_prefix("voltage:") {
                let n: u32 = v.trim().parse().unwrap_or(0);
                voltage_mv = if n > 100_000 { n / 1000 } else { n };
            } else if let Some(v) = t.strip_prefix("health:") {
                health_code = v.trim().parse().unwrap_or(2);
            } else if let Some(v) = t.strip_prefix("technology:") {
                tech = v.trim().to_string();
            }
        }
        let battery_health = match health_code {
            2 => "Good",
            3 => "Overheat",
            4 => "Dead",
            5 => "Over voltage",
            7 => "Cold",
            _ => "Unknown",
        }
        .to_string();

        let ssid_line = sec("SSID", None);
        let wifi_ssid = ssid_line
            .split("SSID:")
            .nth(1)
            .map(|s| s.split(',').next().unwrap_or("").trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty() && s != "<unknown ssid>")
            .unwrap_or_default();

        Ok(SystemInfo {
            model: get("model"),
            manufacturer: get("mfr"),
            brand: get("brand"),
            device: get("device"),
            android_version: get("rel"),
            sdk_level: get("sdk").parse().unwrap_or(0),
            security_patch: get("patch"),
            build_id: get("build"),
            kernel: get("kernel"),
            abi: get("abi"),
            hardware: get("hw").trim().to_string(),
            cpu_cores: get("cores").parse().unwrap_or(0),
            ram_total_kb,
            ram_avail_kb,
            storage_total_kb,
            storage_used_kb,
            storage_avail_kb,
            battery_level: bat.level,
            battery_temp_c: bat.temp_c,
            battery_voltage_mv: voltage_mv,
            battery_health,
            battery_tech: tech,
            battery_charging: bat.charging,
            wifi_ssid,
            wifi_ip: get("ip"),
            carrier: get("carrier"),
            airplane_mode: get("airplane") == "1",
            screen_size,
            screen_density,
            dark_mode: get("night").to_lowercase().contains("yes"),
            anim_scale: {
                let a = get("anim");
                if a.is_empty() || a == "null" { "1.0".into() } else { a }
            },
            show_touches: get("touches") == "1",
            stay_awake_plugged: get("stayon") != "0" && !get("stayon").is_empty() && get("stayon") != "null",
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_set_tweak(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    tweak: String,
    value: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let s = serial.clone();
    let t = tweak.clone();
    let v = value.clone();
    let note = tokio::task::spawn_blocking(move || -> CmdResult<String> {
        match t.as_str() {
            "dark_mode" => {
                let yes = v == "on" || v == "true" || v == "yes";
                adb_shell(&adb, &s, &format!("cmd uimode night {}", if yes { "yes" } else { "no" }), 5)?;
                Ok(if yes { "Dark mode enabled".into() } else { "Light mode enabled".into() })
            }
            "anim_scale" => {
                let scale = match v.as_str() {
                    "0" | "0.0" => "0.0",
                    "0.5" => "0.5",
                    "1" | "1.0" => "1.0",
                    "1.5" => "1.5",
                    _ => return Err("Unsupported animation scale.".into()),
                };
                adb_shell(
                    &adb,
                    &s,
                    &format!(
                        "settings put global window_animation_scale {scale}; \
                         settings put global transition_animation_scale {scale}; \
                         settings put global animator_duration_scale {scale}"
                    ),
                    6,
                )?;
                Ok(format!("Animation speed set to {scale}x"))
            }
            "density" => {
                if v == "reset" {
                    adb_shell(&adb, &s, "wm density reset", 5)?;
                    Ok("Display density reset to default".into())
                } else {
                    let dpi: u32 = v.parse().map_err(|_| "Enter a valid DPI number (e.g. 420).")?;
                    if !(160..=720).contains(&dpi) {
                        return Err("Choose a safe DPI between 160 and 720.".into());
                    }
                    adb_shell(&adb, &s, &format!("wm density {dpi}"), 5)?;
                    Ok(format!("Display density set to {dpi} dpi"))
                }
            }
            "size" => {
                if v == "reset" {
                    adb_shell(&adb, &s, "wm size reset", 5)?;
                    Ok("Display resolution reset to default".into())
                } else {
                    let clean = v.replace('×', "x");
                    let ok = clean
                        .split_once('x')
                        .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
                        .is_some_and(|(w, h)| (480..=3840).contains(&w) && (480..=3840).contains(&h));
                    if !ok {
                        return Err("Enter resolution like 1080x2340.".into());
                    }
                    adb_shell(&adb, &s, &format!("wm size {clean}"), 5)?;
                    Ok(format!("Display resolution set to {clean}"))
                }
            }
            "show_touches" => {
                let on = v == "on" || v == "true";
                adb_shell(&adb, &s, &format!("settings put system show_touches {}", if on { 1 } else { 0 }), 5)?;
                Ok(if on { "Show touches enabled".into() } else { "Show touches disabled".into() })
            }
            "stay_awake" => {
                let on = v == "on" || v == "true";
                adb_shell(&adb, &s, &format!("svc power stayon {}", if on { "usb" } else { "false" }), 5)?;
                Ok(if on { "Stay awake while charging enabled".into() } else { "Stay awake disabled".into() })
            }
            "airplane" => {
                let on = v == "on" || v == "true";
                adb_shell(&adb, &s, &format!("cmd connectivity airplane-mode {}", if on { "enable" } else { "disable" }), 5)?;
                Ok(if on { "Airplane mode enabled".into() } else { "Airplane mode disabled".into() })
            }
            _ => Err("Unknown system tweak.".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(&state, &app, &serial, &serial, "control", &note, &tweak);
    Ok(note)
}

#[tauri::command]
pub async fn mob_power(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    mode: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let s = serial.clone();
    let m = mode.clone();
    let note = tokio::task::spawn_blocking(move || -> CmdResult<String> {
        match m.as_str() {
            "wake" => {
                adb_shell(&adb, &s, "input keyevent 224", 4)?;
                Ok("Woke phone screen".into())
            }
            "sleep" => {
                adb_shell(&adb, &s, "input keyevent 223", 4)?;
                Ok("Locked phone screen".into())
            }
            "reboot" => {
                run_adb(&adb, &["-s", &s, "reboot"], 8)?;
                Ok("Rebooting phone…".into())
            }
            "recovery" => {
                run_adb(&adb, &["-s", &s, "reboot", "recovery"], 8)?;
                Ok("Rebooting into recovery…".into())
            }
            "bootloader" => {
                run_adb(&adb, &["-s", &s, "reboot", "bootloader"], 8)?;
                Ok("Rebooting into bootloader…".into())
            }
            _ => Err("Unknown power action.".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(&state, &app, &serial, &serial, "control", &note, &mode);
    Ok(note)
}

#[tauri::command]
pub async fn mob_shell(
    state: State<'_, AppState>,
    serial: String,
    command: String,
) -> CmdResult<ShellOutput> {
    if command.trim().is_empty() {
        return Ok(ShellOutput { stdout: String::new(), stderr: String::new(), ok: true });
    }
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<ShellOutput> {
        let mut child = hidden_cmd(&adb)
            .args(["-s", &serial, "shell", &command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Couldn't start adb shell: {e}"))?;
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(ShellOutput {
                        stdout: String::new(),
                        stderr: "Command timed out after 15 seconds.".into(),
                        ok: false,
                    });
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                Err(e) => return Err(e.to_string()),
            }
        }
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        Ok(ShellOutput {
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            ok: out.status.success(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_logcat(
    state: State<'_, AppState>,
    serial: String,
    level: Option<String>,
    lines: Option<u32>,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let lv = match level.as_deref() {
        Some("E") => "E",
        Some("W") => "W",
        Some("I") => "I",
        _ => "V",
    };
    let n = lines.unwrap_or(150).clamp(20, 500);
    tokio::task::spawn_blocking(move || {
        adb_shell(&adb, &serial, &format!("logcat -d -t {n} *:{lv}"), 10)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_logcat_clear(state: State<'_, AppState>, serial: String) -> CmdResult<()> {
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || {
        adb_shell(&adb, &serial, "logcat -c", 5)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn parse_tunnel_list(raw: &str, serial: &str, direction: &str) -> Vec<PortTunnel> {
    raw.lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            if parts.len() >= 3 && (parts[0] == serial || parts[0].contains(serial)) {
                Some(PortTunnel {
                    direction: direction.into(),
                    local: parts[1].into(),
                    remote: parts[2].into(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[tauri::command]
pub async fn mob_tunnels_list(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<Vec<PortTunnel>> {
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<Vec<PortTunnel>> {
        let rev = run_adb(&adb, &["-s", &serial, "reverse", "--list"], 5).unwrap_or_default();
        let fwd = run_adb(&adb, &["-s", &serial, "forward", "--list"], 5).unwrap_or_default();
        let mut all = parse_tunnel_list(&rev, &serial, "reverse");
        all.extend(parse_tunnel_list(&fwd, &serial, "forward"));
        Ok(all)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_tunnel_add(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    direction: String,
    local_port: u16,
    remote_port: u16,
) -> CmdResult<Vec<PortTunnel>> {
    let adb = locate_adb(&state)?;
    let dir = if direction == "forward" { "forward" } else { "reverse" };
    let s = serial.clone();
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        let a = format!("tcp:{remote_port}");
        let b = format!("tcp:{local_port}");
        if dir == "reverse" {
            run_adb(&adb, &["-s", &s, "reverse", &a, &b], 6)?;
        } else {
            run_adb(&adb, &["-s", &s, "forward", &b, &a], 6)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "connect",
        &format!("Port {dir} :{local_port} ↔ :{remote_port}"),
        if dir == "reverse" {
            "Phone can open this computer's localhost"
        } else {
            "Computer can open phone's localhost"
        },
    );
    mob_tunnels_list(state, serial).await
}

#[tauri::command]
pub async fn mob_tunnel_remove(
    state: State<'_, AppState>,
    serial: String,
    direction: String,
    spec: String,
) -> CmdResult<Vec<PortTunnel>> {
    let adb = locate_adb(&state)?;
    let dir = if direction == "forward" { "forward" } else { "reverse" };
    let s = serial.clone();
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        if spec == "all" {
            let _ = run_adb(&adb, &["-s", &s, dir, "--remove-all"], 5);
        } else {
            let _ = run_adb(&adb, &["-s", &s, dir, "--remove", &spec], 5);
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    mob_tunnels_list(state, serial).await
}

fn run_scrcpy_probe(scrcpy: &Path, adb: &Path, args: &[&str], timeout_secs: u64) -> CmdResult<String> {
    let mut cmd = hidden_cmd(scrcpy);
    if let Some(parent) = scrcpy.parent() {
        cmd.current_dir(parent);
    }
    let mut child = cmd
        .env("ADB", adb)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't start scrcpy: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Timed out reading camera list from phone.".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.is_empty() {
        combined.push('\n');
        combined.push_str(&err);
    }
    Ok(combined)
}

fn parse_scrcpy_cameras(raw: &str) -> Vec<CameraInfo> {
    let mut cameras: Vec<CameraInfo> = Vec::new();
    let mut back_idx = 0u32;
    let mut front_idx = 0u32;
    let mut in_high_speed = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("--camera-id=") {
            // Example: --camera-id=0 (back, 4000x3000, fps=[15, 24, 30, 60])
            let id = rest.split_whitespace().next().unwrap_or("0").to_string();
            let inside = rest
                .split_once('(')
                .map(|(_, r)| r.trim_end_matches(')'))
                .unwrap_or("");
            let facing = if inside.contains("front") {
                "front"
            } else if inside.contains("external") {
                "external"
            } else {
                "back"
            }
            .to_string();
            let label = if facing == "front" {
                front_idx += 1;
                if front_idx == 1 {
                    format!("Front camera (ID {id})")
                } else {
                    format!("Front lens #{front_idx} (ID {id})")
                }
            } else if facing == "back" {
                back_idx += 1;
                match back_idx {
                    1 => format!("Back main lens (ID {id})"),
                    2 => format!("Back lens #2 · Wide/Ultra-wide (ID {id})"),
                    3 => format!("Back lens #3 · Tele/Macro (ID {id})"),
                    _ => format!("Back lens #{back_idx} (ID {id})"),
                }
            } else {
                format!("External camera (ID {id})")
            };

            let mut fps = Vec::new();
            if let Some(fpart) = inside.split("fps=[").nth(1) {
                let nums = fpart.split(']').next().unwrap_or("");
                for tok in nums.split(',') {
                    if let Ok(n) = tok.trim().parse::<u32>() {
                        fps.push(n);
                    }
                }
            }
            in_high_speed = false;
            cameras.push(CameraInfo {
                id,
                facing,
                label,
                sizes: Vec::new(),
                fps,
                high_speed: Vec::new(),
            });
            continue;
        }
        let Some(cur) = cameras.last_mut() else {
            continue;
        };
        if trimmed.starts_with("High speed capture:") {
            in_high_speed = true;
            continue;
        }
        if let Some(sz) = trimmed.strip_prefix("- ") {
            let val = sz.trim().to_string();
            if in_high_speed {
                if !val.is_empty() && cur.high_speed.len() < 16 {
                    cur.high_speed.push(val);
                }
            } else if val.contains('x') && cur.sizes.len() < 32 {
                cur.sizes.push(val);
            }
        }
    }
    cameras
}

fn parse_scrcpy_displays(raw: &str) -> Vec<DisplayInfo> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("--display-id=") {
            let mut parts = rest.splitn(2, ' ');
            let Some(id_str) = parts.next() else { continue };
            let Ok(id) = id_str.parse::<u32>() else { continue };
            let desc = parts.next().unwrap_or("").trim();
            out.push(DisplayInfo {
                id,
                label: if desc.is_empty() {
                    format!("Display #{id}")
                } else {
                    format!("Display #{id} {desc}")
                },
            });
        }
    }
    out
}

fn find_obs_binary() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let candidates = [
            PathBuf::from(r"C:\Program Files\obs-studio\bin\64bit\obs64.exe"),
            PathBuf::from(r"C:\Program Files (x86)\obs-studio\bin\64bit\obs64.exe"),
        ];
        for c in candidates {
            if c.is_file() {
                return Some(c);
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let path_var = std::env::var_os("PATH")?;
        std::env::split_paths(&path_var)
            .map(|d| d.join("obs"))
            .find(|p| p.is_file())
    }
}

fn list_v4l2_devices() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        let mut list: Vec<String> = std::fs::read_dir("/dev")
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.starts_with("video").then(|| format!("/dev/{name}"))
            })
            .collect();
        list.sort();
        list
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

#[tauri::command]
pub async fn mob_cameras(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<CameraCatalog> {
    let scrcpy = locate_scrcpy(&state)?;
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<CameraCatalog> {
        let sdk_level = adb_shell(&adb, &serial, "getprop ro.build.version.sdk", 4)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let raw = run_scrcpy_probe(&scrcpy, &adb, &["-s", &serial, "--list-camera-sizes"], 12)?;
        let cameras = parse_scrcpy_cameras(&raw);
        let obs_path = find_obs_binary().map(|p| p.to_string_lossy().into_owned());
        Ok(CameraCatalog {
            cameras,
            v4l2_devices: list_v4l2_devices(),
            obs_path,
            supports_usb_uvc: sdk_level >= 34,
            sdk_level,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_webcam_pro(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
    options: Option<WebcamProOptions>,
    stop: Option<bool>,
) -> CmdResult<bool> {
    if stop.unwrap_or(false) {
        if let Ok(mut g) = state.mobile.webcam_proc.lock() {
            if let Some((_, mut c)) = g.take() {
                let _ = c.kill();
            }
        }
        return Ok(false);
    }
    // Restart cleanly if already active so new settings take effect immediately.
    if let Ok(mut g) = state.mobile.webcam_proc.lock() {
        if let Some((_, mut c)) = g.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    let scrcpy = locate_scrcpy(&state)?;
    let adb = locate_adb(&state)?;
    let opts = options.unwrap_or_default();

    let mut cmd = hidden_cmd(&scrcpy);
    if let Some(parent) = scrcpy.parent() {
        cmd.current_dir(parent);
    }
    cmd.env("ADB", &adb)
        .arg("-s")
        .arg(&serial)
        .arg("--video-source=camera")
        .arg("--window-title")
        .arg("NomNom Webcam");

    if let Some(ref cid) = opts.camera_id.as_deref().filter(|s| !s.is_empty()) {
        cmd.arg(format!("--camera-id={cid}"));
    } else {
        let face = match opts.facing.as_deref() {
            Some("front") => "front",
            _ => "back",
        };
        cmd.arg(format!("--camera-facing={face}"));
    }

    if let Some(ref sz) = opts
        .size
        .as_deref()
        .filter(|s| s.contains('x') && s.chars().all(|c| c.is_ascii_digit() || c == 'x'))
    {
        cmd.arg(format!("--camera-size={sz}"));
    } else if let Some(ref ar) = opts
        .aspect_ratio
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "sensor")
    {
        cmd.arg(format!("--camera-ar={ar}"));
    }

    if let Some(fps) = opts.fps.filter(|&f| f > 0 && f <= 240) {
        cmd.arg(format!("--camera-fps={fps}"));
    }
    if opts.high_speed.unwrap_or(false) {
        cmd.arg("--camera-high-speed");
    }
    if let Some(codec) = opts.video_codec.as_deref() {
        if matches!(codec, "h264" | "h265" | "av1") {
            cmd.arg(format!("--video-codec={codec}"));
        }
    }
    if let Some(mbps) = opts.bit_rate_mbps.filter(|&b| b > 0 && b <= 64) {
        cmd.arg("--video-bit-rate").arg(format!("{mbps}M"));
    }

    let deg = opts.orientation.unwrap_or(0);
    let flip = opts.flip.unwrap_or(false);
    let lock = opts.lock_orientation.unwrap_or(false);
    if deg != 0 || flip || lock {
        let base = if flip {
            format!("flip{deg}")
        } else {
            format!("{deg}")
        };
        let spec = if lock { format!("@{base}") } else { base };
        cmd.arg(format!("--capture-orientation={spec}"));
    }

    if opts.audio_mic.unwrap_or(false) {
        cmd.arg("--audio-source=mic");
    } else {
        cmd.arg("--no-audio");
    }
    if opts.borderless.unwrap_or(false) {
        cmd.arg("--window-borderless");
    }
    if opts.always_on_top.unwrap_or(false) {
        cmd.arg("--always-on-top");
    }
    if let Some(buf) = opts.video_buffer_ms.filter(|&b| b > 0 && b <= 500) {
        cmd.arg(format!("--video-buffer={buf}"));
    }
    if let Some(buf) = opts.audio_buffer_ms.filter(|&b| b > 0 && b <= 500) {
        cmd.arg(format!("--audio-buffer={buf}"));
    }
    if opts.record.unwrap_or(false) {
        let dest = phone_save_dir(&state);
        let _ = std::fs::create_dir_all(&dest);
        cmd.arg("--record")
            .arg(dest.join(format!("webcam-{}.mkv", now())));
    }
    #[cfg(target_os = "linux")]
    if let Some(ref sink) = opts.v4l2_sink.as_deref().filter(|s| s.starts_with("/dev/video")) {
        cmd.arg(format!("--v4l2-sink={sink}"));
    }

    let child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Couldn't start Webcam Studio: {e}"))?;
    *state.mobile.webcam_proc.lock().unwrap() = Some((serial.clone(), child));

    let detail = format!(
        "{} · {} fps · {}",
        opts.size.as_deref().unwrap_or("auto"),
        opts.fps.unwrap_or(30),
        opts.video_codec.as_deref().unwrap_or("h264").to_uppercase()
    );
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "webcam",
        "NomNom Webcam Studio",
        &detail,
    );
    Ok(true)
}

#[tauri::command]
pub async fn mob_usb_webcam(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<String> {
    let adb = locate_adb(&state)?;
    let s = serial.clone();
    tokio::task::spawn_blocking(move || -> CmdResult<()> {
        adb_shell(&adb, &s, "svc usb setFunctions uvc", 6)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "webcam",
        "Native USB Webcam (UVC)",
        "Phone switched to hardware USB webcam mode",
    );
    Ok("Switched phone to native USB Webcam mode. Select Android Webcam in Discord, Zoom or Meet.".into())
}

#[tauri::command]
pub fn mob_launch_obs() -> CmdResult<String> {
    let obs = find_obs_binary().ok_or_else(|| {
        "OBS Studio wasn't found in the default folder. Install OBS Studio to use OBS Virtual Camera with Discord or Zoom.".to_string()
    })?;
    let mut cmd = Command::new(&obs);
    if let Some(parent) = obs.parent() {
        cmd.current_dir(parent);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Couldn't open OBS Studio: {e}"))?;
    Ok("Opened OBS Studio".into())
}

#[tauri::command]
pub async fn mob_displays(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<Vec<DisplayInfo>> {
    let scrcpy = locate_scrcpy(&state)?;
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<Vec<DisplayInfo>> {
        let raw = run_scrcpy_probe(&scrcpy, &adb, &["-s", &serial, "--list-displays"], 10)?;
        Ok(parse_scrcpy_displays(&raw))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn build_vcard(contacts: &[PhoneContact]) -> String {
    let mut out = String::new();
    for c in contacts {
        let name = c.name.replace(['\r', '\n'], " ");
        let num = c.number.replace(['\r', '\n'], "");
        out.push_str("BEGIN:VCARD\r\nVERSION:3.0\r\n");
        out.push_str(&format!("FN:{name}\r\n"));
        out.push_str(&format!("TEL;TYPE=CELL:{num}\r\n"));
        out.push_str("END:VCARD\r\n");
    }
    out
}

#[tauri::command]
pub async fn mob_contacts(
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<Vec<PhoneContact>> {
    let adb = locate_adb(&state)?;
    tokio::task::spawn_blocking(move || -> CmdResult<Vec<PhoneContact>> {
        let out = adb_shell(
            &adb,
            &serial,
            "content query --uri content://com.android.contacts/data/phones --projection display_name:data1",
            12,
        )?;
        if out.contains("Permission Denial") || out.contains("Error while accessing provider") {
            return Err("This phone restricts shell contact reading.".into());
        }
        let mut seen = HashSet::new();
        let mut list = Vec::new();
        for row in parse_content_rows(&out) {
            let name = row.get("display_name").cloned().unwrap_or_default();
            let number = row.get("data1").cloned().unwrap_or_default();
            if number.is_empty() {
                continue;
            }
            let key = format!("{}:{}", name.to_lowercase(), number.replace(' ', ""));
            if seen.insert(key) {
                list.push(PhoneContact {
                    name: if name.is_empty() { number.clone() } else { name },
                    number,
                });
            }
        }
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(list)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mob_export_contacts(
    app: AppHandle,
    state: State<'_, AppState>,
    serial: String,
) -> CmdResult<String> {
    let contacts = mob_contacts(state.clone(), serial.clone()).await?;
    if contacts.is_empty() {
        return Err("No contacts found to export.".into());
    }
    let dest_dir = phone_save_dir(&state);
    let _ = std::fs::create_dir_all(&dest_dir);
    let path = nomnom_core::phone::free_path(&dest_dir, &format!("contacts-{}.vcf", now()));
    let vcf = build_vcard(&contacts);
    std::fs::write(&path, vcf).map_err(|e| format!("Couldn't save contacts file: {e}"))?;
    let path_str = path.to_string_lossy().into_owned();
    record_timeline(
        &state,
        &app,
        &serial,
        &serial,
        "file",
        &format!("Exported {} contacts (.vcf)", contacts.len()),
        &path_str,
    );
    Ok(path_str)
}

fn save_wifi_device(state: &AppState, endpoint: &str, name: &str) {
    let ep = endpoint.trim();
    if ep.is_empty() {
        return;
    }
    let ip = ep.split(':').next().unwrap_or(ep).to_string();
    let label = if name.trim().is_empty() { ep } else { name.trim() };
    let _ = state.db.with(|c| {
        c.execute(
            "INSERT INTO mobile_wifi_devices(endpoint, name, ip, last_seen)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(endpoint) DO UPDATE SET name=excluded.name, ip=excluded.ip, last_seen=excluded.last_seen",
            rusqlite::params![ep, label, ip, now()],
        )
    });
}

fn list_saved_wifi_devices(state: &AppState) -> Vec<SavedWifiDevice> {
    state
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT endpoint, name, ip, last_seen FROM mobile_wifi_devices ORDER BY last_seen DESC LIMIT 12",
            )?;
            let rows = st.query_map([], |r| {
                Ok(SavedWifiDevice {
                    endpoint: r.get(0)?,
                    name: r.get(1)?,
                    ip: r.get(2)?,
                    last_seen: r.get(3)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap_or_default()
}

fn parse_mdns_services(raw: &str) -> Vec<MdnsService> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("List of")
            || trimmed.starts_with('*')
            || trimmed.starts_with("adb:")
        {
            continue;
        }
        let cols: Vec<&str> = trimmed.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        let name = cols[0].to_string();
        let stype = cols[1].trim_end_matches('.').to_string();
        let endpoint = cols[2].to_string();
        let kind = if stype.contains("adb-tls-connect") {
            "connect"
        } else if stype.contains("adb-tls-pairing") {
            "pair"
        } else {
            continue;
        };
        let Some((ip, port_str)) = endpoint.rsplit_once(':') else {
            continue;
        };
        let Ok(port) = port_str.parse::<u16>() else {
            continue;
        };
        let ip_clean = ip.trim_matches(|c| c == '[' || c == ']').to_string();
        if !seen.insert(format!("{kind}:{endpoint}")) {
            continue;
        }
        out.push(MdnsService {
            name,
            service_type: stype,
            endpoint,
            ip: ip_clean,
            port,
            kind: kind.to_string(),
        });
    }
    out
}

fn build_wifi_qr_payload(service: &str, password: &str) -> String {
    format!("WIFI:T:ADB;S:{service};P:{password};;")
}

fn render_qr_svg(payload: &str) -> CmdResult<String> {
    let code = qrcode::QrCode::new(payload.as_bytes())
        .map_err(|e| format!("Couldn't build QR code: {e}"))?;
    Ok(code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(220, 220)
        .quiet_zone(true)
        .dark_color(qrcode::render::svg::Color("#000000"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build())
}

fn generate_qr_credentials() -> (String, String) {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seed = format!("nomnom-adb-{}-{}-{}", std::process::id(), nanos, n);
    let hash = blake3::hash(seed.as_bytes());
    let hex = hash.to_hex();
    let service = format!("ADB_WIFI_nomnom_{}", &hex[..8]);
    let password = hex[8..16].to_string();
    (service, password)
}

#[tauri::command]
pub async fn mob_wifi_discover(state: State<'_, AppState>) -> CmdResult<WifiDiscovery> {
    let saved = list_saved_wifi_devices(&state);
    let Some(adb) = state.tools.locate(Tool::Adb) else {
        return Ok(WifiDiscovery {
            services: Vec::new(),
            saved,
        });
    };
    let services = tokio::task::spawn_blocking(move || {
        run_adb(&adb, &["mdns", "services"], 6)
            .map(|out| parse_mdns_services(&out))
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default();
    Ok(WifiDiscovery { services, saved })
}

#[tauri::command]
pub async fn mob_wifi_forget(state: State<'_, AppState>, endpoint: String) -> CmdResult<()> {
    let _ = state.db.with(|c| {
        c.execute(
            "DELETE FROM mobile_wifi_devices WHERE endpoint = ?1",
            rusqlite::params![endpoint.trim()],
        )
    });
    Ok(())
}

#[tauri::command]
pub async fn mob_wifi_autoconnect(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<Vec<String>> {
    let Some(adb) = state.tools.locate(Tool::Adb) else {
        return Ok(Vec::new());
    };
    let saved = list_saved_wifi_devices(&state);
    let connected = tokio::task::spawn_blocking(move || -> Vec<(String, String)> {
        let mut results = Vec::new();
        let mut tried = HashSet::new();
        // 1. Discover any already-paired Android 11+ phones broadcasting `_adb-tls-connect._tcp`.
        if let Ok(mdns_raw) = run_adb(&adb, &["mdns", "services"], 5) {
            for svc in parse_mdns_services(&mdns_raw) {
                if svc.kind != "connect" || !tried.insert(svc.endpoint.clone()) {
                    continue;
                }
                if let Ok(out) = run_adb(&adb, &["connect", &svc.endpoint], 5) {
                    let low = out.to_lowercase();
                    if low.contains("connected to") && !low.contains("cannot connect") {
                        let name = device_display_name(&adb, &svc.endpoint);
                        results.push((svc.endpoint, name));
                    }
                }
            }
        }
        // 2. Try saved Wi-Fi endpoints (with quick 450ms TCP reachability check so offline phones never stall).
        for dev in saved {
            if !tried.insert(dev.endpoint.clone()) {
                continue;
            }
            let reachable = dev
                .endpoint
                .parse::<SocketAddr>()
                .ok()
                .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(450)).ok())
                .is_some();
            if !reachable {
                continue;
            }
            if let Ok(out) = run_adb(&adb, &["connect", &dev.endpoint], 5) {
                let low = out.to_lowercase();
                if low.contains("connected to") && !low.contains("cannot connect") {
                    let name = device_display_name(&adb, &dev.endpoint);
                    results.push((dev.endpoint, name));
                }
            }
        }
        results
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut endpoints = Vec::new();
    for (ep, name) in connected {
        save_wifi_device(&state, &ep, &name);
        record_timeline(
            &state,
            &app,
            &ep,
            &name,
            "connect",
            "Auto-connected over Wi-Fi",
            &ep,
        );
        endpoints.push(ep);
    }
    Ok(endpoints)
}

#[tauri::command]
pub async fn mob_wifi_qr_stop(state: State<'_, AppState>) -> CmdResult<()> {
    if let Ok(mut g) = state.mobile.qr_cancel.lock() {
        if let Some(flag) = g.take() {
            flag.store(true, Ordering::SeqCst);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn mob_wifi_qr_start(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<WifiQrSession> {
    let adb = locate_adb(&state)?;
    // Cancel any previous QR listener.
    if let Ok(mut g) = state.mobile.qr_cancel.lock() {
        if let Some(old) = g.take() {
            old.store(true, Ordering::SeqCst);
        }
    }
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut g) = state.mobile.qr_cancel.lock() {
        *g = Some(cancel.clone());
    }

    let (service, password) = generate_qr_credentials();
    let payload = build_wifi_qr_payload(&service, &password);
    let qr_svg = render_qr_svg(&payload)?;

    let svc_watch = service.clone();
    let pass_watch = password.clone();
    let app_watch = app.clone();

    std::thread::spawn(move || {
        // Ensure the ADB server is up so mDNS discovery is active.
        let _ = run_adb(&adb, &["start-server"], 6);
        let started = Instant::now();
        let mut paired_ip: Option<String> = None;

        while !cancel.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(120) {
            if let Ok(raw) = run_adb(&adb, &["mdns", "services"], 4) {
                let services = parse_mdns_services(&raw);
                if paired_ip.is_none() {
                    if let Some(target) = services
                        .iter()
                        .find(|s| s.kind == "pair" && s.name.contains(&svc_watch))
                    {
                        let _ = app_watch.emit(
                            "mobile://wifi-pair",
                            serde_json::json!({
                                "status": "pairing",
                                "endpoint": target.endpoint,
                                "message": format!("Phone scanned QR ({}) — pairing…", target.endpoint),
                            }),
                        );
                        match run_adb(&adb, &["pair", &target.endpoint, &pass_watch], 15) {
                            Ok(out) if !out.to_lowercase().contains("failed") => {
                                paired_ip = Some(target.ip.clone());
                                let _ = app_watch.emit(
                                    "mobile://wifi-pair",
                                    serde_json::json!({
                                        "status": "paired",
                                        "endpoint": target.endpoint,
                                        "message": "Paired! Waiting for wireless debug port…",
                                    }),
                                );
                            }
                            Ok(out) => {
                                let _ = app_watch.emit(
                                    "mobile://wifi-pair",
                                    serde_json::json!({
                                        "status": "error",
                                        "endpoint": target.endpoint,
                                        "message": out.trim(),
                                    }),
                                );
                                return;
                            }
                            Err(e) => {
                                let _ = app_watch.emit(
                                    "mobile://wifi-pair",
                                    serde_json::json!({
                                        "status": "error",
                                        "endpoint": target.endpoint,
                                        "message": e,
                                    }),
                                );
                                return;
                            }
                        }
                    }
                } else if let Some(ref ip) = paired_ip {
                    if let Some(conn) = services
                        .iter()
                        .find(|s| s.kind == "connect" && s.ip == *ip)
                    {
                        if let Ok(out) = run_adb(&adb, &["connect", &conn.endpoint], 10) {
                            let low = out.to_lowercase();
                            if low.contains("connected to") && !low.contains("cannot connect") {
                                let name = device_display_name(&adb, &conn.endpoint);
                                if let Some(st) = app_watch.try_state::<AppState>() {
                                    save_wifi_device(&st, &conn.endpoint, &name);
                                    record_timeline(
                                        &st,
                                        &app_watch,
                                        &conn.endpoint,
                                        &name,
                                        "pair",
                                        "Paired & connected via QR",
                                        &conn.endpoint,
                                    );
                                }
                                let _ = app_watch.emit(
                                    "mobile://wifi-pair",
                                    serde_json::json!({
                                        "status": "connected",
                                        "endpoint": conn.endpoint,
                                        "name": name,
                                        "message": format!("Connected to {name} over Wi-Fi!"),
                                    }),
                                );
                                return;
                            }
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(1100));
        }

        if !cancel.load(Ordering::Relaxed) {
            let _ = app_watch.emit(
                "mobile://wifi-pair",
                serde_json::json!({
                    "status": "timeout",
                    "message": "QR code expired — generate a fresh code to try again.",
                }),
            );
        }
    });

    Ok(WifiQrSession {
        service,
        password,
        payload,
        qr_svg,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_adb_devices_usb_and_wifi() {
        let sample = "List of devices attached\n\
R5CX1234ABC            device product:a35x model:SM_A356E device:a35x transport_id:1\n\
192.168.1.45:5555      offline product:a35x model:SM_A356E\n";
        let list = parse_devices_output(sample);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].serial, "R5CX1234ABC");
        assert_eq!(list[0].transport, "usb");
        assert_eq!(list[0].model, "SM A356E");
        assert_eq!(list[1].transport, "wifi");
        assert_eq!(list[1].wifi_ip.as_deref(), Some("192.168.1.45"));
    }

    #[test]
    fn parses_battery_dump() {
        let sample = "Current Battery Service state:\n  AC powered: true\n  USB powered: false\n  Wireless powered: false\n  status: 2\n  level: 87\n  temperature: 315\n";
        let b = parse_battery(sample);
        assert_eq!(b.level, 87);
        assert!(b.charging);
        assert_eq!(b.charge_type, "Charging");
        assert!((b.temp_c - 31.5).abs() < 0.01);
    }

    #[test]
    fn parses_media_session_dump() {
        let sample = "Sessions Stack - have 1 sessions:\n\
  package=com.spotify.music\n\
    active=true\n\
    state=PlaybackState {state=3, position=215000, buffered position=0}\n\
    metadata: size=4, description=Intro (Infected), Sickick, null\n\
    android.media.metadata.DURATION=293000\n";
        let m = parse_media_session(sample).expect("media session");
        assert_eq!(m.app, "Spotify");
        assert_eq!(m.title, "Intro (Infected)");
        assert_eq!(m.artist, "Sickick");
        assert!(m.playing);
        assert_eq!(m.position_ms, 215_000);
        assert_eq!(m.duration_ms, 293_000);
    }

    #[test]
    fn parses_notifications_and_filters_muted() {
        let sample = "\
NotificationRecord(0x1 pkg=com.whatsapp user=UserHandle{0} id=1 tag=null key=0|com.whatsapp|1|null|10200)\n\
  postTime=1795400000000\n\
  android.title=String (Instinct)\n\
  android.text=String (Hey there)\n\
  RemoteInput present\n\
NotificationRecord(0x2 pkg=android user=UserHandle{0} id=2 tag=null key=0|android|2|null|1000)\n\
  android.title=String (USB debugging connected)\n";
        let list = parse_notifications(sample, &HashSet::new(), &HashSet::new());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].app_name, "WhatsApp");
        assert_eq!(list[0].title, "Instinct");
        assert_eq!(list[0].text, "Hey there");
        assert!(list[0].can_reply);
    }

    #[test]
    fn escapes_adb_text_safely() {
        assert_eq!(escape_adb_input_text("hello world!"), "hello%sworld\\!");
        assert_eq!(escape_adb_input_text("a&b;c"), "a\\&b\\;c");
    }

    #[test]
    fn validates_sdcard_paths_and_parsers() {
        assert_eq!(clean_sdcard_path("/sdcard/Download/").unwrap(), "/sdcard/Download");
        assert!(clean_sdcard_path("/sdcard/../etc").is_err());
        assert!(clean_sdcard_path("/system/bin").is_err());
        let (tot, used, avail) = parse_df_kb("Filesystem 1K-blocks Used Available Use% Mounted\n/dev/block 120000 80000 40000 67% /data");
        assert_eq!((tot, used, avail), (120000, 80000, 40000));
        let (mt, ma) = parse_meminfo_kb("MemTotal:        8192000 kB\nMemFree:         1000000 kB\nMemAvailable:    3200000 kB\n");
        assert_eq!((mt, ma), (8192000, 3200000));
    }

    #[test]
    fn parses_scrcpy_cameras_displays_and_vcard() {
        let cam_raw = "\
[server] INFO: List of camera sizes:\n\
    --camera-id=0    (back, 4000x3000, fps=[15, 24, 30, 60])\n\
        - 3840x2160\n\
        - 1920x1080\n\
      High speed capture:\n\
        - 1920x1080 (fps=[120])\n\
    --camera-id=1    (front, 3264x2448, fps=[15, 30])\n\
        - 1920x1080\n";
        let cams = parse_scrcpy_cameras(cam_raw);
        assert_eq!(cams.len(), 2);
        assert_eq!(cams[0].id, "0");
        assert_eq!(cams[0].facing, "back");
        assert_eq!(cams[0].sizes, vec!["3840x2160", "1920x1080"]);
        assert_eq!(cams[0].fps, vec![15, 24, 30, 60]);
        assert_eq!(cams[0].high_speed, vec!["1920x1080 (fps=[120])"]);
        assert_eq!(cams[1].facing, "front");

        let disp_raw = "    --display-id=0    (1080x2340)\n    --display-id=2    (1920x1080)\n";
        let disps = parse_scrcpy_displays(disp_raw);
        assert_eq!(disps.len(), 2);
        assert_eq!(disps[1].id, 2);

        let vcf = build_vcard(&[PhoneContact { name: "Ada".into(), number: "+5511999".into() }]);
        assert!(vcf.contains("FN:Ada") && vcf.contains("TEL;TYPE=CELL:+5511999"));
    }

    #[test]
    fn parses_mdns_services_and_builds_adb_qr() {
        let mdns = "\
List of discovered mdns services\n\
adb-R5CX1234ABC-xYz123\t_adb-tls-connect._tcp.\t192.168.1.45:41235\n\
ADB_WIFI_nomnom_a1b2c3d4\t_adb-tls-pairing._tcp\t192.168.1.45:37123\n";
        let svcs = parse_mdns_services(mdns);
        assert_eq!(svcs.len(), 2);
        assert_eq!(svcs[0].kind, "connect");
        assert_eq!(svcs[0].ip, "192.168.1.45");
        assert_eq!(svcs[0].port, 41235);
        assert_eq!(svcs[1].kind, "pair");
        assert_eq!(svcs[1].name, "ADB_WIFI_nomnom_a1b2c3d4");

        let payload = build_wifi_qr_payload("ADB_WIFI_nomnom_1234", "abcd5678");
        assert_eq!(payload, "WIFI:T:ADB;S:ADB_WIFI_nomnom_1234;P:abcd5678;;");
        let svg = render_qr_svg(&payload).expect("qr svg");
        assert!(svg.contains("<svg") && svg.contains("</svg>"));
    }
}
