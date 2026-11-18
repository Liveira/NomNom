//! The Linux side of Playtime on Wayland, over the session D-Bus.
//!
//! - **KDE Plasma**: KWin keeps the focused window to itself, so NomNom loads a tiny KWin script
//!   (`wayland::KWIN_SCRIPT`) that reports the focused window to a service NomNom offers on the bus.
//! - **Idle time** for the desktops that expose it (KDE's screen saver service, GNOME's Mutter).
//!
//! Everything here fails quietly: if a service isn't there, the caller falls back to the next way.

use crate::wayland::{parse_report, Desktop, Focus, KWIN_SCRIPT};
use std::sync::{Arc, Mutex};
use zbus::blocking::{Connection, Proxy};

const NAME: &str = "com.nomnom.ActiveWindow";
const SCRIPT: &str = "nomnom-focus";

type Shared = Arc<Mutex<Option<Focus>>>;

struct Service {
    state: Shared,
}

#[zbus::interface(name = "com.nomnom.ActiveWindow")]
impl Service {
    /// Called by the KWin script with one line (see `wayland::parse_report`).
    fn update(&self, report: &str) {
        if let Some(focus) = parse_report(report) {
            *self.state.lock().unwrap() = Some(focus);
        }
    }
}

pub struct Link {
    conn: Connection,
    desktop: Desktop,
    state: Option<Shared>,
}

impl Link {
    /// Connects to the session bus. On KDE it also starts the KWin script.
    pub fn open(desktop: Desktop, wayland_session: bool) -> Option<Link> {
        if desktop == Desktop::Kde && wayland_session {
            if let Some(link) = Self::open_kde() {
                return Some(link);
            }
        }
        Some(Link {
            conn: Connection::session().ok()?,
            desktop,
            state: None,
        })
    }

    fn open_kde() -> Option<Link> {
        let state: Shared = Arc::new(Mutex::new(None));
        let conn = zbus::blocking::connection::Builder::session()
            .ok()?
            .name(NAME)
            .ok()?
            .serve_at(
                "/ActiveWindow",
                Service {
                    state: state.clone(),
                },
            )
            .ok()?
            .build()
            .ok()?;

        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .filter(|p| p.is_dir())
            .unwrap_or_else(std::env::temp_dir);
        let path = dir.join("nomnom-focus.js");
        std::fs::write(&path, KWIN_SCRIPT).ok()?;

        let scripting = Proxy::new(
            &conn,
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting",
        )
        .ok()?;
        // A script left over from an earlier run of NomNom must not run twice.
        let _ = scripting.call_method("unloadScript", &(SCRIPT,));
        let id: i32 = scripting
            .call("loadScript", &(path.to_str()?, SCRIPT))
            .ok()?;
        if id < 0 {
            return None;
        }
        // Plasma 6 and recent Plasma 5 put the script at /Scripting/Script<id>; older ones at /<id>.
        let started = [format!("/Scripting/Script{id}"), format!("/{id}")]
            .iter()
            .any(|object| {
                Proxy::new(
                    &conn,
                    "org.kde.KWin",
                    object.as_str(),
                    "org.kde.kwin.Script",
                )
                .ok()
                .and_then(|script| script.call_method("run", &()).ok())
                .is_some()
            });
        if !started {
            return None;
        }
        Some(Link {
            conn,
            desktop: Desktop::Kde,
            state: Some(state),
        })
    }

    /// The focused window as the KWin script last reported it. `None` while it hasn't reported yet,
    /// or when there is no script (not KDE).
    pub fn focus(&self) -> Option<Focus> {
        self.state.as_ref()?.lock().unwrap().clone()
    }

    /// Seconds since the last key or mouse input, from the desktop's own idle service.
    pub fn idle_secs(&self) -> Option<u32> {
        match self.desktop {
            Desktop::Kde => {
                let p = Proxy::new(
                    &self.conn,
                    "org.freedesktop.ScreenSaver",
                    "/ScreenSaver",
                    "org.freedesktop.ScreenSaver",
                )
                .ok()?;
                let v: u32 = p.call("GetSessionIdleTime", &()).ok()?;
                // Seconds. A value of more than a week can only be milliseconds.
                Some(if v > 7 * 86_400 { v / 1000 } else { v })
            }
            Desktop::Gnome => {
                let p = Proxy::new(
                    &self.conn,
                    "org.gnome.Mutter.IdleMonitor",
                    "/org/gnome/Mutter/IdleMonitor/Core",
                    "org.gnome.Mutter.IdleMonitor",
                )
                .ok()?;
                let ms: u64 = p.call("GetIdletime", &()).ok()?;
                Some((ms / 1000).min(u32::MAX as u64) as u32)
            }
            Desktop::Other => None,
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        if self.state.is_some() {
            if let Ok(p) = Proxy::new(
                &self.conn,
                "org.kde.KWin",
                "/Scripting",
                "org.kde.kwin.Scripting",
            ) {
                let _ = p.call_method("unloadScript", &(SCRIPT,));
            }
        }
    }
}
