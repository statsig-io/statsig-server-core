use crate::statsig_err::StatsigErr;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::task;
use tokio::time::{interval_at, Instant};

pub trait BackgroundTask: Send + Sync {
    fn run(&self) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

pub struct BackgroundTaskRunner {
    interval_duration: Duration,
    shutdown_notify: Arc<Notify>,
    task_handle: Mutex<Option<task::JoinHandle<()>>>,
    runtime_handle: Handle,
}

impl BackgroundTaskRunner {
    pub fn new(interval_duration_ms: u32, runtime_handle: &Handle) -> Self {
        Self {
            interval_duration: Duration::from_millis(interval_duration_ms as u64),
            shutdown_notify: Arc::new(Notify::new()),
            task_handle: Mutex::new(None),
            runtime_handle: runtime_handle.clone(),
        }
    }

    pub fn start<T: BackgroundTask + 'static>(&self, task: Weak<T>) -> Result<(), String> {
        let interval_duration = self.interval_duration;
        let shutdown_notify = Arc::clone(&self.shutdown_notify);

        let handle = self.runtime_handle.spawn(async move {
            let mut interval = interval_at(Instant::now() + interval_duration, interval_duration);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Some(task) = task.upgrade() {
                            task.run().await;
                        } else {
                            break;
                        }
                    }
                    _ = shutdown_notify.notified() => {
                        break;
                    }
                }
            }
        });

        match self.task_handle.lock() {
            Ok(mut guard) => {
                *guard = Some(handle);
                Ok(())
            }
            Err(err) => Err(format!("Failed to lock task_handle: {}", err)),
        }
    }

    pub async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr> {
        self.shutdown_notify.notify_one();

        let task_handle = {
            match self.task_handle.lock() {
                Ok(mut guard) => guard.take(),
                Err(e) => {
                    return Err(StatsigErr::CustomError(format!(
                        "Failed to acquire task handle {}",
                        e
                    )))
                }
            }
        };

        if let Some(handle) = task_handle {
            let shutdown_future = handle;
            let shutdown_result = tokio::time::timeout(timeout, shutdown_future).await;

            if shutdown_result.is_err() {
                return Err(StatsigErr::CustomError(
                    "Task did not shut down gracefully within the timeout".to_string(),
                ));
            } else {
                return Ok(());
            }
        }

        Err(StatsigErr::CustomError(
            "No running task to shut down".to_string(),
        ))
    }
}
