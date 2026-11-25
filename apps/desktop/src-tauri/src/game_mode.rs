//! Game mode. When a game has been in front for a few seconds (Playtime already knows what a game is),
//! NomNom can hold back its notices, ask Windows for its fastest power plan, and close the programs the
//! person chose. When the game has been gone for a while it puts the power plan back. Off until switched
//! on, every part can be turned off on its own, and nothing is ever force-closed.

use crate::i18n::{lang_of, Lang};
use crate::state::AppState;
use crate::toast::{self, Toast};
#[cfg(windows)]
use nomnom_core::game as rules;
use nomnom_core::game::{Change, Watch};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

static ON: AtomicBool = AtomicBool::new(false);
static QUIET: AtomicBool = AtomicBool::new(false);

/// Should a notice be held back right now? Errors, warnings and reminders always get through.
pub fn holds_back(t: &Toast) -> bool {
    QUIET.load(Ordering::Relaxed)
        && !matches!(t.kind.as_str(), "error" | "warning")
        && t.action.as_deref() != Some("reminders")
}

#[derive(Serialize)]
pub struct Status {
    pub on: bool,
    pub game: Option<String>,
}

#[cfg(windows)]
fn powercfg(args: &[&str]) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| r"C:\Windows".into())
        .join(r"System32\powercfg.exe");
    let out = std::process::Command::new(exe)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Switches to the fastest plan Windows offers. Returns the plan to put back.
#[cfg(windows)]
fn speed_up() -> Option<String> {
    let current = rules::parse_active_scheme(&powercfg(&["/getactivescheme"])?)?;
    let list = powercfg(&["/list"])?;
    let target = [rules::ULTIMATE_PERFORMANCE, rules::HIGH_PERFORMANCE]
        .into_iter()
        .find(|g| rules::lists_scheme(&list, g))?;
    if current.eq_ignore_ascii_case(target) {
        return None;
    }
    powercfg(&["/setactive", target])?;
    Some(current)
}

#[cfg(windows)]
fn put_back(plan: &str) {
    // Only an id that looks like one is handed to the tool.
    if rules::is_guid(plan) {
        let _ = powercfg(&["/setactive", plan]);
    }
}

/// Asks each program to close, the way its window's close button would. Returns those that were open.
#[cfg(windows)]
fn close_programs(names: &[String]) -> Vec<String> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| r"C:\Windows".into())
        .join(r"System32\taskkill.exe");
    let mut closed = Vec::new();
    for n in names {
        if rules::is_protected_program(n) {
            continue;
        }
        // No /F: the program is asked, so it can ask about unsaved work.
        let ok = std::process::Command::new(&exe)
            .args(["/IM", n])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x0800_0000)
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            closed.push(n.clone());
        }
    }
    closed
}

fn text(lang: Lang, entered: bool, faster: bool, closed: &[String]) -> (String, String) {
    let title = match (lang, entered) {
        (Lang::En, true) => "Game mode is on",
        (Lang::En, false) => "Game mode is off",
        (Lang::Pt, true) => "Modo jogo ligado",
        (Lang::Pt, false) => "Modo jogo desligado",
        (Lang::Es, true) => "Modo juego activado",
        (Lang::Es, false) => "Modo juego desactivado",
    };
    let mut parts: Vec<String> = Vec::new();
    if entered {
        parts.push(
            match lang {
                Lang::En => "Notices are held back until you stop playing.",
                Lang::Pt => "Os avisos ficam guardados até você parar de jogar.",
                Lang::Es => "Los avisos se guardan hasta que dejes de jugar.",
            }
            .into(),
        );
        if faster {
            parts.push(
                match lang {
                    Lang::En => "Windows is using its fastest power plan.",
                    Lang::Pt => "O Windows está no plano de energia mais rápido.",
                    Lang::Es => "Windows usa su plan de energía más rápido.",
                }
                .into(),
            );
        }
        if !closed.is_empty() {
            parts.push(match lang {
                Lang::En => format!("Closed: {}.", closed.join(", ")),
                Lang::Pt => format!("Fechados: {}.", closed.join(", ")),
                Lang::Es => format!("Cerrados: {}.", closed.join(", ")),
            });
        }
    } else {
        parts.push(
            match lang {
                Lang::En => "Everything is back as it was.",
                Lang::Pt => "Tudo voltou como estava.",
                Lang::Es => "Todo ha vuelto como estaba.",
            }
            .into(),
        );
    }
    (title.into(), parts.join(" "))
}

fn enter(app: &AppHandle) {
    let st = app.state::<AppState>();
    let cfg = st.settings.get().game;
    let lang = lang_of(&st.settings.get());
    #[allow(unused_mut)]
    let mut faster = false;
    #[allow(unused_mut)]
    let mut closed: Vec<String> = Vec::new();
    #[cfg(windows)]
    {
        if cfg.power_plan {
            if let Some(previous) = speed_up() {
                faster = true;
                // Kept on disk, so a crash during the game can still be repaired.
                let _ = st
                    .settings
                    .update("game", |s| s.game.restore_plan = Some(previous));
            }
        }
        if !cfg.close.is_empty() {
            let names = cfg.close.clone();
            closed = close_programs(&names);
        }
    }
    let (title, body) = text(lang, true, faster, &closed);
    // The card that says it is on is shown before the quiet starts.
    toast::show(app, Toast::new("info", title, body).opens("gamemode"));
    ON.store(true, Ordering::Relaxed);
    QUIET.store(cfg.silence, Ordering::Relaxed);
}

