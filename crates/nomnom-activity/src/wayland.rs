//! Playtime on Wayland desktops that don't share the focused window over a socket (KDE Plasma,
//! GNOME and others). The parts that don't touch the system live here so they can be tested on any
//! computer: the small KWin script and its report format, how to tell the desktop, and how to tell
//! a game's program from the launcher programs around it.

/// What the KWin script tells NomNom each time the focused window changes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Focus {
    pub pid: u32,
    pub class: String,
    pub fullscreen: bool,
}

/// The report is one text line, so it doesn't depend on how the script engine types numbers:
/// `pid TAB fullscreen(0/1) TAB window class`. The window title is never sent.
pub fn parse_report(report: &str) -> Option<Focus> {
    let mut parts = report.splitn(3, '\t');
    let pid = parts.next()?.trim().parse::<u32>().ok()?;
    let fullscreen = parts.next()? == "1";
    let class = parts.next().unwrap_or("").trim().to_string();
    Some(Focus {
        pid,
        class,
        fullscreen,
    })
}

/// Loaded into KWin (Plasma 5 and 6). It reports the focused window now and on every change.
pub const KWIN_SCRIPT: &str = r#"
function report(w) {
  var line = w
    ? String(w.pid || 0) + "\t" + (w.fullScreen ? "1" : "0") + "\t" + String(w.resourceClass || "")
    : "0\t0\t";
  callDBus("com.nomnom.ActiveWindow", "/ActiveWindow", "com.nomnom.ActiveWindow", "Update", line);
}
var current = function () {
  return workspace.activeWindow !== undefined ? workspace.activeWindow : workspace.activeClient;
};
var activated = workspace.windowActivated || workspace.clientActivated;
activated.connect(report);
report(current());
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Desktop {
    Kde,
    Gnome,
    Other,
}

/// From `XDG_CURRENT_DESKTOP`, which can list several names ("ubuntu:GNOME", "KDE").
pub fn desktop_of(xdg_current_desktop: &str) -> Desktop {
    let d = xdg_current_desktop.to_ascii_uppercase();
    if d.split(':').any(|p| p == "KDE") {
        Desktop::Kde
    } else if d
        .split(':')
        .any(|p| p == "GNOME" || p == "UNITY" || p == "X-CINNAMON")
    {
        Desktop::Gnome
    } else {
        Desktop::Other
    }
}

/// Folders where games live, for programs that are not Steam's own helpers.
const GAME_DIRS: &[&str] = &[
    "/steamapps/common/",
    "/games/",
    "/heroic/",
    "/lutris/games/",
];
/// Inside those folders, but not games: Steam's runtimes and Proton itself.
const NOT_GAMES: &[&str] = &[
    "/steamapps/common/proton",
    "/steamapps/common/steamlinuxruntime",
    "/steamapps/common/steam linux runtime",
    "/steamapps/common/steamworks shared",
    "/steamapps/common/steam controller configs",
    "/steamapps/common/steamvr",
    "/lutris/runners/",
    "/lutris/runtime/",
];
/// Helpers that sit next to a game while it runs.
const HELPERS: &[&str] = &[
    "crashhandler",
    "unitycrashhandler",
    "ue4prereqsetup",
    "vc_redist",
    "dxsetup",
    "unins000",
    "services.exe",
    "winedevice.exe",
    "plugplay.exe",
    "svchost.exe",
    "rpcss.exe",
    "explorer.exe",
    "steam.exe",
    "steamwebhelper",
];

/// Is this program (its path, with Wine already unwrapped to the game's `.exe`) a game?
pub fn is_game_exe(path: &str) -> bool {
    let p = path.to_ascii_lowercase().replace('\\', "/");
    if NOT_GAMES.iter().any(|n| p.contains(n)) {
        return false;
    }
    if HELPERS
        .iter()
        .any(|h| p.rsplit('/').next().is_some_and(|f| f.contains(h)))
    {
        return false;
    }
    GAME_DIRS.iter().any(|d| p.contains(d))
}

/// Does a process's environment (`/proc/<pid>/environ`, entries split by NUL) carry `key`?
pub fn environ_has(environ: &[u8], key: &str) -> bool {
    let prefix = format!("{key}=");
    environ
        .split(|&b| b == 0)
        .any(|entry| entry.starts_with(prefix.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_from_the_script_is_read_back() {
        assert_eq!(
            parse_report("4242\t1\tsteam_app_1245620"),
            Some(Focus {
                pid: 4242,
                class: "steam_app_1245620".into(),
                fullscreen: true
            })
        );
        assert_eq!(
            parse_report("0\t0\t").map(|f| (f.pid, f.fullscreen)),
            Some((0, false)),
            "no window in focus"
        );
        assert_eq!(parse_report("garbage"), None);
        assert_eq!(parse_report("12"), None, "needs the fullscreen flag too");
    }

    #[test]
    fn the_script_reports_the_focused_window_and_never_its_title() {
        assert!(KWIN_SCRIPT.contains("com.nomnom.ActiveWindow"));
        assert!(KWIN_SCRIPT.contains("windowActivated"));
        assert!(
            KWIN_SCRIPT.contains("clientActivated"),
            "Plasma 5 still works"
        );
        assert!(!KWIN_SCRIPT.contains("caption"));
    }

    #[test]
    fn desktops_are_told_apart() {
        assert_eq!(desktop_of("KDE"), Desktop::Kde);
        assert_eq!(desktop_of("ubuntu:GNOME"), Desktop::Gnome);
        assert_eq!(desktop_of("Hyprland"), Desktop::Other);
        assert_eq!(desktop_of(""), Desktop::Other);
    }

    #[test]
    fn games_are_told_from_launchers_and_helpers() {
        assert!(is_game_exe(
            "/home/u/.local/share/Steam/steamapps/common/ELDEN RING/Game/eldenring.exe"
        ));
        assert!(is_game_exe(
            "/home/u/.steam/steam/steamapps/common/Hades/Hades"
        ));
        assert!(is_game_exe(
            "/home/u/Games/Heroic/Prefixes/x/drive_c/Games/Foo/foo.exe"
        ));
        assert!(!is_game_exe(
            "/home/u/.local/share/Steam/steamapps/common/Proton 9.0/proton"
        ));
        assert!(!is_game_exe(
            "/home/u/.local/share/Steam/steamapps/common/SteamLinuxRuntime_sniper/run"
        ));
        assert!(!is_game_exe(
            "/home/u/.local/share/Steam/steamapps/common/Hades/UnityCrashHandler64.exe"
        ));
        assert!(!is_game_exe("/usr/bin/firefox"));
        assert!(!is_game_exe(
            "/home/u/.local/share/lutris/runners/wine/x/bin/wine64"
        ));
    }

    #[test]
    fn steam_marks_the_programs_of_a_game_in_their_environment() {
        let env = b"HOME=/home/u\0SteamGameId=1245620\0PATH=/usr/bin\0";
        assert!(environ_has(env, "SteamGameId"));
        assert!(!environ_has(env, "SteamAppId"));
        assert!(!environ_has(b"XSteamGameId=1\0", "SteamGameId"));
    }
}
