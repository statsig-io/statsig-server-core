use super::{
    diagnostics_utils::DiagnosticsUtils,
    marker::{ActionType, KeyType, Marker, StepType},
};
use crate::{event_logging::event_logger::{EventLogger, QueuedEventPayload}, read_lock_or_else, SpecStore};
use crate::event_logging::{
    statsig_event::StatsigEvent, statsig_event_internal::StatsigEventInternal,
};
use crate::log_w;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use chrono::Utc;

const MAX_MARKER_COUNT: usize = 50;
const DIAGNOSTICS_EVENT: &str = "statsig::diagnostics";

#[derive(Eq, Hash, PartialEq)]
pub enum ContextType {
    Initialize, // we only care about initialize for now
}

impl fmt::Display for ContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextType::Initialize => write!(f, "initialize"),
        }
    }
}

const TAG: &str = stringify!(Diagnostics);

pub struct Diagnostics {
    marker_map: Mutex<HashMap<ContextType, Vec<Marker>>>,
    event_logger: Arc<EventLogger>,
    spec_store: Arc<SpecStore>,
}

impl Diagnostics {
    pub fn new(event_logger: Arc<EventLogger>, spec_store: Arc<SpecStore>) -> Self {
        Self {
            event_logger,
            marker_map: Mutex::new(HashMap::new()),
            spec_store,
        }
    }

    pub fn get_markers(&self, context_type: &ContextType) -> Option<Vec<Marker>> {
        if let Ok(map) = self.marker_map.lock() {
            if let Some(markers) = map.get(context_type) {
                return Some(markers.clone());
            }
        }
        None
    }

    pub fn add_marker(&self, context_type: ContextType, marker: Marker) {
        if let Ok(mut map) = self.marker_map.lock() {
            let entry = map.entry(context_type).or_insert_with(Vec::new);
            if entry.len() < MAX_MARKER_COUNT {
                entry.push(marker);
            }
        }
    }

    pub fn clear_markers(&self, context_type: &ContextType) {
        if let Ok(mut map) = self.marker_map.lock() {
            if let Some(markers) = map.get_mut(context_type) {
                markers.clear();
            }
        }
    }

    pub fn mark_init_overall_start(&self) {
        let init_marker = Marker::new(
            KeyType::Overall, 
            ActionType::Start, 
            Some(StepType::Process), 
            Utc::now().timestamp_millis() as u64
        );
        self.add_marker(ContextType::Initialize, init_marker);
    }

    pub fn mark_init_overall_end(&self, success: bool, error_message: Option<String>) {
        let mut init_marker =
            Marker::new(
                KeyType::Overall, 
                ActionType::End, 
                Some(StepType::Process), 
                Utc::now().timestamp_millis() as u64
            ).with_is_success(success);

        if let Some(msg) = error_message {
            init_marker = init_marker.with_message(msg);
        }
        self.add_marker(ContextType::Initialize, init_marker);
        self.enqueue_diagnostics_event(ContextType::Initialize);
    }

    pub fn enqueue_diagnostics_event(&self, context_type: ContextType) {
        let markers = match self.get_markers(&context_type) {
            Some(m) => m,
            None => return,
        };

        if markers.is_empty() {
            return;
        }

        let metadata = match DiagnosticsUtils::format_diagnostics_metadata(&context_type, &markers)
        {
            Ok(data) => data,
            Err(err) => {
                log_w!(TAG, "Failed to format diagnostics metadata: {}", err);
                return;
            }
        };

        let event = StatsigEventInternal::new_diagnostic_event(StatsigEvent {
            event_name: DIAGNOSTICS_EVENT.to_string(),
            value: None,
            metadata: Some(metadata),
        });

        let data = read_lock_or_else!(self.spec_store.data, {
            log_w!(TAG, "Failed to acquire read lock for diagnostics event");
            return;
        });

        let diagnostics = &data.values.diagnostics;

        if let Some(diagnostics) = diagnostics {
            if let Some(sample_rate) = diagnostics.get(&context_type.to_string()) {
                if !DiagnosticsUtils::should_sample(*sample_rate) {
                    self.clear_markers(&context_type);
                    return;
                }
            }
        }

        self.event_logger
            .enqueue(QueuedEventPayload::CustomEvent(event));

        self.clear_markers(&context_type);
    }
}
