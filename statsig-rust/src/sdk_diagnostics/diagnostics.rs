use super::{
    diagnostics_utils::DiagnosticsUtils,
    marker::{KeyType, Marker},
};
use crate::event_logging::event_logger::{EventLogger, QueuedEventPayload};
use crate::event_logging::{
    statsig_event::StatsigEvent, statsig_event_internal::StatsigEventInternal,
};
use crate::log_w;
use rand::Rng;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

const MAX_MARKER_COUNT: usize = 50;
pub const DIAGNOSTICS_EVENT: &str = "statsig::diagnostics";

#[derive(Eq, Hash, PartialEq, Clone, Serialize, Debug, Copy)]
pub enum ContextType {
    Initialize,
    ConfigSync,
}

impl fmt::Display for ContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextType::Initialize => write!(f, "initialize"),
            ContextType::ConfigSync => write!(f, "config_sync"),
        }
    }
}

const TAG: &str = stringify!(Diagnostics);
const MAX_SAMPLING_RATE: f64 = 10000.0;
const DEFAULT_SAMPLING_RATE: f64 = 100.0;

pub struct Diagnostics {
    marker_map: Mutex<HashMap<ContextType, Vec<Marker>>>,
    event_logger: Arc<EventLogger>,
    sampling_rates: Mutex<HashMap<String, f64>>,
    context: Mutex<ContextType>,
}

impl Diagnostics {
    pub fn new(event_logger: Arc<EventLogger>) -> Self {
        Self {
            event_logger,
            marker_map: Mutex::new(HashMap::new()),
            sampling_rates: Mutex::new(HashMap::from([
                ("initialize".to_string(), 10000.0),
                ("config_sync".to_string(), 1000.0),
            ])),
            context: Mutex::new(ContextType::Initialize),
        }
    }

    pub fn set_context(&self, context: &ContextType) {
        if let Ok(mut ctx) = self.context.lock() {
            *ctx = *context;
        }
    }

    pub fn set_sampling_rate(&self, new_sampling_rate: HashMap<std::string::String, f64>) {
        if let Ok(mut rates) = self.sampling_rates.lock() {
            for (key, rate) in new_sampling_rate {
                let clamped_rate = rate.clamp(0.0, MAX_SAMPLING_RATE);
                rates.insert(key, clamped_rate);
            }
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

    pub fn add_marker(&self, context_type: Option<&ContextType>, marker: Marker) {
        let context_type = self.get_context(context_type);
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

    pub fn enqueue_diagnostics_event(
        &self,
        context_type: Option<&ContextType>,
        key: Option<KeyType>,
    ) {
        let context_type: ContextType = self.get_context(context_type);
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
            statsig_metadata: None,
        });

        if !self.should_sample(&context_type, key) {
            self.clear_markers(&context_type);
            return;
        }

        self.event_logger
            .enqueue(QueuedEventPayload::CustomEvent(event));

        self.clear_markers(&context_type);
    }

    pub fn should_sample(&self, context: &ContextType, key: Option<KeyType>) -> bool {
        let mut rng = rand::thread_rng();
        let rand_value = rng.gen::<f64>() * MAX_SAMPLING_RATE;

        let sampling_rates = match self.sampling_rates.lock() {
            Ok(guard) => guard,
            Err(_) => return rand_value < DEFAULT_SAMPLING_RATE,
        };

        if *context == ContextType::Initialize {
            return rand_value
                < *sampling_rates
                    .get("initialize")
                    .unwrap_or(&DEFAULT_SAMPLING_RATE);
        }

        if let Some(key) = key {
            match key {
                KeyType::GetIDList | KeyType::GetIDListSources => {
                    return rand_value
                        < *sampling_rates
                            .get("id_list")
                            .unwrap_or(&DEFAULT_SAMPLING_RATE);
                }
                KeyType::DownloadConfigSpecs => {
                    return rand_value
                        < *sampling_rates.get("dcs").unwrap_or(&DEFAULT_SAMPLING_RATE);
                }
                _ => {}
            }
        }

        rand_value < DEFAULT_SAMPLING_RATE
    }

    fn get_context(&self, maybe_context: Option<&ContextType>) -> ContextType {
        let context_type = match maybe_context {
            Some(ctx) => *ctx,
            None => *self.context.lock().unwrap(),
        };
        context_type
    }
}
