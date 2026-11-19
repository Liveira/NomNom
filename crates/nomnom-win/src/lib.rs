//! Windows-specific setup that touches the registry. Everything is per-user (HKCU), nothing needs
//! administrator rights, and NOMNOM removes only the keys and files it created itself.
//!
//! Compiled as an empty crate on non-Windows hosts so `cargo test --workspace` and
//! `cargo clippy --workspace` succeed cleanly on Linux as well.

#[cfg(windows)]
pub mod autostart;
#[cfg(windows)]
pub mod clipboard;
#[cfg(windows)]
pub mod memory;
#[cfg(windows)]
pub mod native_host;
#[cfg(windows)]
pub mod shell_menu;
#[cfg(windows)]
pub mod sparse;
#[cfg(windows)]
pub mod sparse_manifest;
