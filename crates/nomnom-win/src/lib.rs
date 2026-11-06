//! Windows-specific setup that touches the registry. Everything is per-user (HKCU), nothing needs
//! administrator rights, and NOMNOM removes only the keys and files it created itself.

pub mod autostart;
pub mod native_host;
pub mod shell_menu;
pub mod sparse;
pub mod sparse_manifest;
