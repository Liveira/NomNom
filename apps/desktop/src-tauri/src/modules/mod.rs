//! Built-in modules. Adding a tool means adding a file here and listing it in `register_all`.

use nomnom_core::module::{Module, ModuleRegistry};
use std::sync::Arc;

mod builtin;
pub mod downloads;

pub fn register_all(registry: &mut ModuleRegistry) {
    let modules: Vec<Arc<dyn Module>> = vec![
        Arc::new(builtin::Home),
        Arc::new(builtin::ToolsHub),
        Arc::new(builtin::Downloads),
        Arc::new(builtin::Activity),
        Arc::new(builtin::Backup),
        Arc::new(builtin::Monitor),
        Arc::new(builtin::Organizer),
        Arc::new(builtin::FileTools),
        Arc::new(builtin::Storage),
        Arc::new(builtin::ClipboardModule),
        Arc::new(builtin::TextModule),
        Arc::new(builtin::PhoneModule),
        Arc::new(builtin::Settings),
    ];
    for m in modules {
        registry.register(m);
    }
}
