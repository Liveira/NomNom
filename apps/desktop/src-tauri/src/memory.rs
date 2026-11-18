//! Keeps NOMNOM's memory small while nobody is looking at it.
//!
//! Every few seconds a watcher checks the window. When the window is hidden, minimised or behind
//! another one, the memory NOMNOM and its web view are not using is given back to the system
//! (Windows: the working set is trimmed; Linux: the allocator returns its free pages). While the
//! window is in use this happens only now and then. Nothing is lost: anything used again simply
//! comes back, and nothing about how NOMNOM looks or works changes.

use crate::windows::MAIN;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const CHECK_EVERY: Duration = Duration::from_secs(10);
/// Not in front: give memory back after this long since the last time.
const BACKGROUND_EVERY: Duration = Duration::from_secs(20);
/// In front and in use: only now and then.
const FOREGROUND_EVERY: Duration = Duration::from_secs(90);

/// What the window is doing, which decides how soon memory is given back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// No window at all (NOMNOM is in the tray).
    Closed,
    /// Minimised, hidden or behind another window.
    Background,
    /// In front.
    Foreground,
}

/// How long to wait since the last trim before trimming again.
pub fn wait_for(view: View) -> Duration {
    match view {
        View::Closed | View::Background => BACKGROUND_EVERY,
        View::Foreground => FOREGROUND_EVERY,
    }
}

fn view_of(app: &AppHandle) -> View {
    let Some(win) = app.get_webview_window(MAIN) else {
        return View::Closed;
    };
    let visible = win.is_visible().unwrap_or(true);
    let minimized = win.is_minimized().unwrap_or(false);
    let focused = win.is_focused().unwrap_or(true);
    if visible && !minimized && focused {
        View::Foreground
    } else {
        View::Background
    }
}

/// Gives back what is free. Returns how many processes were trimmed (Windows) or 1.
fn release() -> usize {
    #[cfg(windows)]
    {
        nomnom_win::memory::trim_family(std::process::id())
    }
    #[cfg(all(unix, target_env = "gnu"))]
    {
        // SAFETY: `malloc_trim` only returns free heap pages to the system.
        unsafe {
            libc::malloc_trim(0);
        }
        1
    }
    #[cfg(not(any(windows, all(unix, target_env = "gnu"))))]
    {
        0
    }
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // The start-up burst is over after a few seconds.
        tokio::time::sleep(Duration::from_secs(8)).await;
        let mut last: Option<Instant> = None;
        loop {
            let view = view_of(&app);
            if last.is_none_or(|t| t.elapsed() >= wait_for(view)) {
                let trimmed = tokio::task::spawn_blocking(release).await.unwrap_or(0);
                tracing::debug!(?view, trimmed, "gave unused memory back");
                last = Some(Instant::now());
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_waits_less_when_nobody_is_looking() {
        assert!(wait_for(View::Background) < wait_for(View::Foreground));
        assert_eq!(wait_for(View::Closed), wait_for(View::Background));
    }
}