fn leave(app: &AppHandle) {
    let st = app.state::<AppState>();
    let lang = lang_of(&st.settings.get());
    ON.store(false, Ordering::Relaxed);
    QUIET.store(false, Ordering::Relaxed);
    restore(app);
    let (title, body) = text(lang, false, false, &[]);
    toast::show(app, Toast::new("info", title, body).opens("gamemode"));
}

/// Puts the power plan back if one was changed (also repairs what a crash left behind).
fn restore(app: &AppHandle) {
    let st = app.state::<AppState>();
    let plan = st.settings.get().game.restore_plan;
    if let Some(plan) = plan {
        #[cfg(windows)]
        put_back(&plan);
        #[cfg(not(windows))]
        let _ = plan;
        let _ = st.settings.update("game", |s| s.game.restore_plan = None);
    }
}

fn game_in_front(app: &AppHandle) -> Option<String> {
    let st = app.state::<AppState>();
    let live = st.activity.live()?;
    (live.category == "game" && !live.is_afk).then_some(live.name)
}

static TRACKED_GAME: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // A crash in the middle of a game must not leave the computer on another power plan.
        let app_r = app.clone();
        let _ = tokio::task::spawn_blocking(move || restore(&app_r)).await;
        let mut watch = Watch::default();
        loop {
            tokio::time::sleep(Duration::from_secs(4)).await;
            let on = app.state::<AppState>().settings.get().game.enabled;
            if !on {
                if watch.is_on() || ON.load(Ordering::Relaxed) {
                    watch.reset();
                    ON.store(false, Ordering::Relaxed);
                    QUIET.store(false, Ordering::Relaxed);
                    *TRACKED_GAME.lock().unwrap() = None;
                    let app_r = app.clone();
                    let _ = tokio::task::spawn_blocking(move || restore(&app_r)).await;
                }
                continue;
            }
            let now = chrono::Utc::now().timestamp();
            let front = game_in_front(&app);
            if let Some(ref g) = front {
                *TRACKED_GAME.lock().unwrap() = Some(g.clone());
            }
            match watch.step(front.is_some(), now) {
                Some(Change::Entered) => {
                    let app_e = app.clone();
                    let _ = tokio::task::spawn_blocking(move || enter(&app_e)).await;
                }
                Some(Change::Left) => {
                    *TRACKED_GAME.lock().unwrap() = None;
                    let app_l = app.clone();
                    let _ = tokio::task::spawn_blocking(move || leave(&app_l)).await;
                }
                None => {}
            }
        }
    });
}

#[tauri::command]
pub fn gm_status(app: AppHandle, state: State<'_, AppState>) -> Status {
    let _ = (&app, &state);
    let on = ON.load(Ordering::Relaxed);
    Status {
        on,
        game: if on {
            TRACKED_GAME.lock().unwrap().clone()
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_and_reminders_get_through_and_the_rest_waits() {
        QUIET.store(true, Ordering::Relaxed);
        assert!(holds_back(&Toast::new("success", "Done", "x")));
        assert!(holds_back(
            &Toast::new("info", "News", "x").opens("downloads")
        ));
        assert!(!holds_back(&Toast::new("error", "Broke", "x")));
        assert!(!holds_back(&Toast::new("warning", "Disk", "x")));
        assert!(!holds_back(
            &Toast::new("info", "Reminder", "x").opens("reminders")
        ));
        QUIET.store(false, Ordering::Relaxed);
        assert!(!holds_back(&Toast::new("success", "Done", "x")));
    }

    /// Changes the power plan for a moment and puts it back. Run by hand: `cargo test -p nomnom --
    /// --ignored power_plan`.
    #[test]
    #[ignore]
    #[cfg(windows)]
    fn the_power_plan_really_changes_and_comes_back() {
        let before = rules::parse_active_scheme(&powercfg(&["/getactivescheme"]).unwrap()).unwrap();
        let previous = speed_up();
        let during = rules::parse_active_scheme(&powercfg(&["/getactivescheme"]).unwrap()).unwrap();
        if let Some(p) = &previous {
            assert_eq!(p, &before);
            assert_ne!(during, before, "the plan should have changed");
            put_back(p);
        }
        let after = rules::parse_active_scheme(&powercfg(&["/getactivescheme"]).unwrap()).unwrap();
        println!("before {before} / during {during} / after {after}");
        assert_eq!(after, before, "the old plan must be back");
    }

    #[test]
    fn the_cards_say_what_was_done_in_every_language() {
        for lang in [Lang::En, Lang::Pt, Lang::Es] {
            let (t, b) = text(lang, true, true, &["OneDrive.exe".into()]);
            assert!(!t.is_empty() && b.contains("OneDrive.exe"));
            let (t2, b2) = text(lang, false, false, &[]);
            assert_ne!(t, t2);
            assert!(!b2.is_empty());
        }
    }
}
