//! Cross-platform foreground window and user-activity probe.
//!
//! - **Windows**: Uses `OpenInputDesktop` (lockscreen check), `GetForegroundWindow`,
//!   `EnumChildWindows` (to unwrap UWP `ApplicationFrameHost.exe` into the real app),
//!   `QueryFullProcessImageNameW`, `GetLastInputInfo`, `XInputGetState` (controller input), and
//!   `SHQueryUserNotificationState` (`QUNS_RUNNING_D3D_FULL_SCREEN`).
//! - **Linux**: Supports Hyprland (`HYPRLAND_INSTANCE_SIGNATURE` IPC socket), Sway/i3
//!   (`SWAYSOCK`/`I3SOCK` IPC socket), and X11/XWayland (`x11rb` `_NET_ACTIVE_WINDOW` +
//!   `XScreenSaver`), resolves `/proc/<pid>/exe` (unwrapping Wine/Proton `.exe` games on Arch &
//!   SteamOS), checks lockscreens (`hyprlock`, `swaylock`, `i3lock`, `kscreenlocker_greet`), and
//!   looks up `.desktop` metadata.

use crate::classify::RawForeground;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForegroundSample {
    Active {
        raw: RawForeground,
        idle_secs: u32,
        fullscreen_or_gamepad: bool,
    },
    Idle {
        idle_secs: u32,
    },
    LockedOrNone,
}

pub struct ForegroundProbe {
    #[cfg(windows)]
    last_xinput_packets: [u32; 4],
    #[cfg(windows)]
    last_gamepad_change_tick: u32,
    #[cfg(target_os = "linux")]
    linux: LinuxState,
}

impl Default for ForegroundProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl ForegroundProbe {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            last_xinput_packets: [0; 4],
            #[cfg(windows)]
            last_gamepad_change_tick: 0,
            #[cfg(target_os = "linux")]
            linux: LinuxState::default(),
        }
    }

    pub fn sample(&mut self) -> ForegroundSample {
        #[cfg(windows)]
        {
            self.sample_windows()
        }
        #[cfg(target_os = "linux")]
        {
            self.sample_linux()
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            ForegroundSample::LockedOrNone
        }
    }
}

