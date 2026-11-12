//! Disk analysis for NomNom: drives, a fast scanner, an in-memory index and the questions asked of
//! it. Windows only. No Tauri code lives here.

pub mod analyze;
pub mod drives;
pub mod index;
pub mod ops;
pub mod protect;
pub mod scan;

pub use index::{Index, NodeId, NodeView};
pub use scan::{ScanOptions, Session, State};
