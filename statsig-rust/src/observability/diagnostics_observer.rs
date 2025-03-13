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
    pub context: ContextType,
    pub marker: Option<Marker>,
    pub key: Option<KeyType>,

    pub end: bool, // if end is true we will enqueue diagnostics event
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
        if let Some(marker) = event.marker {
            self.add_diagnostics_marker(event.context.clone(), marker);
        }
        if event.end {
            self.mark_diagnostic_end(event.context, event.key);
        }
    }

    fn add_diagnostics_marker(&self, context: ContextType, marker: Marker) {
        self.diagnostics_handler.add_marker(context, marker);
    }

    fn mark_diagnostic_end(&self, context: ContextType, key: Option<KeyType>) {
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
