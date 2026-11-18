//! Active foreground application and game usage tracking for NomNom Playtime.
//!
//! Only time spent with a window actively focused while the user is present (not idle/AFK and not
//! on a locked screen) is counted. Background processes, minimized windows, system sleep gaps, and
//! desktop shell chrome are ignored. Window titles and URLs are never stored on disk.

pub mod classify;
pub mod icons;
pub mod probe;
pub mod tracker;

pub use classify::{classify_app, AppCategory, AppIdentity, RawForeground};
pub use icons::{encode_icon_data_url, extract_app_meta, ExtractedMeta};
pub use probe::{ForegroundProbe, ForegroundSample};
pub use tracker::{ActiveTracker, HourBucket, LiveFocus, SessionFlush};
