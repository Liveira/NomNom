//! The translator. Select some text anywhere and press a shortcut: it is translated with DeepL and
//! either put in place of what you selected, or shown in a small window next to the pointer.
//!
//! The DeepL key is kept in its own file, locked to the Windows account (Linux: readable only by
//! the user), and never put in the settings the window reads. The selection is read by copying it
//! for a moment; whatever was on the clipboard before is put back afterwards.

use crate::state::AppState;
use crate::toast::{self, Toast};
use crate::windows::shared_webview_args;
use nomnom_core::translate::{self as rules, Language, Translation};
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;

type CmdResult<T> = Result<T, String>;

const LABEL: &str = "translate";
const POPUP_W: f64 = 440.0;
const POPUP_H: f64 = 330.0;

// ---- the key ---------------------------------------------------------------------------------------------

fn key_file(state: &AppState) -> std::path::PathBuf {
    state.paths.data_dir.join("deepl.key")
}

pub fn load_key(state: &AppState) -> Option<String> {
    let bytes = std::fs::read(key_file(state)).ok()?;
    #[cfg(windows)]
    let bytes = nomnom_win::secret::unprotect(&bytes)?;
    let key = String::from_utf8(bytes).ok()?;
    let key = key.trim().to_string();
    (!key.is_empty()).then_some(key)
}

fn save_key(state: &AppState, key: &str) -> Result<(), String> {
    #[cfg(windows)]
    let bytes =
        nomnom_win::secret::protect(key.as_bytes()).ok_or("Windows couldn't lock the key.")?;
    #[cfg(not(windows))]
    let bytes = key.as_bytes().to_vec();
    let path = key_file(state);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

// ---- DeepL --------------------------------------------------------------------------------------------------

/// The DeepL server for this key. A debug build can be pointed at a local test server.
fn base(key: &str) -> String {
    #[cfg(debug_assertions)]
    if let Ok(url) = std::env::var("NOMNOM_TEST_DEEPL") {
        if url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost") {
            return url;
        }
    }
    rules::endpoint(key).to_string()
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("NomNom")
        .build()
        .map_err(|e| e.to_string())
}

async fn call_deepl(key: &str, text: &str, target: &str) -> Result<Translation, String> {
    if text.chars().count() > rules::MAX_CHARS {
        return Err(format!(
            "Select less text: NomNom translates up to {} characters at once.",
            rules::MAX_CHARS
        ));
    }
    let url = format!("{}/v2/translate", base(key));
    let resp = client()?
        .post(url)
        .header("Authorization", format!("DeepL-Auth-Key {}", key.trim()))
        .json(&rules::request_body(text, target))
        .send()
        .await
        .map_err(|_| "Couldn't reach DeepL. Check your internet connection.".to_string())?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(rules::explain_status(status.as_u16()));
    }
    rules::parse_response(&body)
}

/// Characters used and allowed this month, when the key works.
async fn usage(key: &str) -> Result<(u64, u64), String> {
    let url = format!("{}/v2/usage", base(key));
    let resp = client()?
        .get(url)
        .header("Authorization", format!("DeepL-Auth-Key {}", key.trim()))
        .send()
        .await
        .map_err(|_| "Couldn't reach DeepL. Check your internet connection.".to_string())?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(rules::explain_status(status.as_u16()));
    }
    rules::parse_usage(&body)
        .ok_or_else(|| "DeepL sent an answer NomNom couldn't read.".to_string())
}

fn target_of(state: &AppState) -> &'static str {
    let s = state.settings.get();
    let ui = match crate::i18n::lang_of(&s) {
        crate::i18n::Lang::En => "en",
        crate::i18n::Lang::Pt => "pt",
        crate::i18n::Lang::Es => "es",
    };
    rules::target_for(&s.text.target, ui)
}

// ---- reading the selection ---------------------------------------------------------------------------------

/// What was selected, and what the clipboard held before.
struct Captured {
    selected: String,
    before: Option<String>,
}

