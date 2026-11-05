//! Everything the Firefox extension can ask NOMNOM to do, and the Settings side of the connection.

use crate::modules::downloads::{
    default_choice, enqueue_with_defaults, AUDIO_FORMATS, VIDEO_FORMATS,
};
use crate::state::AppState;
use nomnom_core::settings::{DefaultKind, Downloads};
use nomnom_downloads::url_check::{is_probably_supported, validate_url};
use nomnom_downloads::{Choice, Container, DirectRequest, EnqueueOutcome};
use nomnom_ipc::{Body, ErrorCode, Preset, PresetKind, Request, Response};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

/// At most 20 messages in any 10 seconds, so a runaway page can't flood the queue.
pub struct RateLimiter {
    window: Mutex<VecDeque<Instant>>,
}

const LIMIT: usize = 20;
const WINDOW: Duration = Duration::from_secs(10);

impl RateLimiter {
    pub fn new() -> Self {
        RateLimiter {
            window: Mutex::new(VecDeque::new()),
        }
    }

    pub fn allow_at(&self, now: Instant) -> bool {
        let mut w = self.window.lock().unwrap();
        while w.front().is_some_and(|t| now.duration_since(*t) >= WINDOW) {
            w.pop_front();
        }
        if w.len() >= LIMIT {
            return false;
        }
        w.push_back(now);
        true
    }

    pub fn allow(&self) -> bool {
        self.allow_at(Instant::now())
    }
}

/// Turns the extension's preset into a choice. Anything it leaves out comes from the user's
/// defaults in Settings. A format from the wrong list is refused, never guessed.
pub fn choice_from_preset(preset: Option<&Preset>, d: &Downloads) -> Result<Choice, String> {
    let Some(p) = preset else {
        return Ok(default_choice(d));
    };
    let kind = match p.kind {
        PresetKind::Video => DefaultKind::Video,
        PresetKind::Audio => DefaultKind::Audio,
    };
    let mut basis = d.clone();
    basis.default_kind = kind;
    let mut choice = default_choice(&basis);
    if let Some(f) = p.format.as_deref() {
        let allowed: &[&str] = if p.kind == PresetKind::Video {
            &VIDEO_FORMATS
        } else {
            &AUDIO_FORMATS
        };
        if !allowed.contains(&f) {
            return Err("That format isn't available for this type.".into());
        }
        choice.container = Container::parse(f).unwrap_or(Container::Original);
    }
    choice.height = match p.kind {
        PresetKind::Video => p.height,
        PresetKind::Audio => None,
    };
    choice.validate()?;
    Ok(choice)
}

/// The extension only runs on a few sites, and the app double-checks: the link must be a normal
/// web link to a site NOMNOM knows.
pub fn check_enqueue_url(url: &str) -> Result<(), (ErrorCode, String)> {
    let parsed = validate_url(url).map_err(|e| (ErrorCode::Invalid, e.to_string()))?;
    if is_probably_supported(&parsed) {
        Ok(())
    } else {
        Err((
            ErrorCode::Unsupported,
            "NomNom doesn't download from this site.".into(),
        ))
    }
}

fn outcome_ids(out: &EnqueueOutcome) -> (String, bool) {
    match out {
        EnqueueOutcome::Queued { job_id } => (job_id.clone(), false),
        EnqueueOutcome::AlreadyActive { job_id }
        | EnqueueOutcome::AlreadyDownloaded { job_id, .. } => (job_id.clone(), true),
    }
}

