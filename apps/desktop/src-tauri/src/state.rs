use nomnom_core::{
    db::Db, module::ModuleRegistry, paths::Paths, settings::SettingsStore, tasks::TaskManager,
};
use std::sync::Arc;
use std::time::Instant;

/// Everything shared across commands, the tray and background services.
pub struct AppState {
    pub paths: Paths,
    pub settings: Arc<SettingsStore>,
    /// Not read yet: downloads (Phase 2) and the organizer journal (Phase 5) use it.
    #[allow(dead_code)]
    pub db: Arc<Db>,
    pub tasks: TaskManager,
    pub modules: Arc<ModuleRegistry>,
    pub started: Instant,
}
