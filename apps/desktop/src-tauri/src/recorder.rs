//! Recording the screen to a video (MP4) or a small GIF, with ffmpeg (the one NomNom already manages
//! for File Tools). Windows for now. A recording is of the screen only, with no sound. The file is
//! written in pieces, so one cut short by a crash still plays, and a recording is stopped for good
//! when NomNom closes.

use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

type CmdResult<T> = Result<T, String>;

#[derive(Debug, Serialize, Default)]
pub struct Status {
    pub recording: bool,
    pub seconds: u64,
    pub clock: String,
    pub file: Option<String>,
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::i18n::{lang_of, tr};
    use crate::toast::{self, Toast};
    use nomnom_core::recorder::{self as rules, Area};
    use nomnom_tools::Tool;
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    struct Active {
        child: Child,
        out: PathBuf,
        started: Instant,
        gif: bool,
    }
    static ACTIVE: Mutex<Option<Active>> = Mutex::new(None);

    fn say(app: &AppHandle, kind: &str, title: &str, body: &str) {
        let lang = lang_of(&app.state::<AppState>().settings.get());
        toast::show(
            app,
            Toast::new(kind, tr(lang, title), tr(lang, body)).opens("recorder"),
        );
    }

    fn out_dir(state: &AppState) -> PathBuf {
        state
            .paths
            .downloads
            .parent()
            .map(|p| p.join("Videos"))
            .unwrap_or_else(|| state.paths.downloads.clone())
            .join("NomNom")
    }

    pub fn status() -> Status {
        let guard = ACTIVE.lock().unwrap();
        match guard.as_ref() {
            Some(a) => {
                let s = a.started.elapsed().as_secs();
                Status {
                    recording: true,
                    seconds: s,
                    clock: rules::clock(s),
                    file: Some(a.out.to_string_lossy().into_owned()),
                }
            }
            None => Status::default(),
        }
    }

    /// Starts recording `area` (whole-desktop pixels). `gif` makes a GIF when it stops.
    pub fn start(app: &AppHandle, area: Area, gif: bool) -> CmdResult<()> {
        let state = app.state::<AppState>();
        if ACTIVE.lock().unwrap().is_some() {
            return Err("A recording is already running.".into());
        }
        let area = rules::fit(area).ok_or("That area is too small to record.")?;
        let ffmpeg = state
            .tools
            .locate(Tool::Ffmpeg)
            .ok_or("Recording needs ffmpeg. Open File Tools once, or the Downloader, and let it set up its tools.")?;
        let dir = out_dir(&state);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Couldn't make the folder for recordings: {e}"))?;
        let stamp = chrono::Local::now().format("%Y-%m-%d %H-%M-%S").to_string();
        // A GIF is made from a video, so the video is made first (and kept only until then).
        let video = dir.join(rules::file_name(&stamp, false));
        let out = if gif {
            dir.join(rules::file_name(&stamp, true))
        } else {
            video.clone()
        };
        let args = rules::record_args(area, 30, 600, &video.to_string_lossy());
        let child = Command::new(ffmpeg)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|e| format!("Couldn't start ffmpeg: {e}"))?;
        *ACTIVE.lock().unwrap() = Some(Active {
            child,
            out: if gif { video } else { out },
            started: Instant::now(),
            gif,
        });
        say(
            app,
            "info",
            "Recording",
            "Recording the screen. Press the shortcut again, or use the page, to stop.",
        );
        Ok(())
    }

    /// Stops the recording and, if asked, turns it into a GIF. Returns the file.
    pub fn stop(app: &AppHandle) -> CmdResult<String> {
        let mut active = ACTIVE
            .lock()
            .unwrap()
            .take()
            .ok_or("Nothing is being recorded.")?;
        // "q" makes ffmpeg finish the file properly.
        if let Some(mut stdin) = active.child.stdin.take() {
            let _ = stdin.write_all(b"q\n");
            let _ = stdin.flush();
        }
        let waited = Instant::now();
        loop {
            match active.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if waited.elapsed() > Duration::from_secs(10) => {
                    let _ = active.child.kill();
                    let _ = active.child.wait();
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => break,
            }
        }
        let video = active.out.clone();
        let ok = std::fs::metadata(&video).is_ok_and(|m| m.len() > 1024);
        if !ok {
            let _ = std::fs::remove_file(&video);
            say(
                app,
                "error",
                "Recording",
                "The recording didn't work. ffmpeg couldn't capture the screen.",
            );
            return Err("The recording didn't work.".into());
        }
        let final_path = if active.gif {
            let gif = video.with_extension("gif");
            let state = app.state::<AppState>();
            let made = state.tools.locate(Tool::Ffmpeg).is_some_and(|ff| {
                Command::new(ff)
                    .args(rules::gif_args(
                        &video.to_string_lossy(),
                        &gif.to_string_lossy(),
                        12,
                        720,
                    ))
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .creation_flags(0x0800_0000)
                    .status()
                    .is_ok_and(|s| s.success())
            }) && std::fs::metadata(&gif).is_ok_and(|m| m.len() > 0);
            if made {
                // The video was only a step on the way to the GIF.
                let _ = std::fs::remove_file(&video);
                gif
            } else {
                video
            }
        } else {
            video
        };
        say(
            app,
            "success",
            "Recording saved",
            &final_path.to_string_lossy(),
        );
        Ok(final_path.to_string_lossy().into_owned())
    }

    /// NomNom is closing: the recording is stopped, not left running.
    pub fn shutdown() {
        if let Some(mut a) = ACTIVE.lock().unwrap().take() {
            if let Some(mut stdin) = a.child.stdin.take() {
                let _ = stdin.write_all(b"q\n");
            }
            std::thread::sleep(Duration::from_millis(800));
            let _ = a.child.kill();
            let _ = a.child.wait();
        }
    }

    pub fn folder(app: &AppHandle) -> PathBuf {
        out_dir(&app.state::<AppState>())
    }

    /// The monitor under the pointer, as an area.
    pub fn monitor_area(app: &AppHandle) -> Option<Area> {
        let c = app.cursor_position().ok()?;
        let m = app
            .monitor_from_point(c.x, c.y)
            .ok()
            .flatten()
            .or_else(|| app.primary_monitor().ok().flatten())?;
        Some(Area {
            x: m.position().x,
            y: m.position().y,
            w: m.size().width,
            h: m.size().height,
        })
    }

    pub use rules::Area as AreaT;
}

