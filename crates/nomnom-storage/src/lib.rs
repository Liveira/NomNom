//! Disk analysis for NomNom: drives and mounts, a fast parallel scanner, an in-memory index and
//! the questions asked of it. Supports Windows and Linux. No Tauri code lives here.

pub mod analyze;
pub mod batch;
pub mod cleanup;
pub mod drives;
pub mod dupes;
pub mod elevate;
pub mod index;
pub mod ntfs;
pub mod ops;
pub mod protect;
pub mod report;
pub mod scan;
pub mod search;
pub mod snapshot;

pub use index::{Index, NodeId, NodeView};
pub use scan::{ScanOptions, Session, State};
