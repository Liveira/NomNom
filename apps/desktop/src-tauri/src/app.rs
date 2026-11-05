use crate::modules::downloads;
use crate::{commands, events, modules, notifier, state::AppState, tray, windows};
use nomnom_core::{
    db::Db, module::ModuleRegistry, paths::Paths, settings::SettingsStore, tasks::TaskManager,
};
use nomnom_downloads::{DownloadManager, YtDlpBackend};
use nomnom_tools::Tools;
use std::sync::Arc;
use std::time::Instant;
use tauri::RunEvent;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

fn init_logging(paths: &Paths) -> Option<WorkerGuard> {
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

pub fn run() {
    let started = Instant::now();
    let args: Vec<String> = std::env::args().collect();
    let background = wants_background(&args);

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
        db,
        tasks,
        modules: Arc::new(registry),
        tools,
        downloads,
        analysis_cache: Default::default(),
        started,
    };

    let app = tauri::Builder::default()
        // A second launch (shortcut, shell action, browser host) is forwarded to this instance.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if !wants_background(&argv) {
                windows::open_main(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::list_modules,
            commands::get_settings,
            commands::update_settings,
            commands::list_tasks,
            commands::app_ready,
            commands::default_download_dir,
            downloads::dl_analyze,
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
            if !background {
                windows::open_main(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build NOMNOM");

    app.run(move |_app, event| {
        // Closing the last window destroys it. Keep the process alive in the tray if allowed.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() && settings.get().general.close_to_tray {
                api.prevent_exit();
            }
        }
    });
}
