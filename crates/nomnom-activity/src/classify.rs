//! Normalizes OS process paths and window classes into clean app names and categories,
//! and filters out desktop shell overlays.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppCategory {
    Creative,
    Game,
    Browser,
    Social,
    Work,
    Media,
    General,
}

impl AppCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            AppCategory::Creative => "creative",
            AppCategory::Game => "game",
            AppCategory::Browser => "browser",
            AppCategory::Social => "social",
            AppCategory::Work => "work",
            AppCategory::Media => "media",
            AppCategory::General => "general",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "creative" => AppCategory::Creative,
            "game" | "games" => AppCategory::Game,
            "browser" | "browsers" => AppCategory::Browser,
            "social" | "chat" => AppCategory::Social,
            "work" | "dev" => AppCategory::Work,
            "media" | "music" => AppCategory::Media,
            _ => AppCategory::General,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawForeground {
    pub pid: u32,
    pub exe_path: Option<String>,
    pub wm_class: Option<String>,
    pub desktop_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppIdentity {
    pub id: String,
    pub name: String,
    pub exe_path: Option<String>,
    pub wm_class: Option<String>,
    pub category: AppCategory,
}

const SHELL_NOISE_STEMS: &[&str] = &[
    "searchhost",
    "searchapp",
    "shellexperiencehost",
    "startmenuexperiencehost",
    "lockapp",
    "logonui",
    "screenclippinghost",
    "textinputhost",
    "applicationframehost",
    "dwm",
    "csrss",
    "sihost",
    "taskmgr",
    "consent",
    "plasmashell",
    "krunner",
    "kscreenlocker_greet",
    "polkit-kde-authentication-agent-1",
    "hyprpolkitagent",
    "waybar",
    "rofi",
    "wofi",
    "fuzzel",
    "dmenu",
    "swaync",
    "mako",
    "dunst",
    "hyprlock",
    "swaylock",
    "i3lock",
    "gtklock",
    "xdg-desktop-portal",
    "xdg-desktop-portal-gtk",
    "xdg-desktop-portal-kde",
    "xdg-desktop-portal-hyprland",
];

struct KnownApp {
    keys: &'static [&'static str],
    canonical_id: &'static str,
    name: &'static str,
    category: AppCategory,
}

