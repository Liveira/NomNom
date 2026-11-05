//! Media downloads: link checks, yt-dlp integration, the queue and its history.

pub mod analyze;
pub mod backend;
pub mod direct;
pub mod error;
pub mod files;
pub mod format;
pub mod manager;
pub mod progress;
pub mod url_check;

pub use backend::{MediaBackend, YtDlpBackend};
pub use error::DlError;
pub use format::{Choice, Container, Kind};
pub use manager::{
    DirectRequest, DownloadManager, EnqueueOutcome, EnqueueRequest, JobEvent, JobRow,
};