// ============================================================================
// Windows implementation
// ============================================================================
#[cfg(windows)]
mod win_impl {
    use super::*;
    use std::path::Path;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_READOBJECTS,
    };
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible,
    };

    struct ChildLookup {
        host_pid: u32,
        real_pid: u32,
    }

    unsafe extern "system" fn find_uwp_child_pid(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut ChildLookup);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 && pid != state.host_pid {
            state.real_pid = pid;
            return BOOL(0);
        }
        BOOL(1)
    }

    fn is_workstation_unlocked() -> bool {
        unsafe {
            match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) {
                Ok(desk) => {
                    let _ = CloseDesktop(desk);
                    true
                }
                Err(_) => false,
            }
        }
    }

    fn process_image_path(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = vec![0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok();
            let _ = CloseHandle(handle);
            if !ok || len == 0 {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    fn window_class_name(hwnd: HWND) -> String {
        unsafe {
            let mut buf = [0u16; 128];
            let n = GetClassNameW(hwnd, &mut buf);
            if n <= 0 {
                String::new()
            } else {
                String::from_utf16_lossy(&buf[..n as usize])
            }
        }
    }

    impl ForegroundProbe {
        pub(super) fn sample_windows(&mut self) -> ForegroundSample {
            if !is_workstation_unlocked() {
                return ForegroundSample::LockedOrNone;
            }
            unsafe {
                let hwnd = GetForegroundWindow();
                if hwnd.0.is_null() || !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool()
                {
                    return ForegroundSample::LockedOrNone;
                }

                let class = window_class_name(hwnd);
                if matches!(
                    class.as_str(),
                    "Progman"
                        | "WorkerW"
                        | "Shell_TrayWnd"
                        | "Shell_SecondaryTrayWnd"
                        | "MultitaskingViewFrame"
                ) {
                    return ForegroundSample::LockedOrNone;
                }

                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if pid == 0 {
                    return ForegroundSample::LockedOrNone;
                }

                let mut exe = process_image_path(pid);
                if exe
                    .as_deref()
                    .and_then(|p| Path::new(p).file_name())
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("ApplicationFrameHost.exe"))
                {
                    let mut lookup = ChildLookup {
                        host_pid: pid,
                        real_pid: 0,
                    };
                    let _ = EnumChildWindows(
                        hwnd,
                        Some(find_uwp_child_pid),
                        LPARAM(&mut lookup as *mut ChildLookup as isize),
                    );
                    if lookup.real_pid != 0 {
                        pid = lookup.real_pid;
                        exe = process_image_path(pid);
                    }
                }

                // Idle check: keyboard/mouse via GetLastInputInfo + gamepad via XInput packet deltas.
                let now_tick = GetTickCount();
                let mut lii = LASTINPUTINFO {
                    cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                    dwTime: 0,
                };
                let kb_mouse_idle_ms = if GetLastInputInfo(&mut lii).as_bool() {
                    now_tick.wrapping_sub(lii.dwTime)
                } else {
                    0
                };

                let mut gamepad_active = false;
                for slot in 0..4u32 {
                    let mut st = XINPUT_STATE::default();
                    if XInputGetState(slot, &mut st) == 0 {
                        let prev = self.last_xinput_packets[slot as usize];
                        if st.dwPacketNumber != prev {
                            self.last_xinput_packets[slot as usize] = st.dwPacketNumber;
                            if prev != 0 {
                                self.last_gamepad_change_tick = now_tick;
                                gamepad_active = true;
                            }
                        }
                    }
                }

                let gamepad_idle_ms = if self.last_gamepad_change_tick != 0 {
                    now_tick.wrapping_sub(self.last_gamepad_change_tick)
                } else {
                    u32::MAX
                };
                let idle_secs = kb_mouse_idle_ms.min(gamepad_idle_ms) / 1000;

                let fullscreen_d3d =
                    SHQueryUserNotificationState().is_ok_and(|s| s == QUNS_RUNNING_D3D_FULL_SCREEN);

                ForegroundSample::Active {
                    raw: RawForeground {
                        pid,
                        exe_path: exe,
                        wm_class: (!class.is_empty()).then_some(class),
                        desktop_name: None,
                    },
                    idle_secs,
                    fullscreen_or_gamepad: gamepad_active || (fullscreen_d3d && idle_secs < 900),
                }
            }
        }
    }
}

// ============================================================================
// Linux implementation (Hyprland, Sway/i3, X11/XWayland, Wine/Proton unwrapping)
// ============================================================================
#[cfg(target_os = "linux")]
#[derive(Default)]
struct LinuxState {
    x11: Option<X11Handle>,
    /// Which way the focused window was found last time, so the log says when that changes.
    source: &'static str,
}

#[cfg(target_os = "linux")]
struct X11Handle {
    conn: x11rb::rust_connection::RustConnection,
    root: u32,
    atom_active_window: u32,
    atom_wm_pid: u32,
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    use crate::icons::extract_desktop_name;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use x11rb::connection::Connection;
    use x11rb::protocol::screensaver::ConnectionExt as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};

    const LOCK_PROCESSES: &[&str] = &[
        "hyprlock",
        "swaylock",
        "i3lock",
        "gtklock",
        "waylock",
        "kscreenlocker_greet",
    ];

    fn is_screen_locked() -> bool {
        let my_uid = unsafe { libc::geteuid() };
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return false;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(s) = name.to_str() else { continue };
            if !s.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let proc_dir = entry.path();
            if let Ok(meta) = std::fs::metadata(&proc_dir) {
                use std::os::unix::fs::MetadataExt;
                if meta.uid() != my_uid {
                    continue;
                }
            }
            if let Ok(comm) = std::fs::read_to_string(proc_dir.join("comm")) {
                let c = comm.trim();
                if LOCK_PROCESSES.contains(&c) {
                    return true;
                }
            }
        }
        false
    }

    fn resolve_proc_exe(pid: u32) -> Option<String> {
        if pid == 0 {
            return None;
        }
        let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
            .ok()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                s.strip_suffix(" (deleted)").unwrap_or(&s).to_string()
            });

        // Unwrap Wine / Proton (`wine-preloader`, `wine64-preloader`) so Steam/Lutris Windows
        // games on Arch Linux report their real game `.exe` inside `steamapps/common/...`!
        let is_wine = exe
            .as_deref()
            .and_then(|p| Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .is_some_and(|n| {
                matches!(
                    n,
                    "wine-preloader" | "wine64-preloader" | "wine" | "wine64" | "reaper"
                )
            });

        if is_wine {
            if let Ok(cmdline) = std::fs::read(format!("/proc/{pid}/cmdline")) {
                for arg_bytes in cmdline.split(|&b| b == 0) {
                    let arg = String::from_utf8_lossy(arg_bytes);
                    let lower = arg.to_ascii_lowercase();
                    if lower.ends_with(".exe")
                        && !lower.ends_with("steam.exe")
                        && !lower.ends_with("services.exe")
                        && !lower.ends_with(" explorer.exe")
                        && !lower.ends_with("\\explorer.exe")
                        && !lower.ends_with("plugplay.exe")
                        && !lower.ends_with("svchost.exe")
                        && !lower.ends_with("rpcss.exe")
                    {
                        let cleaned = arg
                            .strip_prefix("Z:")
                            .or_else(|| arg.strip_prefix("z:"))
                            .unwrap_or(&arg)
                            .replace('\\', "/");
                        return Some(cleaned);
                    }
                }
            }
        }

        exe
    }

    fn hyprland_socket_path() -> Option<PathBuf> {
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
        if sig.is_empty() || sig.contains("..") {
            return None;
        }
        if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
            let p = PathBuf::from(rt)
                .join("hypr")
                .join(&sig)
                .join(".socket.sock");
            if p.exists() {
                return Some(p);
            }
        }
        let fallback = PathBuf::from("/tmp/hypr").join(sig).join(".socket.sock");
        fallback.exists().then_some(fallback)
    }

    fn query_hyprland() -> Option<(u32, Option<String>, bool)> {
        let sock = hyprland_socket_path()?;
        let mut stream = UnixStream::connect(sock).ok()?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(120)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(120)));
        stream.write_all(b"j/activewindow").ok()?;
        let mut resp = String::new();
        stream.read_to_string(&mut resp).ok()?;
        let v: serde_json::Value = serde_json::from_str(&resp).ok()?;
        let pid = v.get("pid").and_then(|p| p.as_i64()).unwrap_or(0);
        if pid <= 0 {
            return None;
        }
        let class = v
            .get("class")
            .and_then(|c| c.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                v.get("initialClass")
                    .and_then(|c| c.as_str())
                    .filter(|s| !s.is_empty())
            })
            .map(str::to_string);
        let fullscreen = v
            .get("fullscreen")
            .and_then(|f| f.as_bool().or_else(|| f.as_i64().map(|n| n > 0)))
            .unwrap_or(false);
        Some((pid as u32, class, fullscreen))
    }

    fn query_sway_or_i3() -> Option<(u32, Option<String>, bool)> {
        let sock = std::env::var_os("SWAYSOCK")
            .or_else(|| std::env::var_os("I3SOCK"))
            .map(PathBuf::from)?;
        let mut stream = UnixStream::connect(sock).ok()?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(150)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(150)));
        // i3-ipc header: b"i3-ipc" + payload_len: u32 LE (0) + msg_type: u32 LE (4 = GET_TREE)
        let mut hdr = [0u8; 14];
        hdr[..6].copy_from_slice(b"i3-ipc");
        hdr[10..14].copy_from_slice(&4u32.to_le_bytes());
        stream.write_all(&hdr).ok()?;

        let mut resp_hdr = [0u8; 14];
        stream.read_exact(&mut resp_hdr).ok()?;
        if &resp_hdr[..6] != b"i3-ipc" {
            return None;
        }
        let len = u32::from_le_bytes([resp_hdr[6], resp_hdr[7], resp_hdr[8], resp_hdr[9]]) as usize;
        if len == 0 || len > 8 * 1024 * 1024 {
            return None;
        }
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).ok()?;
        let tree: serde_json::Value = serde_json::from_slice(&payload).ok()?;
        find_focused_sway_node(&tree)
    }

    fn find_focused_sway_node(node: &serde_json::Value) -> Option<(u32, Option<String>, bool)> {
        if node.get("focused").and_then(|b| b.as_bool()) == Some(true) {
            let pid = node.get("pid").and_then(|p| p.as_u64()).unwrap_or(0) as u32;
            if pid > 0 {
                let class = node
                    .get("app_id")
                    .and_then(|s| s.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        node.get("window_properties")
                            .and_then(|w| w.get("class"))
                            .and_then(|s| s.as_str())
                            .filter(|s| !s.is_empty())
                    })
                    .map(str::to_string);
                let fullscreen = node
                    .get("fullscreen_mode")
                    .and_then(|n| n.as_i64())
                    .unwrap_or(0)
                    > 0;
                return Some((pid, class, fullscreen));
            }
        }
        for key in ["nodes", "floating_nodes"] {
            if let Some(arr) = node.get(key).and_then(|a| a.as_array()) {
                for child in arr {
                    if let Some(found) = find_focused_sway_node(child) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    fn connect_x11() -> Option<X11Handle> {
        let (conn, screen_num) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen_num)?.root;
        let atom_active_window = conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let atom_wm_pid = conn
            .intern_atom(false, b"_NET_WM_PID")
            .ok()?
            .reply()
            .ok()?
            .atom;
        Some(X11Handle {
            conn,
            root,
            atom_active_window,
            atom_wm_pid,
        })
    }

    fn query_x11(h: &X11Handle) -> Result<Option<(u32, Option<String>, u32)>, ()> {
        let win_reply = h
            .conn
            .get_property(false, h.root, h.atom_active_window, AtomEnum::WINDOW, 0, 1)
            .map_err(|_| ())?
            .reply()
            .map_err(|_| ())?;
        let Some(win) = win_reply.value32().and_then(|mut it| it.next()) else {
            return Ok(None);
        };
        if win == 0 {
            return Ok(None);
        }
        let pid = h
            .conn
            .get_property(false, win, h.atom_wm_pid, AtomEnum::CARDINAL, 0, 1)
            .map_err(|_| ())?
            .reply()
            .ok()
            .and_then(|r| r.value32().and_then(|mut it| it.next()))
            .unwrap_or(0);
        if pid == 0 {
            return Ok(None);
        }

        let wm_class = h
            .conn
            .get_property(false, win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| {
                let parts: Vec<&[u8]> = r
                    .value
                    .split(|&b| b == 0)
                    .filter(|s| !s.is_empty())
                    .collect();
                parts
                    .get(1)
                    .or_else(|| parts.first())
                    .map(|b| String::from_utf8_lossy(b).to_string())
            });

        let idle_secs = h
            .conn
            .screensaver_query_info(h.root)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.ms_since_user_input / 1000)
            .unwrap_or(0);

        Ok(Some((pid, wm_class, idle_secs)))
    }

    impl ForegroundProbe {
        /// Writes one line to the log each time the way of finding the focused window changes. When
        /// nothing works (a Wayland desktop that doesn't share it), the line says which desktop it
        /// is, so the cause can be seen without guessing.
        fn note_source(&mut self, source: &'static str) {
            if self.linux.source == source {
                return;
            }
            self.linux.source = source;
            let var = |k: &str| std::env::var(k).unwrap_or_else(|_| "unset".into());
            if source == "none" {
                tracing::warn!(
                    "Playtime can't see the focused window: session={} desktop={} DISPLAY={} WAYLAND_DISPLAY={}",
                    var("XDG_SESSION_TYPE"),
                    var("XDG_CURRENT_DESKTOP"),
                    var("DISPLAY"),
                    var("WAYLAND_DISPLAY")
                );
            } else {
                tracing::info!(
                    "Playtime reads the focused window through {source} (session={} desktop={})",
                    var("XDG_SESSION_TYPE"),
                    var("XDG_CURRENT_DESKTOP")
                );
            }
        }

        pub(super) fn sample_linux(&mut self) -> ForegroundSample {
            if is_screen_locked() {
                return ForegroundSample::LockedOrNone;
            }

            // 1. Hyprland native IPC socket (popular on Arch Linux)
            if let Some((pid, wm_class, fullscreen)) = query_hyprland() {
                self.note_source("Hyprland");
                let exe_path = resolve_proc_exe(pid);
                let desktop_name = extract_desktop_name(exe_path.as_deref(), wm_class.as_deref());
                let idle_secs = self.query_x11_idle().unwrap_or(0);
                return ForegroundSample::Active {
                    raw: RawForeground {
                        pid,
                        exe_path,
                        wm_class,
                        desktop_name,
                    },
                    idle_secs,
                    fullscreen_or_gamepad: fullscreen,
                };
            }

            // 2. Sway / i3 native IPC socket
            if let Some((pid, wm_class, fullscreen)) = query_sway_or_i3() {
                self.note_source("Sway or i3");
                let exe_path = resolve_proc_exe(pid);
                let desktop_name = extract_desktop_name(exe_path.as_deref(), wm_class.as_deref());
                let idle_secs = self.query_x11_idle().unwrap_or(0);
                return ForegroundSample::Active {
                    raw: RawForeground {
                        pid,
                        exe_path,
                        wm_class,
                        desktop_name,
                    },
                    idle_secs,
                    fullscreen_or_gamepad: fullscreen,
                };
            }

            // 3. X11 & XWayland (`_NET_ACTIVE_WINDOW` + `XScreenSaver`)
            if self.linux.x11.is_none() && std::env::var_os("DISPLAY").is_some() {
                self.linux.x11 = connect_x11();
            }
            let x11_answer = self.linux.x11.as_ref().map(query_x11);
            if let Some(answer) = x11_answer {
                match answer {
                    Ok(Some((pid, wm_class, idle_secs))) => {
                        self.note_source("X11");
                        let exe_path = resolve_proc_exe(pid);
                        let desktop_name =
                            extract_desktop_name(exe_path.as_deref(), wm_class.as_deref());
                        return ForegroundSample::Active {
                            raw: RawForeground {
                                pid,
                                exe_path,
                                wm_class,
                                desktop_name,
                            },
                            idle_secs,
                            fullscreen_or_gamepad: false,
                        };
                    }
                    Ok(None) => {}
                    Err(()) => {
                        self.linux.x11 = None;
                    }
                }
            }

            if self.linux.source.is_empty() || self.linux.source == "none" {
                self.note_source("none");
            }
            ForegroundSample::LockedOrNone
        }

        fn query_x11_idle(&mut self) -> Option<u32> {
            if self.linux.x11.is_none() && std::env::var_os("DISPLAY").is_some() {
                self.linux.x11 = connect_x11();
            }
            let h = self.linux.x11.as_ref()?;
            h.conn
                .screensaver_query_info(h.root)
                .ok()?
                .reply()
                .ok()
                .map(|r| r.ms_since_user_input / 1000)
        }
    }
}
