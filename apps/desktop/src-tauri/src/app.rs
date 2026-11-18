use crate::modules::downloads;
use crate::{
    activity, backup, browser, commands, events, filetools, focus, integrations, modules, notifier,
    organizer, pdftools, pipe_server, quick, shortcuts, state::AppState, storage, storage_cleanup,
    storage_history, storage_ops, storage_tools, tray, updater, windows,
};
use nomnom_core::{
    db::Db, module::ModuleRegistry, paths::Paths, settings::SettingsStore, tasks::TaskManager,
};
use nomnom_downloads::{DownloadManager, YtDlpBackend};
use nomnom_tools::Tools;
use std::sync::Arc;
use std::time::Instant;
use tauri::Manager;
use tauri::RunEvent;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

fn init_logging(paths: &Paths) -> Option<WorkerGuard> {
    // The first run has no logs folder yet; the rolling appender wants it to exist.
    let _ = std::fs::create_dir_all(paths.logs_dir());
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("nomnom")
        .filename_suffix("log")
        .max_log_files(7)
        .build(paths.logs_dir())
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    Some(guard)
}

fn wants_background(args: &[String]) -> bool {
    args.iter().any(|a| a == "--background")
}

fn wants_quick(args: &[String]) -> bool {
    args.iter().any(|a| a == "--quick")
}

