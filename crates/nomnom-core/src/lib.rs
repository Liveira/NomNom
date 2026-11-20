//! NOMNOM core: shared infrastructure used by every feature module.

pub mod actions;
pub mod archive;
pub mod clipboard;
pub mod compare;
pub mod db;
pub mod error;
pub mod filesearch;
pub mod folder_rules;
pub mod fs;
pub mod game;
pub mod hardware;
pub mod launcher;
pub mod links;
pub mod module;
pub mod paths;
pub mod phone;
pub mod process;
pub mod qr;
pub mod reminders;
pub mod rename;
pub mod settings;
pub mod snippets;
pub mod startup;
pub mod tasks;
pub mod translate;

pub use error::{Error, Result};
