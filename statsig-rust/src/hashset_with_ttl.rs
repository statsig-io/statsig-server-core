use crate::log_d;
use crate::log_e;
use crate::StatsigRuntime;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

const TAG: &str = stringify!(HashSetWithTTL);

pub struct HashSetWithTTL {
    set: Arc<Mutex<HashSet<String>>>,
    shutdown_notify: Arc<Notify>,
}

impl HashSetWithTTL {
    pub fn new(statsig_runtime: &Arc<StatsigRuntime>, interval_duration: Duration) -> Self {
        let instance = Self {
            set: Arc::new(Mutex::new(HashSet::new())),
            shutdown_notify: Arc::new(Notify::new()),
        };

        let weak_instance = Arc::downgrade(&instance.set);
        let shutdown_notify = instance.shutdown_notify.clone();

        let spawn_result = statsig_runtime.spawn(
            "hashset_with_ttl_worker",
            move |rt_shutdown_notify| async move {
                Self::run_background_reset(
                    weak_instance,
                    interval_duration,
                    rt_shutdown_notify,
                    shutdown_notify,
                )
                .await;
            },
        );

        if let Err(e) = spawn_result {
            log_e!(TAG, "Failed to spawn hashset with ttl worker: {e}");
        }

        instance
    }

    pub fn add(&self, key: String) -> Result<(), String> {
        match self.set.lock() {
            Ok(mut set) => {
                set.insert(key);
                Ok(())
            }
            Err(e) => Err(format!("Failed to acquire lock: {e}")),
        }
    }

    pub fn contains(&self, key: &str) -> Result<bool, String> {
        match self.set.lock() {
            Ok(set) => Ok(set.contains(key)),
            Err(e) => Err(format!("Failed to acquire lock: {e}")),
        }
    }

    pub async fn shutdown(&self) {
        self.shutdown_notify.notify_one();
    }

    fn reset(set: &Mutex<HashSet<String>>) -> Result<(), String> {
        match set.lock() {
            Ok(mut set) => {
                set.clear();
                Ok(())
            }
            Err(e) => Err(format!("Failed to acquire lock: {e}")),
        }
    }

    async fn run_background_reset(
        weak_instance: Weak<Mutex<HashSet<String>>>,
        interval_duration: Duration,
        rt_shutdown_notify: Arc<Notify>,
        shutdown_notify: Arc<Notify>,
    ) {
        loop {
            tokio::select! {
                () = sleep(interval_duration) => {
                    if let Some(set) = weak_instance.upgrade() {
                        if let Err(e) = Self::reset(&set) {
                            log_d!(TAG, "Failed to reset HashSetWithTTL: {}", e);
                        }
                    } else {
                        break;
                    }
                }
                () = rt_shutdown_notify.notified() => {
                    log_d!(TAG, "Runtime shutdown. Exiting hashset with ttl worker.");
                    break;
                }
                () = shutdown_notify.notified() => {
                    log_d!(TAG, "Shutdown signal received. Exiting hashset with ttl worker.");
                    break;
                }
            }
        }
    }
}