#[cfg(windows)]
fn capture(app: &AppHandle) -> Result<Captured, String> {
    use nomnom_win::clipboard as keys;
    keys::wait_modifiers_released(900);
    let before = app.clipboard().read_text().ok();
    crate::clipboard::suppress(3500);
    let seq = keys::sequence();
    keys::press_copy();
    let start = std::time::Instant::now();
    while keys::sequence() == seq {
        if start.elapsed() > Duration::from_millis(900) {
            return Err("Select some text first.".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(40));
    let selected = app
        .clipboard()
        .read_text()
        .map_err(|_| "The selection isn't text.".to_string())?;
    if selected.trim().is_empty() {
        return Err("Select some text first.".into());
    }
    Ok(Captured { selected, before })
}

/// On Linux the selection is what the desktop calls the "primary" clipboard: whatever is
/// highlighted, with no copying. Reading it needs a small tool.
#[cfg(not(windows))]
fn capture(app: &AppHandle) -> Result<Captured, String> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let attempts: Vec<(&str, Vec<&str>)> = if wayland {
        vec![("wl-paste", vec!["--primary", "--no-newline"])]
    } else {
        vec![
            ("xclip", vec!["-o", "-selection", "primary"]),
            ("xsel", vec!["-p", "-o"]),
        ]
    };
    for (cmd, args) in attempts {
        if let Ok(out) = std::process::Command::new(cmd).args(args).output() {
            if out.status.success() {
                let selected = String::from_utf8_lossy(&out.stdout).into_owned();
                if !selected.trim().is_empty() {
                    return Ok(Captured {
                        selected,
                        before: app.clipboard().read_text().ok(),
                    });
                }
                return Err("Select some text first.".into());
            }
        }
    }
    Err(if wayland {
        "To translate selected text, install wl-clipboard (wl-paste).".into()
    } else {
        "To translate selected text, install xclip or xsel.".into()
    })
}

fn say(app: &AppHandle, kind: &str, title: &str, body: &str) {
    let lang = crate::i18n::lang_of(&app.state::<AppState>().settings.get());
    toast::show(
        app,
        Toast::new(
            kind,
            crate::i18n::tr(lang, title),
            crate::i18n::tr(lang, body),
        )
        .opens("text"),
    );
}

fn say_error(app: &AppHandle, message: &str) {
    let lang = crate::i18n::lang_of(&app.state::<AppState>().settings.get());
    toast::show(
        app,
        Toast::new(
            "error",
            crate::i18n::tr(lang, "Couldn't translate"),
            crate::i18n::tr(lang, message),
        )
        .opens("text"),
    );
}

/// Both shortcuts begin the same way: make sure it is set up, then read the selection.
fn begin(app: &AppHandle) -> Option<(String, Captured, &'static str)> {
    let state = app.state::<AppState>();
    if !state.settings.get().text.enabled {
        return None;
    }
    let Some(key) = load_key(&state) else {
        say_error(app, "Add your DeepL key in the Translator page first.");
        return None;
    };
    let target = target_of(&state);
    match capture(app) {
        Ok(c) => Some((key, c, target)),
        Err(e) => {
            say_error(app, &e);
            None
        }
    }
}

fn put_back(app: &AppHandle, before: Option<String>) {
    if let Some(text) = before {
        let _ = app.clipboard().write_text(text);
    }
}

/// Shortcut one: the selected text is replaced by its translation.
pub fn replace_selection(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let a = app.clone();
        let Ok(Some((key, captured, target))) =
            tokio::task::spawn_blocking(move || begin(&a)).await
        else {
            return;
        };
        match call_deepl(&key, &captured.selected, target).await {
            Ok(t) => {
                if rules::already_in(&t.detected, target)
                    && t.text.trim() == captured.selected.trim()
                {
                    put_back(&app, captured.before);
                    say(
                        &app,
                        "info",
                        "Already in that language",
                        "Nothing to change.",
                    );
                    return;
                }
                if app.clipboard().write_text(t.text).is_err() {
                    say_error(&app, "Couldn't use the clipboard.");
                    return;
                }
                #[cfg(windows)]
                {
                    let before = captured.before;
                    let app2 = app.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        std::thread::sleep(Duration::from_millis(60));
                        nomnom_win::clipboard::press_paste();
                        // The pasting program reads the clipboard a moment later.
                        std::thread::sleep(Duration::from_millis(450));
                        put_back(&app2, before);
                    })
                    .await;
                }
                #[cfg(not(windows))]
                say(
                    &app,
                    "success",
                    "Translated",
                    "It is on the clipboard: press Ctrl+V to paste it.",
                );
            }
            Err(e) => {
                put_back(&app, captured.before);
                say_error(&app, &e);
            }
        }
    });
}

// ---- the small window --------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Default)]
pub struct PopupData {
    pub original: String,
    pub translated: Option<String>,
    pub detected: String,
    pub target: String,
    pub error: Option<String>,
}

static POPUP: Mutex<Option<PopupData>> = Mutex::new(None);
/// The window that had the keyboard (to put the translation back into it).
static SOURCE: Mutex<isize> = Mutex::new(0);

/// Shortcut two: the translation is shown next to the pointer.
pub fn popup_selection(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(windows)]
        {
            *SOURCE.lock().unwrap() = nomnom_win::clipboard::foreground();
        }
        let a = app.clone();
        let Ok(Some((key, captured, target))) =
            tokio::task::spawn_blocking(move || begin(&a)).await
        else {
            return;
        };
        // The clipboard is back as it was straight away: only the window needs the text.
        put_back(&app, captured.before.clone());
        *POPUP.lock().unwrap() = Some(PopupData {
            original: captured.selected.clone(),
            target: target.to_string(),
            ..Default::default()
        });
        open_popup_window(&app);
        let result = call_deepl(&key, &captured.selected, target).await;
        {
            let mut guard = POPUP.lock().unwrap();
            if let Some(p) = guard.as_mut() {
                match result {
                    Ok(t) => {
                        p.translated = Some(t.text);
                        p.detected = t.detected;
                    }
                    Err(e) => p.error = Some(e),
                }
            }
        }
        let _ = app.emit("translate://done", ());
    });
}

