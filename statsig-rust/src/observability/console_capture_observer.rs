use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    console_capture::console_capture_handler::ConsoleCaptureHandler,
    observability::ops_stats::OpsStatsEvent, user::StatsigUserLoggable, OpsStatsEventObserver,
};

#[derive(Clone)]
pub struct ConsoleCaptureEvent {
    pub level: String,
    pub payload: Vec<String>,
    pub timestamp: u64,
    pub user: StatsigUserLoggable,

    pub stack_trace: Option<String>,
}
pub struct ConsoleCaptureObserver {
    console_capture_handler: Arc<ConsoleCaptureHandler>,
}

impl ConsoleCaptureObserver {
    pub fn new(console_capture_handler: Arc<ConsoleCaptureHandler>) -> Self {
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