pub fn run() {
    let started = Instant::now();
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--elevated-scan") {
        let pipe = args.get(pos + 1).map(String::as_str).unwrap_or("");
        let root = args.get(pos + 2).map(String::as_str).unwrap_or("");
        let opts = args.get(pos + 3).map(String::as_str).unwrap_or("{}");
        std::process::exit(nomnom_storage::elevate::run_elevated_helper(
            pipe, root, opts,
        ));
    }
    let background = wants_background(&args);

    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
        && (std::path::Path::new("/dev/nvidia0").exists()
            || std::path::Path::new("/proc/driver/nvidia/version").exists())
    {
        // Prevents blank WebKitGTK surfaces on Wayland compositors (Hyprland, Sway, Plasma, GNOME)
        // with proprietary NVIDIA drivers unless the user explicitly set the variable themselves.
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let paths = Paths::resolve().expect("couldn't locate the user's known folders");
    let _ = std::fs::create_dir_all(&paths.data_dir);
    let _log_guard = init_logging(&paths);
    tracing::info!(background, "NOMNOM starting");

    let settings = Arc::new(SettingsStore::load(&paths.settings_file()).expect("settings"));
    let db = Arc::new(Db::open(&paths.db_file()).expect("database"));
    let mut registry = ModuleRegistry::new();
    modules::register_all(&mut registry);

    // Tasks run on Tauri's runtime so synchronous commands on the main thread can queue work.
    let tauri::async_runtime::RuntimeHandle::Tokio(runtime) = tauri::async_runtime::handle();
    let tasks = TaskManager::with_runtime(runtime);
    tasks.set_limit(
        "downloads",
        settings.get().downloads.max_concurrent as usize,
    );
    let tools = Tools::new(paths.tools_dir());
    let downloads = DownloadManager::new(
        Arc::new(YtDlpBackend::new(tools.clone())),
        tasks.clone(),
        db.clone(),
    );
    downloads.recover();

    let state = AppState {
        paths,
        settings: settings.clone(),
        db: db.clone(),
        tasks,
        modules: Arc::new(registry),
        tools,
        downloads,
        analysis_cache: Default::default(),
        quick: Default::default(),
        quick_shown: Default::default(),
        shortcuts_registered: Default::default(),
        shortcut_errors: Default::default(),
        rate_limiter: crate::browser::RateLimiter::new(),
        journal: nomnom_organizer::journal::Journal::new(db.clone()),
        organizer: Default::default(),
        filetools: Default::default(),
        storage: Default::default(),
        activity: Default::default(),
        shell_note: Default::default(),
        started,
    };

    // Command-line setup, used by the installer and for repairs. These run and exit; no window.
    if args.iter().any(|a| a == "--install-integrations") {
        let result = integrations::install_all(&state, args.iter().any(|a| a == "--defaults"));
        tracing::info!(ok = result.is_ok(), "install integrations");
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    if args.iter().any(|a| a == "--uninstall-integrations") {
        let result = integrations::uninstall_all(&state);
        tracing::info!(ok = result.is_ok(), "uninstall integrations");
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }

    let app = tauri::Builder::default()
        // A second launch (shortcut, shell action, browser host) is forwarded to this instance.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(action) = filetools::parse_shell_args(&argv) {
                filetools::handle_shell_action(app, action);
            } else if wants_quick(&argv) {
                quick::trigger(app);
            } else if !wants_background(&argv) {
                windows::open_main(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state)
        .manage(updater::UpdateState::default())
        .manage(focus::FocusState::default())
        .manage(backup::BackupState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_modules,
            commands::get_settings,
            commands::update_settings,
            commands::list_tasks,
            commands::app_ready,
            commands::default_download_dir,
            commands::shortcuts_status,
            organizer::org_defaults,
            organizer::org_scan,
            organizer::org_execute,
            organizer::org_cancel,
            organizer::org_undo,
            organizer::org_history,
            filetools::ft_info,
            filetools::ft_inspect,
            filetools::ft_run,
            filetools::ft_cancel,
            filetools::ft_take_inbox,
            filetools::ft_open,
            storage::st_drives,
            storage::st_take_inbox,
            storage::st_scan_start,
            storage::st_scan_save,
            storage::st_scan_load,
            storage::st_scan_pause,
            storage::st_scan_resume,
            storage::st_scan_cancel,
            storage::st_rescan,
            storage::st_close,
            storage::st_sessions,
            storage::st_progress,
            storage::st_errors,
            storage::st_children,
            storage::st_node,
            storage::st_top_files,
            storage::st_top_folders,
            storage::st_stats,
            storage::st_paths,
            storage::st_treemap,
            storage_ops::st_op_plan,
            storage_ops::st_op_run,
            storage_ops::st_op_cancel,
            storage_ops::st_open,
            storage_tools::st_search,
            storage_tools::st_by_age,
            storage_tools::st_dupes_start,
            storage_tools::st_dupes_progress,
            storage_tools::st_dupes_groups,
            storage_tools::st_dupes_cancel,
            storage_tools::st_dupes_plan,
            storage_tools::st_rename_preview,
            storage_tools::st_rename_plan,
            storage_tools::st_export,
            storage_tools::st_export_paths,
            storage_history::st_snapshot_save,
            storage_history::st_snapshots,
            storage_history::st_snapshot_delete,
            storage_history::st_snapshot_compare,
            storage_history::st_export_comparison,
            storage_history::st_snapshot_trend,
            storage_history::st_schedule_status,
            storage_cleanup::st_cleanup_spots,
            pdftools::pdf_info,
            pdftools::pdf_merge,
            pdftools::pdf_split,
            downloads::dl_schedule,
            downloads::dl_scheduled,
            downloads::dl_unschedule,
            backup::bk_plans,
            backup::bk_save,
            backup::bk_delete,
            backup::bk_run,
            backup::bk_cancel,
            backup::bk_versions,
            backup::bk_tree,
            backup::bk_restore,
            backup::bk_verify,
            focus::focus_start,
            focus::focus_stop,
            focus::focus_status,
            updater::update_version,
            updater::update_known,
            updater::update_check,
            updater::update_install,
            activity::act_live,
            activity::act_overview,
            activity::act_app_detail,
            activity::act_update_app,
            activity::act_delete_app,
            integrations::integration_status,
            integrations::integration_repair,
            browser::browser_status,
            browser::browser_connect,
            browser::browser_disconnect,
            commands::set_shortcut,
            quick::quick_context,
            quick::quick_ready,
            quick::quick_close,
            downloads::dl_analyze,
            downloads::dl_default_choice,
            downloads::dl_enqueue,
            downloads::dl_list,
            downloads::dl_cancel,
            downloads::dl_pause,
            downloads::dl_resume,
            downloads::dl_retry,
            downloads::dl_remove,
            downloads::dl_clear_history,
            downloads::dl_open,
            downloads::tools_status,
            downloads::tools_install,
            downloads::tools_update_ytdlp,
        ])
        .setup(move |app| {
            tray::create(app.handle())?;
            events::spawn_forwarder(app.handle().clone());
            notifier::spawn(app.handle().clone());
            shortcuts::register_all(app.handle());
            browser::write_integration(&app.state::<AppState>());
            pipe_server::spawn(app.handle().clone());
            quick::apply_keep_ready(app.handle());
            organizer::recover_at_startup(&app.state::<AppState>().journal);
            organizer::apply_auto(app.handle());
            storage_history::spawn(app.handle().clone());
            activity::spawn(app.handle().clone());
            updater::spawn(app.handle().clone());
            focus::spawn(app.handle().clone());
            backup::spawn_scheduler(app.handle().clone());
            downloads::spawn_scheduler(app.handle().clone());
            downloads::apply_subtitles(&app.state::<AppState>());
            if let Some(action) = filetools::parse_shell_args(&args) {
                filetools::handle_shell_action(app.handle(), action);
            } else if wants_quick(&args) {
                quick::trigger(app.handle());
            } else if !background {
                windows::open_main(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build NOMNOM");

    app.run(move |app, event| match event {
        // Closing the last window destroys it. Keep the process alive in the tray if allowed.
        RunEvent::ExitRequested { api, code, .. } => {
            if code.is_none() && settings.get().general.close_to_tray {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => {
            activity::flush_on_exit(app);
        }
        _ => {}
    });
}