#[tauri::command]
pub fn rec_status() -> Status {
    #[cfg(windows)]
    {
        imp::status()
    }
    #[cfg(not(windows))]
    {
        Status::default()
    }
}

/// Records the whole monitor the pointer is on.
#[tauri::command]
pub fn rec_start_screen(app: AppHandle, gif: bool) -> CmdResult<()> {
    #[cfg(windows)]
    {
        let area = imp::monitor_area(&app).ok_or("Couldn't find the screen.")?;
        imp::start(&app, area, gif)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, gif);
        Err("Recording the screen works on Windows for now.".into())
    }
}

/// Lets the person choose a rectangle on the frozen screen, then records it.
#[tauri::command]
pub fn rec_start_area(app: AppHandle, gif: bool) -> CmdResult<()> {
    #[cfg(windows)]
    {
        if imp::status().recording {
            return Err("A recording is already running.".into());
        }
        crate::shots::start_for_recording(&app, gif);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (app, gif);
        Err("Recording the screen works on Windows for now.".into())
    }
}

/// Called with the rectangle chosen on the overlay (whole-desktop pixels).
#[cfg(windows)]
pub fn start_chosen(app: &AppHandle, x: i32, y: i32, w: u32, h: u32, gif: bool) {
    if let Err(e) = imp::start(app, imp::AreaT { x, y, w, h }, gif) {
        let lang = crate::i18n::lang_of(&app.state::<AppState>().settings.get());
        crate::toast::show(
            app,
            crate::toast::Toast::new(
                "error",
                crate::i18n::tr(lang, "Recording"),
                crate::i18n::tr(lang, &e),
            ),
        );
    }
}

#[tauri::command]
pub async fn rec_stop(app: AppHandle) -> CmdResult<String> {
    #[cfg(windows)]
    {
        tokio::task::spawn_blocking(move || imp::stop(&app))
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Nothing is being recorded.".into())
    }
}

/// The shortcut: stops a recording, or starts one on a chosen area.
pub fn toggle(app: &AppHandle) {
    #[cfg(windows)]
    {
        if imp::status().recording {
            let app = app.clone();
            std::thread::spawn(move || {
                let _ = imp::stop(&app);
            });
        } else {
            let _ = rec_start_area(app.clone(), false);
        }
    }
    #[cfg(not(windows))]
    {
        let lang = crate::i18n::lang_of(&app.state::<AppState>().settings.get());
        crate::toast::show(
            app,
            crate::toast::Toast::new(
                "info",
                "NomNom",
                crate::i18n::tr(lang, "Recording the screen works on Windows for now."),
            ),
        );
    }
}

pub fn shutdown() {
    #[cfg(windows)]
    imp::shutdown();
}

/// Where recordings are kept.
#[tauri::command]
pub fn rec_folder(app: AppHandle, state: State<'_, AppState>) -> String {
    #[cfg(windows)]
    {
        let _ = &state;
        imp::folder(&app).to_string_lossy().into_owned()
    }
    #[cfg(not(windows))]
    {
        let _ = (app, state);
        String::new()
    }
}