const KNOWN_APPS: &[KnownApp] = &[
    // Creative & Art
    KnownApp {
        keys: &["clipstudiopaint", "clip studio paint"],
        canonical_id: "app:clip-studio-paint",
        name: "Clip Studio Paint",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["clipstudio"],
        canonical_id: "app:clip-studio",
        name: "Clip Studio",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["krita", "org.kde.krita"],
        canonical_id: "app:krita",
        name: "Krita",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["photoshop", "adobephotoshop"],
        canonical_id: "app:photoshop",
        name: "Adobe Photoshop",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["illustrator"],
        canonical_id: "app:illustrator",
        name: "Adobe Illustrator",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["afterfx", "after effects"],
        canonical_id: "app:after-effects",
        name: "After Effects",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["adobe premiere pro", "premiere"],
        canonical_id: "app:premiere-pro",
        name: "Premiere Pro",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["blender", "org.blender.blender"],
        canonical_id: "app:blender",
        name: "Blender",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["aseprite"],
        canonical_id: "app:aseprite",
        name: "Aseprite",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["gimp", "gimp-2.10", "gimp-3.0", "org.gimp.gimp"],
        canonical_id: "app:gimp",
        name: "GIMP",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["inkscape", "org.inkscape.inkscape"],
        canonical_id: "app:inkscape",
        name: "Inkscape",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["resolve", "davinci resolve"],
        canonical_id: "app:davinci-resolve",
        name: "DaVinci Resolve",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["figma"],
        canonical_id: "app:figma",
        name: "Figma",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["obs64", "obs", "com.obsproject.studio"],
        canonical_id: "app:obs-studio",
        name: "OBS Studio",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["pureRef", "pureref"],
        canonical_id: "app:pureref",
        name: "PureRef",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["live2d", "cubism"],
        canonical_id: "app:live2d",
        name: "Live2D Cubism",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["zbrush"],
        canonical_id: "app:zbrush",
        name: "ZBrush",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["fl64", "fl studio"],
        canonical_id: "app:fl-studio",
        name: "FL Studio",
        category: AppCategory::Creative,
    },
    KnownApp {
        keys: &["ableton", "live"],
        canonical_id: "app:ableton-live",
        name: "Ableton Live",
        category: AppCategory::Creative,
    },
    // Browsers
    KnownApp {
        keys: &["chrome", "google-chrome", "google-chrome-stable"],
        canonical_id: "app:google-chrome",
        name: "Google Chrome",
        category: AppCategory::Browser,
    },
    KnownApp {
        keys: &["firefox", "org.mozilla.firefox", "firefox-esr"],
        canonical_id: "app:firefox",
        name: "Firefox",
        category: AppCategory::Browser,
    },
    KnownApp {
        keys: &["librewolf", "io.gitlab.librewolf-community"],
        canonical_id: "app:librewolf",
        name: "LibreWolf",
        category: AppCategory::Browser,
    },
    KnownApp {
        keys: &["zen", "zen-browser", "app.zen_browser.zen"],
        canonical_id: "app:zen-browser",
        name: "Zen Browser",
        category: AppCategory::Browser,
    },
    KnownApp {
        keys: &["brave", "brave-browser"],
        canonical_id: "app:brave",
        name: "Brave",
        category: AppCategory::Browser,
    },
    KnownApp {
        keys: &["msedge", "microsoft-edge"],
        canonical_id: "app:edge",
        name: "Microsoft Edge",
        category: AppCategory::Browser,
    },
    KnownApp {
        keys: &["vivaldi", "vivaldi-stable"],
        canonical_id: "app:vivaldi",
        name: "Vivaldi",
        category: AppCategory::Browser,
    },
    KnownApp {
        keys: &["opera"],
        canonical_id: "app:opera",
        name: "Opera",
        category: AppCategory::Browser,
    },
    KnownApp {
        keys: &["chromium", "org.chromium.chromium"],
        canonical_id: "app:chromium",
        name: "Chromium",
        category: AppCategory::Browser,
    },
    // Social & Chat
    KnownApp {
        keys: &["discord", "com.discordapp.discord"],
        canonical_id: "app:discord",
        name: "Discord",
        category: AppCategory::Social,
    },
    KnownApp {
        keys: &["vesktop", "dev.vencord.vesktop"],
        canonical_id: "app:vesktop",
        name: "Vesktop",
        category: AppCategory::Social,
    },
    KnownApp {
        keys: &["telegram", "telegram-desktop", "org.telegram.desktop"],
        canonical_id: "app:telegram",
        name: "Telegram",
        category: AppCategory::Social,
    },
    KnownApp {
        keys: &["slack", "com.slack.slack"],
        canonical_id: "app:slack",
        name: "Slack",
        category: AppCategory::Social,
    },
    KnownApp {
        keys: &["whatsapp"],
        canonical_id: "app:whatsapp",
        name: "WhatsApp",
        category: AppCategory::Social,
    },
    // Work & Development
    KnownApp {
        keys: &["code", "code-oss", "vscodium"],
        canonical_id: "app:vscode",
        name: "VS Code",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["cursor"],
        canonical_id: "app:cursor",
        name: "Cursor",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["zed", "dev.zed.zed"],
        canonical_id: "app:zed",
        name: "Zed",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["obsidian", "md.obsidian.obsidian"],
        canonical_id: "app:obsidian",
        name: "Obsidian",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["notion"],
        canonical_id: "app:notion",
        name: "Notion",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["soffice", "soffice.bin", "libreoffice"],
        canonical_id: "app:libreoffice",
        name: "LibreOffice",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["winword"],
        canonical_id: "app:word",
        name: "Microsoft Word",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["excel"],
        canonical_id: "app:excel",
        name: "Microsoft Excel",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["powerpnt"],
        canonical_id: "app:powerpoint",
        name: "PowerPoint",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["windowsterminal", "wt"],
        canonical_id: "app:windows-terminal",
        name: "Windows Terminal",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["kitty"],
        canonical_id: "app:kitty",
        name: "Kitty Terminal",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["alacritty", "org.alacritty.alacritty"],
        canonical_id: "app:alacritty",
        name: "Alacritty",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["wezterm", "wezterm-gui", "org.wezfurlong.wezterm"],
        canonical_id: "app:wezterm",
        name: "WezTerm",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["ghostty", "com.mitchellh.ghostty"],
        canonical_id: "app:ghostty",
        name: "Ghostty",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["konsole", "org.kde.konsole"],
        canonical_id: "app:konsole",
        name: "Konsole",
        category: AppCategory::Work,
    },
    KnownApp {
        keys: &["dolphin", "org.kde.dolphin"],
        canonical_id: "app:dolphin",
        name: "Dolphin",
        category: AppCategory::General,
    },
    KnownApp {
        keys: &["explorer"],
        canonical_id: "app:explorer",
        name: "File Explorer",
        category: AppCategory::General,
    },
    KnownApp {
        keys: &["org.gnome.nautilus", "nautilus"],
        canonical_id: "app:nautilus",
        name: "GNOME Files",
        category: AppCategory::General,
    },
    // Media
    KnownApp {
        keys: &["spotify", "com.spotify.client"],
        canonical_id: "app:spotify",
        name: "Spotify",
        category: AppCategory::Media,
    },
    KnownApp {
        keys: &["vlc", "org.videolan.vlc"],
        canonical_id: "app:vlc",
        name: "VLC",
        category: AppCategory::Media,
    },
    KnownApp {
        keys: &["mpv", "io.mpv.mpv"],
        canonical_id: "app:mpv",
        name: "mpv",
        category: AppCategory::Media,
    },
    // Game Launchers & Popular Games
    KnownApp {
        keys: &["steam", "steamwebhelper"],
        canonical_id: "app:steam",
        name: "Steam",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["epicgameslauncher"],
        canonical_id: "app:epic-games",
        name: "Epic Games",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["lutris", "net.lutris.lutris"],
        canonical_id: "app:lutris",
        name: "Lutris",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["heroic", "com.heroicgameslauncher.hgl"],
        canonical_id: "app:heroic",
        name: "Heroic Launcher",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["prismlauncher", "org.prismlauncher.prismlauncher"],
        canonical_id: "app:prismlauncher",
        name: "Prism Launcher",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["javaw", "minecraft"],
        canonical_id: "app:minecraft",
        name: "Minecraft",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["valorant-win64-shipping", "valorant"],
        canonical_id: "app:valorant",
        name: "VALORANT",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["league of legends", "leagueclientux"],
        canonical_id: "app:league-of-legends",
        name: "League of Legends",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["cs2"],
        canonical_id: "app:counter-strike-2",
        name: "Counter-Strike 2",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["eldenring"],
        canonical_id: "app:elden-ring",
        name: "Elden Ring",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["genshinimpact", "yuanshen"],
        canonical_id: "app:genshin-impact",
        name: "Genshin Impact",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["starrail"],
        canonical_id: "app:honkai-star-rail",
        name: "Honkai: Star Rail",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["zenlesszonezero"],
        canonical_id: "app:zenless-zone-zero",
        name: "Zenless Zone Zero",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["osu!"],
        canonical_id: "app:osu",
        name: "osu!",
        category: AppCategory::Game,
    },
    KnownApp {
        keys: &["nomnom"],
        canonical_id: "app:nomnom",
        name: "NomNom",
        category: AppCategory::General,
    },
];

