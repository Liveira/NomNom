//! Windows-specific setup that touches the registry. Everything is per-user (HKCU), nothing needs
//! administrator rights, and NOMNOM removes only the keys and files it created itself.

pub mod native_host;