/// Handles one validated request from the extension.
pub fn handle(app: &AppHandle, request: Request) -> Response {
    let state = app.state::<AppState>();
    let id = request.id.clone();
    if !state.rate_limiter.allow() {
        return Response::err(
            id,
            ErrorCode::RateLimited,
            "Too many requests. Wait a moment.",
        );
    }
    match request.body {
        Body::Ping => {
            let mut r = Response::ok(id);
            r.app = Some(env!("CARGO_PKG_VERSION").to_string());
            r
        }
        Body::Enqueue { url, preset } => {
            if let Err((code, msg)) = check_enqueue_url(&url) {
                return Response::err(id, code, msg);
            }
            let d = state.settings.get().downloads;
            let choice = match choice_from_preset(preset.as_ref(), &d) {
                Ok(c) => c,
                Err(m) => return Response::err(id, ErrorCode::Invalid, m),
            };
            match enqueue_with_defaults(&state, url, choice, None, false) {
                Ok(out) => {
                    let (job, duplicate) = outcome_ids(&out);
                    let mut r = Response::ok(id);
                    r.tasks = Some(vec![job]);
                    r.duplicate = Some(duplicate);
                    r
                }
                Err(m) => Response::err(id, ErrorCode::Invalid, m),
            }
        }
        Body::EnqueueDirect { source, media } => {
            let d = state.settings.get().downloads;
            let dest_dir = d
                .output_dir
                .clone()
                .unwrap_or_else(|| state.paths.downloads.clone());
            let req = DirectRequest {
                source,
                media,
                dest_dir,
                sort: d.sort_into_folders,
                allow_duplicates: d.allow_duplicates,
            };
            match state.downloads.enqueue_direct(req) {
                Ok(outs) => {
                    let ids: Vec<(String, bool)> = outs.iter().map(outcome_ids).collect();
                    let mut r = Response::ok(id);
                    r.duplicate = Some(!ids.is_empty() && ids.iter().all(|(_, dup)| *dup));
                    r.tasks = Some(ids.into_iter().map(|(i, _)| i).collect());
                    r
                }
                Err(e) => Response::err(id, ErrorCode::Invalid, e.to_string()),
            }
        }
        Body::OpenInApp { url } => {
            if let Err((code, msg)) = check_enqueue_url(&url) {
                return Response::err(id, code, msg);
            }
            crate::windows::open_main(app);
            let _ = app.emit("browser://open", url);
            Response::ok(id)
        }
    }
}

// ---- the connection between Firefox and NOMNOM (Settings, Browser) ----

pub fn integration_file(state: &AppState) -> PathBuf {
    state.paths.data_dir.join("integration.json")
}

/// Tells the host program where NOMNOM is and whether it may start it. The host has no other way
/// to learn this (it doesn't read settings.json).
pub fn write_integration(state: &AppState) {
    let launch = state.settings.get().browser.launch_from_browser;
    let exe = std::env::current_exe().ok();
    let body = serde_json::json!({ "exe": exe, "launchFromBrowser": launch });
    let file = integration_file(state);
    let tmp = file.with_extension("json.tmp");
    let ok = std::fs::write(&tmp, body.to_string()).and_then(|_| std::fs::rename(&tmp, &file));
    if let Err(e) = ok {
        tracing::warn!("couldn't write integration.json: {e}");
    }
}

fn host_exe() -> Option<PathBuf> {
    let p = std::env::current_exe()
        .ok()?
        .parent()?
        .join("nomnom-native-host.exe");
    p.is_file().then_some(p)
}

fn manifest_path(state: &AppState) -> PathBuf {
    state.paths.data_dir.join("com.nomnom.host.json")
}

#[derive(serde::Serialize)]
pub struct BrowserStatus {
    /// "not_connected", "needs_repair" or "connected"
    state: &'static str,
    detail: Option<String>,
    helper_found: bool,
    launch_from_browser: bool,
}

#[tauri::command]
pub fn browser_status(state: State<'_, AppState>) -> BrowserStatus {
    use nomnom_win::native_host::{status, Status, FIREFOX_ROOT};
    let (s, detail) = match status(FIREFOX_ROOT) {
        Status::NotRegistered => ("not_connected", None),
        Status::Broken(why) => ("needs_repair", Some(why)),
        Status::Ready => ("connected", None),
    };
    BrowserStatus {
        state: s,
        detail,
        helper_found: host_exe().is_some(),
        launch_from_browser: state.settings.get().browser.launch_from_browser,
    }
}

/// Connects Firefox. Safe to run again: it also repairs a damaged setup.
pub fn connect(state: &AppState) -> Result<(), String> {
    let host =
        host_exe().ok_or("The helper program (nomnom-native-host.exe) isn't next to NomNom.")?;
    write_integration(state);
    nomnom_win::native_host::install(
        &host,
        &manifest_path(state),
        nomnom_win::native_host::FIREFOX_ROOT,
    )
    .map_err(|e| e.to_string())
}