fn open_popup_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.emit("translate://done", ());
        let _ = win.show();
        return;
    }
    let built = WebviewWindowBuilder::new(
        app,
        LABEL,
        WebviewUrl::App("panel.html?kind=translate".into()),
    )
    .title("NomNom translation")
    .inner_size(POPUP_W, POPUP_H)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false);
    match shared_webview_args(built).build() {
        Ok(win) => {
            place_near_pointer(app, &win);
            let watched = win.clone();
            win.on_window_event(move |event| {
                if let WindowEvent::Focused(false) = event {
                    let _ = watched.destroy();
                    *POPUP.lock().unwrap() = None;
                }
            });
        }
        Err(e) => {
            tracing::warn!("couldn't open the translation window: {e}");
            say_error(app, "Couldn't open the translation window.");
        }
    }
}

fn place_near_pointer(app: &AppHandle, win: &tauri::WebviewWindow) {
    let Ok(cursor) = app.cursor_position() else {
        return;
    };
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let (Some(m), Ok(size)) = (monitor, win.outer_size()) else {
        return;
    };
    let area = m.work_area();
    let (w, h) = (size.width as i32, size.height as i32);
    let mut x = cursor.x as i32 + 14;
    let mut y = cursor.y as i32 + 18;
    x = x.clamp(
        area.position.x,
        (area.position.x + area.size.width as i32 - w).max(area.position.x),
    );
    y = y.clamp(
        area.position.y,
        (area.position.y + area.size.height as i32 - h).max(area.position.y),
    );
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

#[tauri::command]
pub fn tr_popup_take() -> Option<PopupData> {
    POPUP.lock().unwrap().clone()
}

/// The page draws, then asks for the window to be shown.
#[tauri::command]
pub fn tr_popup_ready(app: AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[tauri::command]
pub fn tr_popup_close(app: AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.destroy();
    }
    *POPUP.lock().unwrap() = None;
}

/// "copy" puts the translation on the clipboard; "replace" also writes it over the selection.
#[tauri::command]
pub fn tr_popup_use(app: AppHandle, action: String) -> CmdResult<()> {
    let text = POPUP
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|p| p.translated.clone())
        .ok_or("There is no translation yet.")?;
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;
    if action == "replace" {
        let source = *SOURCE.lock().unwrap();
        tr_popup_close(app.clone());
        #[cfg(windows)]
        std::thread::spawn(move || nomnom_win::clipboard::paste_into(source));
        #[cfg(not(windows))]
        let _ = source;
    } else {
        tr_popup_close(app);
    }
    Ok(())
}

// ---- the page ---------------------------------------------------------------------------------------------------

#[derive(Serialize)]
pub struct KeyStatus {
    pub set: bool,
    pub used: Option<u64>,
    pub limit: Option<u64>,
    pub free: bool,
}

#[tauri::command]
pub async fn tr_key_status(state: State<'_, AppState>) -> CmdResult<KeyStatus> {
    let Some(key) = load_key(&state) else {
        return Ok(KeyStatus {
            set: false,
            used: None,
            limit: None,
            free: false,
        });
    };
    let free = key.ends_with(":fx");
    let (used, limit) = match usage(&key).await {
        Ok((u, l)) => (Some(u), Some(l)),
        Err(_) => (None, None),
    };
    Ok(KeyStatus {
        set: true,
        used,
        limit,
        free,
    })
}

/// Checks the key with DeepL, and keeps it only if it works.
#[tauri::command]
pub async fn tr_set_key(state: State<'_, AppState>, key: String) -> CmdResult<KeyStatus> {
    let key = key.trim().to_string();
    if key.len() < 10 || key.len() > 200 || key.chars().any(|c| c.is_whitespace()) {
        return Err("That doesn't look like a DeepL key.".into());
    }
    let (used, limit) = usage(&key).await?;
    save_key(&state, &key)?;
    Ok(KeyStatus {
        set: true,
        used: Some(used),
        limit: Some(limit),
        free: key.ends_with(":fx"),
    })
}

#[tauri::command]
pub fn tr_clear_key(state: State<'_, AppState>) -> CmdResult<()> {
    match std::fs::remove_file(key_file(&state)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn tr_languages() -> Vec<Language> {
    rules::LANGUAGES.to_vec()
}

/// Translates a text typed in the page (to try it out).
#[tauri::command]
pub async fn tr_translate(state: State<'_, AppState>, text: String) -> CmdResult<Translation> {
    let key = load_key(&state).ok_or("Add your DeepL key first.")?;
    let target = target_of(&state);
    call_deepl(&key, &text, target).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_popup_state_starts_empty_and_can_be_read_back() {
        *POPUP.lock().unwrap() = None;
        assert!(tr_popup_take().is_none());
        *POPUP.lock().unwrap() = Some(PopupData {
            original: "Olá".into(),
            target: "EN-US".into(),
            ..Default::default()
        });
        let p = tr_popup_take().unwrap();
        assert_eq!(p.original, "Olá");
        assert!(p.translated.is_none() && p.error.is_none());
        *POPUP.lock().unwrap() = None;
    }
}
