use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::{
    sdk_diagnostics::{
        diagnostics::{ContextType, Diagnostics},
        marker::{KeyType, Marker},
    },
    OpsStatsEventObserver,
};

use super::ops_stats::OpsStatsEvent;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsEvent {
    pub marker: Option<Marker>,
    pub context: Option<ContextType>,
    pub key: Option<KeyType>,

    pub should_enqueue: bool,
}

pub struct DiagnosticsObserver {
    diagnostics_handler: Arc<Diagnostics>,
}

impl DiagnosticsObserver {
    pub fn new(diagnostics: Arc<Diagnostics>) -> Self {
        Self {
            diagnostics_handler: diagnostics,
        }
    }

    async fn handle_diagnostics_event(&self, event: DiagnosticsEvent) {
        if let Some(context) = event.context {
            self.diagnostics_handler.set_context(&context);
        }
        if let Some(marker) = event.marker {
            self.add_diagnostics_marker(event.context.as_ref(), marker);
        }
        if event.should_enqueue {
            self.enqueue_diagnostics_event(event.context.as_ref(), event.key);
        }
    }

    fn add_diagnostics_marker(&self, context: Option<&ContextType>, marker: Marker) {
        self.diagnostics_handler.add_marker(context, marker);
    }

    fn enqueue_diagnostics_event(&self, context: Option<&ContextType>, key: Option<KeyType>) {
        self.diagnostics_handler
            .enqueue_diagnostics_event(context, key);
    }
}

#[async_trait]
impl OpsStatsEventObserver for DiagnosticsObserver {
    async fn handle_event(&self, event: OpsStatsEvent) {
        if let OpsStatsEvent::Diagnostics(e) = event {
            self.handle_diagnostics_event(e).await;
        }
    }
}