pub fn disconnect(state: &AppState) -> Result<(), String> {
    nomnom_win::native_host::uninstall(&manifest_path(state), nomnom_win::native_host::FIREFOX_ROOT)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn browser_connect(state: State<'_, AppState>) -> Result<(), String> {
    connect(&state)
}

#[tauri::command]
pub fn browser_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    disconnect(&state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::downloads::subfolder_for;
    use nomnom_downloads::Kind;

    fn defaults() -> Downloads {
        Downloads {
            default_kind: DefaultKind::Video,
            default_video: "mp4".into(),
            default_audio: "mp3".into(),
            ..Downloads::default()
        }
    }

    fn preset(kind: PresetKind, format: Option<&str>, height: Option<u32>) -> Preset {
        Preset {
            kind,
            format: format.map(str::to_string),
            height,
        }
    }

    #[test]
    fn no_preset_means_the_users_defaults() {
        let c = choice_from_preset(None, &defaults()).unwrap();
        assert_eq!(
            (c.kind, c.container, c.height),
            (Kind::Video, Container::Mp4, None)
        );
        let audio_default = Downloads {
            default_kind: DefaultKind::Audio,
            ..defaults()
        };
        let c = choice_from_preset(None, &audio_default).unwrap();
        assert_eq!((c.kind, c.container), (Kind::Audio, Container::Mp3));
    }

    #[test]
    fn a_preset_picks_the_type_and_fills_the_format_from_defaults() {
        let c =
            choice_from_preset(Some(&preset(PresetKind::Audio, None, None)), &defaults()).unwrap();
        assert_eq!((c.kind, c.container), (Kind::Audio, Container::Mp3));
        let c = choice_from_preset(
            Some(&preset(PresetKind::Video, None, Some(720))),
            &defaults(),
        )
        .unwrap();
        assert_eq!(
            (c.kind, c.container, c.height),
            (Kind::Video, Container::Mp4, Some(720))
        );
    }

    #[test]
    fn an_explicit_format_is_used_when_it_fits_the_type() {
        let c = choice_from_preset(
            Some(&preset(PresetKind::Audio, Some("flac"), None)),
            &defaults(),
        )
        .unwrap();
        assert_eq!(c.container, Container::Flac);
        let c = choice_from_preset(
            Some(&preset(PresetKind::Video, Some("webm"), None)),
            &defaults(),
        )
        .unwrap();
        assert_eq!(c.container, Container::Webm);
    }

    #[test]
    fn formats_from_the_wrong_list_or_nonsense_are_refused() {
        assert!(choice_from_preset(
            Some(&preset(PresetKind::Video, Some("mp3"), None)),
            &defaults()
        )
        .is_err());
        assert!(choice_from_preset(
            Some(&preset(PresetKind::Audio, Some("mp4"), None)),
            &defaults()
        )
        .is_err());
        assert!(choice_from_preset(
            Some(&preset(PresetKind::Audio, Some("exe"), None)),
            &defaults()
        )
        .is_err());
    }

    #[test]
    fn audio_ignores_a_height_instead_of_failing() {
        let c = choice_from_preset(
            Some(&preset(PresetKind::Audio, None, Some(1080))),
            &defaults(),
        )
        .unwrap();
        assert_eq!(c.height, None);
    }

    #[test]
    fn only_known_sites_and_valid_links_are_accepted() {
        assert!(check_enqueue_url("https://www.youtube.com/watch?v=abc").is_ok());
        assert!(check_enqueue_url("https://x.com/a/status/1").is_ok());
        assert!(check_enqueue_url("https://www.instagram.com/reel/abc/").is_ok());
        assert!(check_enqueue_url("https://www.reddit.com/r/x/comments/1/t/").is_ok());
        assert_eq!(
            check_enqueue_url("https://example.com/v").unwrap_err().0,
            ErrorCode::Unsupported
        );
        assert_eq!(
            check_enqueue_url("file:///C:/x.mp4").unwrap_err().0,
            ErrorCode::Invalid
        );
        assert_eq!(
            check_enqueue_url("http://localhost:1234/").unwrap_err().0,
            ErrorCode::Invalid
        );
    }

    #[test]
    fn the_rate_limiter_allows_a_burst_then_blocks_then_recovers() {
        let rl = RateLimiter::new();
        let t0 = Instant::now();
        for i in 0..LIMIT {
            assert!(
                rl.allow_at(t0 + Duration::from_millis(i as u64)),
                "message {i}"
            );
        }
        assert!(
            !rl.allow_at(t0 + Duration::from_secs(5)),
            "the 21st within 10 s is refused"
        );
        assert!(
            rl.allow_at(t0 + Duration::from_secs(11)),
            "allowed again once the window has passed"
        );
    }

    #[test]
    fn sorted_folder_helper_is_consistent_with_direct_media() {
        // direct downloads decide Images/Videos themselves; video/audio use subfolder_for.
        let d = Downloads::default();
        assert_eq!(
            subfolder_for(&d, &Choice::BEST_VIDEO).as_deref(),
            Some("Videos")
        );
    }
}
