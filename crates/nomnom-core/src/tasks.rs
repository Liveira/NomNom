//! Central task manager: per-category concurrency limits, cancellation, progress.

use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub type TaskId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: TaskId,
    pub module: String,
    pub category: String,
    pub title: String,
    /// Lets a module tie a task to its own record (for example a download job id).
    pub ref_id: Option<String>,
    pub state: TaskState,
    /// 0.0..=1.0 when known.
    pub progress: Option<f32>,
    pub speed_bps: Option<u64>,
    pub eta_secs: Option<u64>,
    pub detail: Option<String>,
}

/// Handle given to the running work so it can publish progress and observe cancellation.
#[derive(Clone)]
pub struct TaskHandle {
    id: TaskId,
    manager: TaskManager,
    pub cancel: CancellationToken,
}

impl TaskHandle {
    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn progress(&self, fraction: Option<f32>, speed_bps: Option<u64>, eta_secs: Option<u64>) {
        self.manager.modify(self.id, |t| {
            t.progress = fraction.map(|f| f.clamp(0.0, 1.0));
            t.speed_bps = speed_bps;
            t.eta_secs = eta_secs;
        });
    }

    pub fn detail(&self, text: impl Into<String>) {
        let text = text.into();
        self.manager.modify(self.id, |t| t.detail = Some(text));
    }
}

struct Inner {
    tasks: Mutex<HashMap<TaskId, (Task, CancellationToken)>>,
    limits: Mutex<HashMap<String, Arc<Semaphore>>>,
    events: broadcast::Sender<Task>,
    /// Where tasks run. Without it, `submit` must be called from inside a Tokio runtime.
    runtime: Option<tokio::runtime::Handle>,
}

