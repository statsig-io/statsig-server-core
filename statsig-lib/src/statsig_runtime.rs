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

pub struct StatsigRuntime {
    pub runtime_handle: Handle,
    inner_runtime: Mutex<Option<Runtime>>,
    spawned_tasks: Mutex<HashMap<tokio::task::Id, JoinHandle<()>>>,
    shutdown_notify: Arc<Notify>,
}

impl StatsigRuntime {
    pub fn get_runtime() -> Arc<StatsigRuntime> {
        let (opt_runtime, runtime_handle) = create_runtime_if_required();

        let shutdown_notify = Notify::new();
        Arc::new(StatsigRuntime {
            inner_runtime: Mutex::new(opt_runtime),
            runtime_handle,
            spawned_tasks: Mutex::new(HashMap::new()),
            shutdown_notify: Arc::new(shutdown_notify),
        })
    }

    pub fn get_handle(&self) -> Handle {
        self.runtime_handle.clone()
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
        let tag = tag.to_string();
        log_d!(TAG, "Spawning task {}", tag);
        let shutdown_notify = self.shutdown_notify.clone();
        let handle = self.runtime_handle.spawn(async move {
            log_d!(TAG, "Executing task {}", tag);
            task(shutdown_notify).await;
        });

        self.insert_join_handle(handle)
    }

    pub fn spawn_blocking<F, R>(&self, task: F) -> tokio::task::Id
    where
        F: FnOnce(Arc<Notify>) -> R + Send + 'static,
        R: Send + 'static,
    {
        let shutdown_notify = self.shutdown_notify.clone();
        let handle = self.runtime_handle.spawn_blocking(move || {
            task(shutdown_notify);
        });

        self.insert_join_handle(handle)
    }

    pub async fn await_join_handle(&self, handle_id: &tokio::task::Id) -> Result<(), StatsigErr> {
        let handle = match self.spawned_tasks.lock() {
            Ok(mut lock) => match lock.remove(handle_id) {
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

    fn insert_join_handle(&self, handle: JoinHandle<()>) -> tokio::task::Id {
        let handle_id = handle.id();

        match self.spawned_tasks.lock() {
            Ok(mut lock) => {
                lock.insert(handle_id, handle);
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
