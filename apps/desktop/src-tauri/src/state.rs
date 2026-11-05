use nomnom_core::{
    db::Db, module::ModuleRegistry, paths::Paths, settings::SettingsStore, tasks::TaskManager,
};
use nomnom_downloads::DownloadManager;
use nomnom_tools::Tools;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Everything shared across commands, the tray and background services.
pub struct AppState {
    pub paths: Paths,
    pub settings: Arc<SettingsStore>,
    /// The organizer journal (Phase 5) uses this directly; downloads go through their manager.
    #[allow(dead_code)]
    pub db: Arc<Db>,
    pub tasks: TaskManager,
    pub modules: Arc<ModuleRegistry>,
    pub tools: Tools,
    pub downloads: DownloadManager,
    /// Recent link analyses, so reopening the same link is instant.
    pub analysis_cache: Mutex<HashMap<String, (Instant, serde_json::Value)>>,
    pub started: Instant,
}