#[derive(Clone)]
pub struct TaskManager {
    inner: Arc<Inner>,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManager {
    pub fn new() -> Self {
        Self::build(None)
    }

    /// Runs tasks on `runtime`, so `submit` can be called from any thread (for example from a
    /// synchronous UI command on the main thread).
    pub fn with_runtime(runtime: tokio::runtime::Handle) -> Self {
        Self::build(Some(runtime))
    }

    fn build(runtime: Option<tokio::runtime::Handle>) -> Self {
        let (events, _) = broadcast::channel(256);
        TaskManager {
            inner: Arc::new(Inner {
                tasks: Mutex::new(HashMap::new()),
                limits: Mutex::new(HashMap::new()),
                events,
                runtime,
            }),
        }
    }

    /// Sets how many tasks of `category` may run at once. Applies to tasks submitted afterwards.
    pub fn set_limit(&self, category: &str, max: usize) {
        self.inner
            .limits
            .lock()
            .unwrap()
            .insert(category.to_string(), Arc::new(Semaphore::new(max.max(1))));
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Task> {
        self.inner.events.subscribe()
    }

    pub fn list(&self) -> Vec<Task> {
        self.inner
            .tasks
            .lock()
            .unwrap()
            .values()
            .map(|(t, _)| t.clone())
            .collect()
    }

    pub fn get(&self, id: TaskId) -> Option<Task> {
        self.inner
            .tasks
            .lock()
            .unwrap()
            .get(&id)
            .map(|(t, _)| t.clone())
    }

    pub fn cancel(&self, id: TaskId) {
        if let Some((_, token)) = self.inner.tasks.lock().unwrap().get(&id) {
            token.cancel();
        }
    }

    /// Removes finished tasks from the in-memory list (history lives in the database).
    pub fn clear_finished(&self) {
        self.inner
            .tasks
            .lock()
            .unwrap()
            .retain(|_, (t, _)| matches!(t.state, TaskState::Queued | TaskState::Running));
    }

    fn modify(&self, id: TaskId, f: impl FnOnce(&mut Task)) {
        let snapshot = {
            let mut map = self.inner.tasks.lock().unwrap();
            map.get_mut(&id).map(|(t, _)| {
                f(t);
                t.clone()
            })
        };
        if let Some(t) = snapshot {
            let _ = self.inner.events.send(t);
        }
    }

    /// Queues `work`. It starts once a slot in `category` is free. The returned error string of a
    /// failed task becomes its `detail`.
    pub fn submit<F, Fut>(&self, module: &str, category: &str, title: &str, work: F) -> TaskId
    where
        F: FnOnce(TaskHandle) -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<(), String>> + Send + 'static,
    {
        self.submit_ref(module, category, title, None, work)
    }

    /// Like [`submit`](Self::submit), but tags the task with a module-owned reference id.
    pub fn submit_ref<F, Fut>(
        &self,
        module: &str,
        category: &str,
        title: &str,
        ref_id: Option<String>,
        work: F,
    ) -> TaskId
    where
        F: FnOnce(TaskHandle) -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<(), String>> + Send + 'static,
    {
        let id = Uuid::new_v4();
        let cancel = CancellationToken::new();
        let task = Task {
            id,
            module: module.to_string(),
            category: category.to_string(),
            title: title.to_string(),
            ref_id,
            state: TaskState::Queued,
            progress: None,
            speed_bps: None,
            eta_secs: None,
            detail: None,
        };
        self.inner
            .tasks
            .lock()
            .unwrap()
            .insert(id, (task.clone(), cancel.clone()));
        let _ = self.inner.events.send(task);

        let sem = self
            .inner
            .limits
            .lock()
            .unwrap()
            .entry(category.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(2)))
            .clone();
        let manager = self.clone();
        let handle = TaskHandle {
            id,
            manager: manager.clone(),
            cancel: cancel.clone(),
        };

        let runner = async move {
            let permit = tokio::select! {
                p = sem.acquire_owned() => p.ok(),
                _ = cancel.cancelled() => None,
            };
            let Some(_permit) = permit else {
                manager.modify(id, |t| t.state = TaskState::Cancelled);
                return;
            };
            manager.modify(id, |t| t.state = TaskState::Running);
            let outcome = tokio::select! {
                r = work(handle) => Some(r),
                _ = cancel.cancelled() => None,
            };
            match outcome {
                Some(Ok(())) => manager.modify(id, |t| {
                    t.state = TaskState::Done;
                    t.progress = Some(1.0);
                    t.speed_bps = None;
                    t.eta_secs = None;
                }),
                Some(Err(msg)) => manager.modify(id, |t| {
                    t.state = TaskState::Failed;
                    t.detail = Some(msg);
                }),
                None => manager.modify(id, |t| t.state = TaskState::Cancelled),
            }
        };
        match &self.inner.runtime {
            Some(rt) => {
                rt.spawn(runner);
            }
            None => {
                tokio::spawn(runner);
            }
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    async fn wait_for(m: &TaskManager, id: TaskId, want: TaskState) {
        for _ in 0..200 {
            if m.get(id).map(|t| t.state) == Some(want) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!(
            "task never reached {want:?}; is {:?}",
            m.get(id).map(|t| t.state)
        );
    }

    #[tokio::test]
    async fn respects_concurrency_limit() {
        let m = TaskManager::new();
        m.set_limit("dl", 2);
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut ids = vec![];
        for _ in 0..6 {
            let (running, peak) = (running.clone(), peak.clone());
            ids.push(m.submit("t", "dl", "job", move |_| async move {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(30)).await;
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            }));
        }
        for id in ids {
            wait_for(&m, id, TaskState::Done).await;
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn cancels_running_and_queued_tasks() {
        let m = TaskManager::new();
        m.set_limit("c", 1);
        let running = m.submit("t", "c", "long", |_| async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            Ok(())
        });
        let queued = m.submit("t", "c", "waiting", |_| async { Ok(()) });
        wait_for(&m, running, TaskState::Running).await;
        m.cancel(queued);
        wait_for(&m, queued, TaskState::Cancelled).await;
        m.cancel(running);
        wait_for(&m, running, TaskState::Cancelled).await;
    }

    /// Regression: a synchronous UI command runs on the main thread, outside any runtime.
    /// Submitting from there used to panic with "there is no reactor running".
    #[test]
    fn submit_works_from_a_thread_without_a_runtime() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let m = TaskManager::with_runtime(rt.handle().clone());
        let id = std::thread::spawn({
            let m = m.clone();
            move || m.submit("t", "x", "from plain thread", |_| async { Ok(()) })
        })
        .join()
        .unwrap();
        rt.block_on(wait_for(&m, id, TaskState::Done));
    }

    #[tokio::test]
    async fn failure_is_recorded_with_message() {
        let m = TaskManager::new();
        let id = m.submit("t", "x", "bad", |_| async { Err("boom".to_string()) });
        wait_for(&m, id, TaskState::Failed).await;
        assert_eq!(m.get(id).unwrap().detail.as_deref(), Some("boom"));
    }
}
