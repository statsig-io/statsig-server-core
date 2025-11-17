use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::{
    console_capture::console_capture_helper::ConsoleCapture,
    observability::ops_stats::OpsStatsEvent, OpsStatsEventObserver,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleCaptureEvent {
    pub level: String,
    pub payload: Vec<String>,
    pub timestamp: u64,
}
pub struct ConsoleCaptureObserver {
    console_capture_handler: Arc<ConsoleCapture>,
}

impl ConsoleCaptureObserver {
    pub fn new(console_capture_handler: Arc<ConsoleCapture>) -> Self {
        Self {
            console_capture_handler,
        }
    }

    async fn handle_console_capture_event(&self, event: ConsoleCaptureEvent) {
        self.console_capture_handler
            .handle_console_capture_event(event);
    }
}

#[async_trait]
impl OpsStatsEventObserver for ConsoleCaptureObserver {
    async fn handle_event(&self, event: OpsStatsEvent) {
        if let OpsStatsEvent::ConsoleCapture(e) = event {
            self.handle_console_capture_event(e).await;
        }
    }
}
