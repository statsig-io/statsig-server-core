use async_trait::async_trait;
use std::{f32::consts::E, sync::Arc};
use tokio::sync::broadcast::{self, Sender};

use crate::{log_w, StatsigRuntime};

use super::observability_client_adapter::ObservabilityEvent;

/* Ideally we don't need to pass OpsStats around, but right now I could find a good way to do it to support multiple instances
 lazy_static! {
//   pub static ref OPS_STATS: OpsStats = OpsStats::new();
 }


pub struct OpsStats {
  instances_map: RwLock<HashMap<String, OpsStatsForInstance>>
}

impl OpsStats {
  pub fn new() -> Self {
    OpsStats {
      instances_map: HashMap::new().into()
    }
  }

  pub get_for_instance() {}
}
*/

#[derive(Clone)]
pub enum OpsStatsEvent {
    ObservabilityEvent(ObservabilityEvent),
}

pub struct OpsStatsForInstance {
    sender: Sender<OpsStatsEvent>,
}

// The class used to handle all observability events including diagnostics, error, event logging, and external metric sharing
impl OpsStatsForInstance {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        OpsStatsForInstance { sender: tx }
    }

    pub fn log(&self, event: OpsStatsEvent) {
        match self.sender.send(event) {
            Ok(_) => {}
            Err(e) => {
                log_w!("OpsStats Message Queue", "Dropping ops stats event {}", e.to_string());
            }
        }
    }

    pub fn subscribe(
        &self,
        runtime: Arc<StatsigRuntime>,
        observer: Arc<dyn IOpsStatsEventObserver>,
    ) {
        let mut rx = self.sender.subscribe();
        let _ = runtime.spawn("listen_for", |_| async move {
            while let Ok(event) = rx.recv().await {
                observer.handle_event(event).await;
            }
        });
    }
}

#[async_trait]
pub trait IOpsStatsEventObserver: Send + Sync + 'static {
    async fn handle_event(&self, event: OpsStatsEvent);
    
}