fn exe_stem(exe_path: &str) -> Option<String> {
    let normalized = exe_path.replace('\\', "/");
    let file_name = normalized.rsplit('/').next()?;
    let stem = file_name
        .strip_suffix(".exe")
        .or_else(|| file_name.strip_suffix(".EXE"))
        .or_else(|| file_name.strip_suffix(".AppImage"))
        .or_else(|| file_name.strip_suffix(".appimage"))
        .unwrap_or(file_name);
    let trimmed = stem.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Extracts the game folder name from paths like `.../steamapps/common/Hollow Knight/hollow_knight.exe`
/// or `.../Epic Games/Celeste/Celeste.exe`.
fn detect_game_install_folder(exe_path: &str) -> Option<(String, String)> {
    let norm = exe_path.replace('\\', "/");
    let lower = norm.to_ascii_lowercase();
    for marker in [
        "/steamapps/common/",
        "/epic games/",
        "/gog galaxy/games/",
        "/games/heroic/",
        "/lutris/games/",
    ] {
        if let Some(idx) = lower.find(marker) {
            let rest = &norm[idx + marker.len()..];
            if let Some(folder) = rest.split('/').next().filter(|s| !s.is_empty()) {
                let fl = folder.to_ascii_lowercase();
                if fl.starts_with("proton")
                    || fl.starts_with("steamlinuxruntime")
                    || fl == "steamworks shared"
                {
                    continue;
                }
                let slug = slugify(folder);
                return Some((format!("game:{slug}"), folder.to_string()));
            }
        }
    }
    // Steam AppID in Linux WM_CLASS (`steam_app_1245620`)
    None
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "unknown".into()
    } else {
        out
    }
}

