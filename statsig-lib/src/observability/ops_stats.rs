use async_trait::async_trait;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::broadcast::{self, Sender};

use crate::{log_w, StatsigRuntime};

use super::{
    observability_client_adapter::ObservabilityEvent, sdk_errors_observer::ErrorBoundaryEvent,
};

const TAG: &str = stringify!(OpsStats);

/* Ideally we don't need to pass OpsStats around, but right now I could find a good way to do it to support multiple instances*/
lazy_static! {
    pub static ref OPS_STATS: OpsStats = OpsStats::new();
}

pub struct OpsStats {
    instances_map: RwLock<HashMap<String, Arc<OpsStatsForInstance>>>, // key is sdk key
}

impl OpsStats {
    pub fn new() -> Self {
        OpsStats {
            instances_map: HashMap::new().into(),
        }
    }

    pub fn get_for_instance(&self, sdk_key: &str) -> Arc<OpsStatsForInstance> {
        if let Ok(read_guard) = self.instances_map.read() {
            if read_guard.contains_key(sdk_key) {
                return read_guard
                    .get(sdk_key)
                    .unwrap_or(&Arc::new(OpsStatsForInstance::new()))
                    .clone();
            } else {
                drop(read_guard);
                if let Ok(mut write_lock) = self.instances_map.write() {
                    let instance = Arc::new(OpsStatsForInstance::new());
                    write_lock.insert(sdk_key.to_string(), instance.clone());
                    return instance;
                }
            }
        }

        log_w!(TAG, "Failed to retrieve stateful OpsStats");
        Arc::new(OpsStatsForInstance::new())
    }
}

#[derive(Clone)]
pub enum OpsStatsEvent {
    ObservabilityEvent(ObservabilityEvent),
    SDKErrorEvent(ErrorBoundaryEvent),
}

pub struct OpsStatsForInstance {
    sender: Sender<OpsStatsEvent>,
}

// The class used to handle all observability events including diagnostics, error, event logging, and external metric sharing
impl Default for OpsStatsForInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl OpsStatsForInstance {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        OpsStatsForInstance { sender: tx }
    }

    pub fn log(&self, event: OpsStatsEvent) {
        match self.sender.send(event) {
            Ok(_) => {}
            Err(e) => {
                log_w!(
                    "OpsStats Message Queue",
                    "Dropping ops stats event {}",
                    e.to_string()
                );
            }
        }
    }

    pub fn log_error(&self, error: ErrorBoundaryEvent) {
        self.log(OpsStatsEvent::SDKErrorEvent(error))
    }

    pub fn subscribe(
        &self,
        runtime: Arc<StatsigRuntime>,
        observer: Arc<dyn OpsStatsEventObserver>,
    ) {
        let mut rx = self.sender.subscribe();
        let _ = runtime.spawn("opts_stats_listen_for", |_| async move {
            while let Ok(event) = rx.recv().await {
                observer.handle_event(event).await;
            }
        });
    }
}

#[async_trait]
pub trait OpsStatsEventObserver: Send + Sync + 'static {
    async fn handle_event(&self, event: OpsStatsEvent);
}
