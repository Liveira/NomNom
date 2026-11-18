//! Disk analysis for NomNom: drives, a fast scanner, an in-memory index and the questions asked of
//! it. Windows only. No Tauri code lives here.

pub mod analyze;
pub mod batch;
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
