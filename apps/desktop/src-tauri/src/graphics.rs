#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
//! How the web view draws on Linux (WebKitGTK), chosen before the window exists.
//!
//! WebKitGTK can hand each frame to the graphics card without copying it (DMABUF), or fall back to
//! copying every frame through the processor. The fallback is what makes an app run at half the
//! speed and keep the fans busy, so it is used only where the fast way is known to show a blank
//! window: the NVIDIA driver on Wayland without `nvidia-drm.modeset=1`.

/// What the computer looks like to the web view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Machine {
    pub wayland: bool,
    pub nvidia: bool,
    /// The NVIDIA driver's kernel mode setting is on (the default on current Arch and CachyOS).
    pub nvidia_modeset: bool,
}

pub const DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
pub const COMPOSITING: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";

/// The environment variables to set for the setting `choice`:
/// - `auto`: the fast way, except where it is known to fail.
/// - `gpu`: always the fast way.
/// - `cpu`: no graphics card at all (slowest to scroll, but nothing wakes the card).
pub fn webkit_env(choice: &str, m: &Machine) -> Vec<(&'static str, &'static str)> {
    match choice {
        "cpu" => vec![(DMABUF, "1"), (COMPOSITING, "1")],
        "gpu" => Vec::new(),
        _ if m.wayland && m.nvidia && !m.nvidia_modeset => vec![(DMABUF, "1")],
        _ => Vec::new(),
    }
}

#[cfg(target_os = "linux")]
pub fn machine() -> Machine {
    use std::path::Path;
    let modeset = std::fs::read_to_string("/sys/module/nvidia_drm/parameters/modeset")
        .map(|s| s.trim().eq_ignore_ascii_case("y"))
        .unwrap_or(false);
    Machine {
        wayland: std::env::var_os("WAYLAND_DISPLAY").is_some(),
        nvidia: Path::new("/dev/nvidia0").exists()
            || Path::new("/proc/driver/nvidia/version").exists(),
        nvidia_modeset: modeset,
    }
}

/// Sets the variables the web view reads, unless the person already set them in their own
/// environment (that always wins).
#[cfg(target_os = "linux")]
pub fn apply(choice: &str) {
    let m = machine();
    let vars = webkit_env(choice, &m);
    for (key, value) in &vars {
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    }
    tracing::info!(choice, ?m, ?vars, "web view graphics");
}

#[cfg(test)]
mod tests {
    use super::*;

    const NVIDIA_NO_MODESET: Machine = Machine {
        wayland: true,
        nvidia: true,
        nvidia_modeset: false,
    };

    #[test]
    fn auto_keeps_the_fast_way_wherever_it_works() {
        let amd = Machine {
            wayland: true,
            ..Machine::default()
        };
        assert!(webkit_env("auto", &amd).is_empty());
        let nvidia_ok = Machine {
            wayland: true,
            nvidia: true,
            nvidia_modeset: true,
        };
        assert!(
            webkit_env("auto", &nvidia_ok).is_empty(),
            "modeset is on: no blank window"
        );
        let x11 = Machine {
            nvidia: true,
            ..Machine::default()
        };
        assert!(webkit_env("auto", &x11).is_empty());
    }

    #[test]
    fn auto_uses_the_safe_way_only_where_the_window_would_be_blank() {
        assert_eq!(webkit_env("auto", &NVIDIA_NO_MODESET), vec![(DMABUF, "1")]);
        assert_eq!(
            webkit_env("anything else", &NVIDIA_NO_MODESET),
            vec![(DMABUF, "1")]
        );
    }

    #[test]
    fn the_person_can_force_either_way() {
        assert!(webkit_env("gpu", &NVIDIA_NO_MODESET).is_empty());
        let cpu = webkit_env("cpu", &Machine::default());
        assert!(cpu.contains(&(DMABUF, "1")) && cpu.contains(&(COMPOSITING, "1")));
    }
}
