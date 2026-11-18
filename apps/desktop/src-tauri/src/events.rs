//! Forwards task updates to the UI and the tray, coalesced so a burst of progress callbacks
//! becomes at most ~8 UI events per second. With no window open, nothing is emitted.

use crate::i18n::{self, Lang};
use crate::{state::AppState, tray, windows};
use nomnom_core::tasks::{Task, TaskState};
use std::collections::HashMap;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;

const TICK: Duration = Duration::from_millis(125);

#[cfg(test)]
pub fn status_text(tasks: &[Task]) -> String {
    status_line(Lang::En, tasks).0
}

/// The tray's status text in a language, and whether nothing is running.
pub fn status_line(lang: Lang, tasks: &[Task]) -> (String, bool) {
    let active: Vec<&Task> = tasks
        .iter()
        .filter(|t| matches!(t.state, TaskState::Running | TaskState::Queued))
        .collect();
    if active.is_empty() {
        return (i18n::tr(lang, "Nothing downloading"), true);
    }
    let known: Vec<f32> = active.iter().filter_map(|t| t.progress).collect();
    let percent = if known.is_empty() {
        None
    } else {
        let avg = known.iter().sum::<f32>() / known.len() as f32;
        Some((avg * 100.0).round() as u32)
    };
    (i18n::active_count(lang, active.len(), percent), false)
}

pub fn spawn_forwarder(app: AppHandle) {
    let mut rx = app.state::<AppState>().tasks.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut last_status = String::new();
        loop {
            let mut batch: HashMap<_, Task> = HashMap::new();
            match rx.recv().await {
                Ok(t) => {
                    batch.insert(t.id, t);
                }
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return,
            }
            tokio::time::sleep(TICK).await;
            while let Ok(t) = rx.try_recv() {
                batch.insert(t.id, t);
            }

            if app.get_webview_window(windows::MAIN).is_some() {
                let updates: Vec<&Task> = batch.values().collect();
                let _ = app.emit("tasks://batch", updates);
            }
            let lang = i18n::lang_of(&app.state::<AppState>().settings.get());
            let (text, idle) = status_line(lang, &app.state::<AppState>().tasks.list());
            if text != last_status {
                tray::set_status(&app, &text, idle);
                last_status = text;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn task(state: TaskState, progress: Option<f32>) -> Task {
        Task {
            id: Uuid::new_v4(),
            module: "t".into(),
            category: "c".into(),
            title: "x".into(),
            ref_id: None,
            state,
            progress,
            speed_bps: None,
            eta_secs: None,
            detail: None,
        }
    }

    #[test]
    fn idle_and_active_status_text() {
        assert_eq!(status_text(&[]), "Nothing downloading");
        assert_eq!(
            status_text(&[task(TaskState::Done, Some(1.0))]),
            "Nothing downloading"
        );
        let two = [
            task(TaskState::Running, Some(0.2)),
            task(TaskState::Queued, None),
        ];
        assert_eq!(status_text(&two), "2 active · 20%");
        assert_eq!(status_text(&[task(TaskState::Queued, None)]), "1 active");
    }
}
