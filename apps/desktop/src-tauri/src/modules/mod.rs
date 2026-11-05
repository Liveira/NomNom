//! Built-in modules. Adding a tool means adding a file here and listing it in `register_all`.

use nomnom_core::module::{Module, ModuleRegistry};
use std::sync::Arc;

mod builtin;
pub mod downloads;

pub fn register_all(registry: &mut ModuleRegistry) {
    let modules: Vec<Arc<dyn Module>> = vec![
        Arc::new(builtin::Home),
        Arc::new(builtin::Downloads),
        Arc::new(builtin::Settings),
    ];
    for m in modules {
        registry.register(m);
    }
}
