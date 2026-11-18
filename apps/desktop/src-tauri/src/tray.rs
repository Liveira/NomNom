use crate::i18n::{self, Lang};
use crate::state::AppState;
use crate::windows;
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

/// Handles to the menu rows, so task events and a language change can update them.
pub struct TrayStatus {
    pub open: MenuItem<Wry>,
    pub status: MenuItem<Wry>,
    pub folder: MenuItem<Wry>,
    pub quit: MenuItem<Wry>,
}

fn lang(app: &AppHandle) -> Lang {
    app.try_state::<AppState>()
        .map(|s| i18n::lang_of(&s.settings.get()))
        .unwrap_or(Lang::En)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let l = lang(app);
    let open = MenuItemBuilder::with_id("open", i18n::tr(l, "Open NomNom")).build(app)?;
    let status = MenuItemBuilder::with_id("status", i18n::tr(l, "Nothing downloading"))
        .enabled(false)
        .build(app)?;
    let folder =
        MenuItemBuilder::with_id("folder", i18n::tr(l, "Open Downloads folder")).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", i18n::tr(l, "Quit NomNom")).build(app)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &sep1, &status, &folder, &sep2, &quit])
        .build()?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("NomNom")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => windows::open_main(app),
            "folder" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = app
                        .opener()
                        .open_path(state.paths.downloads.to_string_lossy(), None::<&str>);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::open_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;

    app.manage(TrayStatus {
        open,
        status,
        folder,
        quit,
    });
    Ok(())
}

/// Puts the tray menu in the language now chosen in Settings.
pub fn relabel(app: &AppHandle) {
    let l = lang(app);
    if let Some(t) = app.try_state::<TrayStatus>() {
        let _ = t.open.set_text(i18n::tr(l, "Open NomNom"));
        let _ = t.folder.set_text(i18n::tr(l, "Open Downloads folder"));
        let _ = t.quit.set_text(i18n::tr(l, "Quit NomNom"));
        let tasks = app.state::<AppState>().tasks.list();
        let (text, idle) = crate::events::status_line(l, &tasks);
        set_status(app, &text, idle);
    }
}

/// Updates the status row and tooltip. Called only when task state changes, never on a timer.
pub fn set_status(app: &AppHandle, text: &str, idle: bool) {
    if let Some(t) = app.try_state::<TrayStatus>() {
        let _ = t.status.set_text(text);
    }
    if let Some(tray) = app.tray_by_id("main") {
        let tip = if idle {
            "NomNom".to_string()
        } else {
            format!("NomNom · {text}")
        };
        let _ = tray.set_tooltip(Some(tip));
    }
}
