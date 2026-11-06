use crate::state::AppState;
use crate::windows;
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

/// Handle to the disabled "status" row so task events can update it.
pub struct TrayStatus(pub MenuItem<Wry>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("open", "Open NomNom").build(app)?;
    let status = MenuItemBuilder::with_id("status", "Nothing downloading")
        .enabled(false)
        .build(app)?;
    let folder = MenuItemBuilder::with_id("folder", "Open Downloads folder").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit NomNom").build(app)?;
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

    app.manage(TrayStatus(status));
    Ok(())
}

/// Updates the status row and tooltip. Called only when task state changes, never on a timer.
pub fn set_status(app: &AppHandle, text: &str) {
    if let Some(status) = app.try_state::<TrayStatus>() {
        let _ = status.0.set_text(text);
    }
    if let Some(tray) = app.tray_by_id("main") {
        let tip = if text.starts_with("Nothing") {
            "NomNom".to_string()
        } else {
            format!("NomNom · {text}")
        };
        let _ = tray.set_tooltip(Some(tip));
    }
}
