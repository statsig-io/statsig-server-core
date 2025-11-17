use super::{
    observability_client_adapter::ObservabilityEvent, sdk_errors_observer::ErrorBoundaryEvent,
    DiagnosticsEvent,
};
use crate::{log_e, log_w, StatsigRuntime};
use crate::{
    observability::console_capture_observer::ConsoleCaptureEvent,
    sdk_diagnostics::{
        diagnostics::ContextType,
        marker::{KeyType, Marker},
    },
};
use async_trait::async_trait;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use tokio::sync::broadcast::{self, Sender};
use tokio::sync::Notify;

const TAG: &str = stringify!(OpsStats);

/* Ideally we don't need to pass OpsStats around, but right now I could find a good way to do it to support multiple instances*/
lazy_static! {
    pub static ref OPS_STATS: OpsStats = OpsStats::new();
}

pub struct OpsStats {
    instances_map: RwLock<HashMap<String, Weak<OpsStatsForInstance>>>, // key is sdk key
}

impl Default for OpsStats {
    fn default() -> Self {
        Self::new()
    }
}

impl OpsStats {
    pub fn new() -> Self {
        OpsStats {
            instances_map: HashMap::new().into(),
        }
    }

    pub fn get_for_instance(&self, sdk_key: &str) -> Arc<OpsStatsForInstance> {
        match self
            .instances_map
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(read_guard) => {
                if let Some(instance) = read_guard.get(sdk_key) {
                    if let Some(instance) = instance.upgrade() {
                        return instance.clone();
                    }
                }
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to get read guard: Failed to lock instances_map"
                );
            }
        }

        let instance = Arc::new(OpsStatsForInstance::new());
        match self
            .instances_map
            .try_write_for(std::time::Duration::from_secs(5))
        {
            Some(mut write_guard) => {
                write_guard.insert(sdk_key.into(), Arc::downgrade(&instance));
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to get write guard: Failed to lock instances_map"
                );
            }
        }

        instance
    }

    pub fn get_weak_instance_for_key(&self, sdk_key: &str) -> Option<Weak<OpsStatsForInstance>> {
        match self
            .instances_map
            .try_read_for(std::time::Duration::from_secs(5))
        {
            Some(read_guard) => read_guard.get(sdk_key).cloned(),
            None => None,
        }
    }
}

#[derive(Clone)]
pub enum OpsStatsEvent {
    Observability(ObservabilityEvent),
    SDKError(ErrorBoundaryEvent),
    Diagnostics(DiagnosticsEvent),
    ConsoleCapture(ConsoleCaptureEvent),
}

pub struct OpsStatsForInstance {
    sender: Sender<OpsStatsEvent>,
    shutdown_notify: Arc<Notify>,
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
        OpsStatsForInstance {
            sender: tx,
            shutdown_notify: Arc::new(Notify::new()),
        }
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
        self.log(OpsStatsEvent::SDKError(error));
    }

    pub fn add_marker(&self, marker: Marker, context: Option<ContextType>) {
        self.log(OpsStatsEvent::Diagnostics(DiagnosticsEvent {
            marker: Some(marker),
            context,
            key: None,
            should_enqueue: false,
        }));
    }

    pub fn set_diagnostics_context(&self, context: ContextType) {
        self.log(OpsStatsEvent::Diagnostics(DiagnosticsEvent {
            marker: None,
            context: Some(context),
            key: None,
            should_enqueue: false,
        }));
    }

    pub fn enqueue_diagnostics_event(&self, key: Option<KeyType>, context: Option<ContextType>) {
        self.log(OpsStatsEvent::Diagnostics(DiagnosticsEvent {
            marker: None,
            context,
            key,
            should_enqueue: true,
        }));
    }

    pub fn enqueue_console_capture_event(
        &self,
        level: String,
        payload: Vec<String>,
        timestamp: u64,
    ) {
        self.log(OpsStatsEvent::ConsoleCapture(ConsoleCaptureEvent {
            level,
            payload,
            timestamp,
        }));
    }

    pub fn subscribe(
        &self,
        runtime: Arc<StatsigRuntime>,
        observer: Weak<dyn OpsStatsEventObserver>,
    ) {
        let mut rx = self.sender.subscribe();
        let shutdown_notify = self.shutdown_notify.clone();
        let _ = runtime.spawn("opts_stats_listen_for", |rt_shutdown_notify| async move {
            loop {
                tokio::select! {
                    event = rx.recv() => {
                        let observer = match observer.upgrade() {
                            Some(observer) => observer,
                            None => break,
                        };

                        if let Ok(event) = event {
                            observer.handle_event(event).await;
                        }
                    }
                    () = rt_shutdown_notify.notified() => {
                        break;
                    }
                    () = shutdown_notify.notified() => {
                        break;
                    }
                }
            }
        });
    }
}

impl Drop for OpsStatsForInstance {
    fn drop(&mut self) {
        self.shutdown_notify.notify_waiters();
    }
}

#[async_trait]
pub trait OpsStatsEventObserver: Send + Sync + 'static {
    async fn handle_event(&self, event: OpsStatsEvent);
}
