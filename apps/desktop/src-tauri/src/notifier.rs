//! Native notifications for finished and failed downloads. Bursts are merged into one toast, and
//! nothing is shown while the main window has focus.

use crate::i18n::{self, Lang};
use crate::{state::AppState, windows};
use nomnom_downloads::JobEvent;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::broadcast::error::RecvError;

const MERGE_WINDOW: Duration = Duration::from_secs(4);

#[cfg(test)]
pub fn summary(done: &[String], failed: &[String]) -> Option<(String, String)> {
    summary_in(Lang::En, done, failed)
}

pub fn summary_in(lang: Lang, done: &[String], failed: &[String]) -> Option<(String, String)> {
    let t = |s: &str| i18n::tr(lang, s);
    match (done.len(), failed.len()) {
        (0, 0) => None,
        (1, 0) => Some((t("Download finished"), done[0].clone())),
        (n, 0) => Some((t("Downloads finished"), i18n::n_done(lang, n))),
        (0, 1) => Some((t("Couldn't download"), failed[0].clone())),
        (0, n) => Some((t("Downloads failed"), i18n::n_failed(lang, n))),
        (d, f) => Some((t("Downloads finished"), i18n::done_and_failed(lang, d, f))),
    }
}

pub fn spawn(app: AppHandle) {
    let mut rx = app.state::<AppState>().downloads.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            let (mut done, mut failed) = (Vec::new(), Vec::new());
            let mut handle = |e: JobEvent, app: &AppHandle| {
                // The window refreshes its list from this signal.
                let _ = app.emit("downloads://changed", &e);
                match e.status.as_str() {
                    "done" => done.push(e.title),
                    "failed" => failed.push(e.error.unwrap_or(e.title)),
                    _ => {}
                }
            };
            match rx.recv().await {
                Ok(e) => handle(e, &app),
                Err(RecvError::Lagged(_)) => {
                    let _ = app.emit("downloads://changed", ());
                    continue;
                }
                Err(RecvError::Closed) => return,
            }
            let deadline = tokio::time::Instant::now() + MERGE_WINDOW;
            while let Ok(Ok(e)) = tokio::time::timeout_at(deadline, rx.recv()).await {
                handle(e, &app);
            }

            let enabled = app.state::<AppState>().settings.get().downloads.notify;
            let focused = app
                .get_webview_window(windows::MAIN)
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);
            if let (true, false, Some((title, body))) = (
                enabled,
                focused,
                summary_in(
                    i18n::lang_of(&app.state::<AppState>().settings.get()),
                    &done,
                    &failed,
                ),
            ) {
                let _ = app.notification().builder().title(title).body(body).show();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn merges_bursts_into_one_message() {
        assert_eq!(summary(&[], &[]), None);
        assert_eq!(summary(&v(&["Clip"]), &[]).unwrap().1, "Clip");
        assert_eq!(
            summary(&v(&["a", "b", "c"]), &[]).unwrap().1,
            "3 downloads are done."
        );
        assert_eq!(
            summary(&[], &v(&["Link expired"])).unwrap().0,
            "Couldn't download"
        );
        assert_eq!(
            summary(&v(&["a"]), &v(&["x", "y"])).unwrap().1,
            "1 done, 2 failed."
        );
    }
}
