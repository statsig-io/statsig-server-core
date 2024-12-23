use futures::future::join_all;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::runtime::{Builder, Handle, Runtime};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::log_d;
use crate::log_e;
use crate::StatsigErr;

const TAG: &str = stringify!(StatsigRuntime);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TaskId {
    tag: String,
    tokio_id: tokio::task::Id,
}

pub struct StatsigRuntime {
    pub runtime_handle: Handle,
    inner_runtime: Mutex<Option<Runtime>>,
    spawned_tasks: Arc<Mutex<HashMap<TaskId, JoinHandle<()>>>>,
    shutdown_notify: Arc<Notify>,
}

impl StatsigRuntime {
    pub fn get_runtime() -> Arc<StatsigRuntime> {
        let (opt_runtime, runtime_handle) = create_runtime_if_required();

        let shutdown_notify = Notify::new();
        Arc::new(StatsigRuntime {
            inner_runtime: Mutex::new(opt_runtime),
            runtime_handle,
            spawned_tasks: Arc::new(Mutex::new(HashMap::new())),
            shutdown_notify: Arc::new(shutdown_notify),
        })
    }

    pub fn get_handle(&self) -> Handle {
        self.runtime_handle.clone()
    }

    pub fn get_num_active_tasks(&self) -> usize {
        match self.spawned_tasks.lock() {
            Ok(lock) => lock.len(),
            Err(e) => {
                log_e!(TAG, "Failed to lock spawned tasks {}", e);
                0
            }
        }
    }

    pub fn shutdown(&self, timeout: Duration) {
        self.shutdown_notify.notify_waiters();

        if let Ok(mut lock) = self.spawned_tasks.lock() {
            for (_, task) in lock.drain() {
                task.abort();
            }
        }

        if let Ok(mut lock) = self.inner_runtime.lock() {
            if let Some(runtime) = lock.take() {
                log_d!(
                    TAG,
                    "Shutting down Statsig runtime with timeout: {:?}",
                    timeout
                );
                if timeout.as_millis() > 0 {
                    runtime.shutdown_timeout(timeout);
                } else {
                    runtime.shutdown_background();
                }
            }
        }
    }

    pub fn shutdown_immediate(&self) {
        self.shutdown(Duration::from_millis(0));
    }

    pub fn spawn<F, Fut>(&self, tag: &str, task: F) -> tokio::task::Id
    where
        F: FnOnce(Arc<Notify>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let tag_string = tag.to_string();
        let shutdown_notify = self.shutdown_notify.clone();
        let spawned_tasks = self.spawned_tasks.clone();

        log_d!(TAG, "Spawning task {}", tag);

        let handle = self.runtime_handle.spawn(async move {
            let task_id = tokio::task::id();
            log_d!(TAG, "Executing task {}.{}", tag_string, task_id);
            task(shutdown_notify).await;
            remove_join_handle_with_id(spawned_tasks, tag_string, &task_id);
        });

        self.insert_join_handle(tag, handle)
    }

    pub async fn await_tasks_with_tag(&self, tag: &str) {
        let mut handles = Vec::new();

        match self.spawned_tasks.lock() {
            Ok(mut lock) => {
                let keys: Vec<TaskId> = lock.keys().cloned().collect();
                for key in &keys {
                    if key.tag == tag {
                        handles.push(lock.remove(key).unwrap());
                    }
                }
            }
            Err(e) => {
                log_e!(TAG, "Failed to lock spawned tasks {}", e);
                return;
            }
        };

        join_all(handles).await;
    }

    pub async fn await_join_handle(
        &self,
        tag: &str,
        handle_id: &tokio::task::Id,
    ) -> Result<(), StatsigErr> {
        let task_id = TaskId {
            tag: tag.to_string(),
            tokio_id: *handle_id,
        };

        let handle = match self.spawned_tasks.lock() {
            Ok(mut lock) => match lock.remove(&task_id) {
                Some(handle) => handle,
                None => {
                    return Err(StatsigErr::ThreadFailure(
                        "No running task found".to_string(),
                    ));
                }
            },
            Err(e) => {
                log_e!(
                    TAG,
                    "An error occurred while getting join handle with id: {}: {}",
                    handle_id,
                    e.to_string()
                );
                return Err(StatsigErr::ThreadFailure(e.to_string()));
            }
        };

        handle
            .await
            .map_err(|e| StatsigErr::ThreadFailure(e.to_string()))?;

        Ok(())
    }

    fn insert_join_handle(&self, tag: &str, handle: JoinHandle<()>) -> tokio::task::Id {
        let handle_id = handle.id();
        let task_id = TaskId {
            tag: tag.to_string(),
            tokio_id: handle_id,
        };

        match self.spawned_tasks.lock() {
            Ok(mut lock) => {
                lock.insert(task_id, handle);
            }
            Err(e) => {
                log_e!(
                    TAG,
                    "An error occurred while inserting join handle: {}",
                    e.to_string()
                );
            }
        }

        handle_id
    }
}

fn remove_join_handle_with_id(
    spawned_tasks: Arc<Mutex<HashMap<TaskId, JoinHandle<()>>>>,
    tag: String,
    handle_id: &tokio::task::Id,
) {
    let task_id = TaskId {
        tag,
        tokio_id: *handle_id,
    };

    match spawned_tasks.lock() {
        Ok(mut lock) => {
            lock.remove(&task_id);
        }
        Err(e) => {
            log_e!(
                TAG,
                "An error occurred while removing join handle {}",
                e.to_string()
            );
        }
    }
}

fn create_runtime_if_required() -> (Option<Runtime>, Handle) {
    if let Ok(handle) = Handle::try_current() {
        log_d!(TAG, "Existing tokio runtime found");
        return (None, handle);
    }

    // todo: remove expects and return error
    let rt = Builder::new_multi_thread()
        .worker_threads(3)
        .thread_name("statsig")
        .enable_all()
        .build()
        .expect("Failed to find or create a tokio Runtime");

    let handle = rt.handle().clone();
    log_d!(TAG, "New tokio runtime created");
    (Some(rt), handle)
}

impl Drop for StatsigRuntime {
    fn drop(&mut self) {
        self.shutdown(Duration::from_secs(1));
    }
}