fn prettify_stem(raw: &str) -> String {
    let cleaned = raw
        .trim_end_matches("-Win64-Shipping")
        .trim_end_matches("-win64-shipping")
        .trim_end_matches("_x64")
        .trim_end_matches("-x64")
        .replace(['_', '-'], " ");
    // Split camelCase (`ClipStudioPaint` -> `Clip Studio Paint`) when mixed case is present.
    let mut spaced = String::with_capacity(cleaned.len() + 4);
    let chars: Vec<char> = cleaned.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
            if c.is_ascii_uppercase()
                && (prev.is_ascii_lowercase() || (prev.is_ascii_uppercase() && next_lower))
                && prev != ' '
            {
                spaced.push(' ');
            }
        }
        spaced.push(c);
    }
    spaced
        .split_whitespace()
        .map(|word| {
            let mut cs = word.chars();
            match cs.next() {
                None => String::new(),
                Some(first) => {
                    if word.chars().any(|c| c.is_ascii_uppercase()) {
                        word.to_string()
                    } else {
                        first.to_uppercase().collect::<String>() + cs.as_str()
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns `None` when the foreground window is system/shell chrome that should not be tracked.
pub fn classify_app(raw: &RawForeground) -> Option<AppIdentity> {
    let stem = raw.exe_path.as_deref().and_then(exe_stem);
    let stem_lower = stem.as_deref().map(str::to_ascii_lowercase);
    let class_lower = raw
        .wm_class
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    if let Some(ref s) = stem_lower {
        if SHELL_NOISE_STEMS.contains(&s.as_str()) {
            return None;
        }
    }
    if let Some(ref c) = class_lower {
        if SHELL_NOISE_STEMS.contains(&c.as_str()) {
            return None;
        }
    }

    // 1. Check if the executable lives inside a game library folder (`steamapps/common/<Game>`, etc.)
    if let Some(ref path) = raw.exe_path {
        if let Some((game_id, game_folder)) = detect_game_install_folder(path) {
            return Some(AppIdentity {
                id: game_id,
                name: raw
                    .desktop_name
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(game_folder),
                exe_path: raw.exe_path.clone(),
                wm_class: raw.wm_class.clone(),
                category: AppCategory::Game,
            });
        }
    }

    // 2. Match curated well-known creative/work/social/browser/game apps
    for known in KNOWN_APPS {
        let matched = known.keys.iter().any(|k| {
            let kl = k.to_ascii_lowercase();
            stem_lower.as_deref() == Some(kl.as_str())
                || class_lower.as_deref() == Some(kl.as_str())
        });
        if matched {
            return Some(AppIdentity {
                id: known.canonical_id.to_string(),
                name: known.name.to_string(),
                exe_path: raw.exe_path.clone(),
                wm_class: raw.wm_class.clone(),
                category: known.category,
            });
        }
    }

    // 3. Steam Proton / Linux `steam_app_<id>` fallback when exe path wasn't inside steamapps
    if let Some(ref c) = class_lower {
        if c.starts_with("steam_app_") {
            let name = raw
                .desktop_name
                .clone()
                .or_else(|| stem.as_deref().map(prettify_stem))
                .unwrap_or_else(|| format!("Steam Game ({})", c.trim_start_matches("steam_app_")));
            return Some(AppIdentity {
                id: format!("game:{}", slugify(c)),
                name,
                exe_path: raw.exe_path.clone(),
                wm_class: raw.wm_class.clone(),
                category: AppCategory::Game,
            });
        }
    }

    // 4. Generic app resolution: prefer `.desktop` Name (on Linux) or prettified stem / WM_CLASS
    let base_key = class_lower
        .as_deref()
        .or(stem_lower.as_deref())
        .filter(|s| !s.is_empty())?;

    // Ignore bare system shells if no GUI class exists
    if raw.wm_class.is_none()
        && matches!(
            base_key,
            "bash" | "zsh" | "fish" | "sh" | "conhost" | "cmd" | "powershell" | "pwsh" | "systemd"
        )
    {
        return None;
    }

    let display_name = raw
        .desktop_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| stem.as_deref().map(prettify_stem))
        .or_else(|| raw.wm_class.as_deref().map(prettify_stem))?;

    let category = infer_category_from_path(raw.exe_path.as_deref());
    let id = if let Some(ref st) = stem_lower {
        format!("app:{}", slugify(st))
    } else {
        format!("app:{}", slugify(base_key))
    };

    Some(AppIdentity {
        id,
        name: display_name,
        exe_path: raw.exe_path.clone(),
        wm_class: raw.wm_class.clone(),
        category,
    })
}

fn infer_category_from_path(exe_path: Option<&str>) -> AppCategory {
    let Some(p) = exe_path else {
        return AppCategory::General;
    };
    let lower = p.replace('\\', "/").to_ascii_lowercase();
    if lower.contains("/games/")
        || lower.contains("/riot games/")
        || lower.contains("/xboxgames/")
        || lower.contains("/battle.net/")
        || lower.contains("/ubisoft/")
        || Path::new(p)
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.to_ascii_lowercase().ends_with("-win64-shipping.exe"))
    {
        AppCategory::Game
    } else {
        AppCategory::General
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_clip_studio_chrome_and_discord() {
        let csp = classify_app(&RawForeground {
            pid: 100,
            exe_path: Some(
                r"C:\Program Files\CELSYS\CLIP STUDIO 1.5\CLIP STUDIO PAINT\CLIPStudioPaint.exe"
                    .into(),
            ),
            wm_class: None,
            desktop_name: None,
        })
        .unwrap();
        assert_eq!(csp.id, "app:clip-studio-paint");
        assert_eq!(csp.name, "Clip Studio Paint");
        assert_eq!(csp.category, AppCategory::Creative);

        let chrome = classify_app(&RawForeground {
            pid: 101,
            exe_path: Some("/usr/lib/chromium/chrome".into()),
            wm_class: Some("google-chrome".into()),
            desktop_name: None,
        })
        .unwrap();
        assert_eq!(chrome.id, "app:google-chrome");
        assert_eq!(chrome.name, "Google Chrome");
        assert_eq!(chrome.category, AppCategory::Browser);

        let discord = classify_app(&RawForeground {
            pid: 102,
            exe_path: Some("/opt/discord/Discord".into()),
            wm_class: Some("discord".into()),
            desktop_name: None,
        })
        .unwrap();
        assert_eq!(discord.id, "app:discord");
        assert_eq!(discord.category, AppCategory::Social);
    }

    #[test]
    fn extracts_steam_game_folder_names_on_windows_and_linux() {
        let bg3 = classify_app(&RawForeground {
            pid: 200,
            exe_path: Some(
                r"D:\SteamLibrary\steamapps\common\Baldurs Gate 3\bin\bg3_dx11.exe".into(),
            ),
            wm_class: None,
            desktop_name: None,
        })
        .unwrap();
        assert_eq!(bg3.id, "game:baldurs-gate-3");
        assert_eq!(bg3.name, "Baldurs Gate 3");
        assert_eq!(bg3.category, AppCategory::Game);

        let hollow = classify_app(&RawForeground {
            pid: 201,
            exe_path: Some(
                "/home/arch/.local/share/Steam/steamapps/common/Hollow Knight/hollow_knight.x86_64"
                    .into(),
            ),
            wm_class: Some("steam_app_367520".into()),
            desktop_name: None,
        })
        .unwrap();
        assert_eq!(hollow.id, "game:hollow-knight");
        assert_eq!(hollow.name, "Hollow Knight");
        assert_eq!(hollow.category, AppCategory::Game);
    }

    #[test]
    fn filters_out_shell_and_lockscreen_noise() {
        for noise in ["SearchHost.exe", "LockApp.exe", "waybar", "hyprlock"] {
            assert!(classify_app(&RawForeground {
                pid: 300,
                exe_path: Some(noise.into()),
                wm_class: Some(noise.into()),
                desktop_name: None,
            })
            .is_none());
        }
    }
}
