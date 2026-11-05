//! NOMNOM core: shared infrastructure used by every feature module.

pub mod db;
pub mod error;
pub mod fs;
pub mod module;
pub mod paths;
pub mod process;
pub mod settings;
pub mod tasks;

pub use error::{Error, Result};
