use async_trait::async_trait;

use statsig_rust::{log_e, InstanceRegistry, ObservabilityClient, OpsStatsEventObserver};
use std::collections::HashMap;
use std::ffi::c_char;
use std::sync::Mutex;

use crate::ffi_utils::string_to_c_char;

const TAG: &str = "ObservabilityClientC";
pub struct ObservabilityClientC {
    pub ob_client_ref: Mutex<Option<u64>>,
    pub init_fn: extern "C" fn(ob_client_ref: u64),
    pub increment_fn: extern "C" fn(
        ob_client_ref: u64,
        metric_name: *const c_char,
        value: f64,
        tags: *const c_char,
    ),
    pub gauge_fn: extern "C" fn(
        ob_client_ref: u64,
        metric_name: *const c_char,
        value: f64,
        tags: *const c_char,
    ),
    pub dist_fn: extern "C" fn(
        ob_client_ref: u64,
        metric_name: *const c_char,
        value: f64,
        tags: *const c_char,
    ),
    pub error_fn: extern "C" fn(ob_client_ref: u64, tag: *const c_char, error: *const c_char),
    pub should_enable_high_cardinality_for_this_tag_fn:
        extern "C" fn(ob_client_ref: u64, tag: *const c_char) -> bool,
}

#[async_trait]
impl ObservabilityClient for ObservabilityClientC {
    fn init(&self) {
        let ref_id = self.ob_client_ref.lock().unwrap().unwrap_or(0);
        (self.init_fn)(ref_id);
    }

    fn increment(&self, metric: String, value: f64, tags: Option<HashMap<String, String>>) {
        let ref_id = self.ob_client_ref.lock().unwrap().unwrap_or(0);
        let metric = string_to_c_char(metric);
        let tags_c_char = string_to_c_char(serde_json::to_string(&tags).unwrap());

        (self.increment_fn)(ref_id, metric, value, tags_c_char);
    }

    fn gauge(&self, metric: String, value: f64, tags: Option<HashMap<String, String>>) {
        let ref_id = self.ob_client_ref.lock().unwrap().unwrap_or(0);
        let metric = string_to_c_char(metric);
        let tags_c_char = string_to_c_char(serde_json::to_string(&tags).unwrap());

        (self.gauge_fn)(ref_id, metric, value, tags_c_char);
    }

    fn dist(&self, metric: String, value: f64, tags: Option<HashMap<String, String>>) {
        let ref_id = self.ob_client_ref.lock().unwrap().unwrap_or(0);
        let metric = string_to_c_char(metric);
        let tags_c_char = string_to_c_char(serde_json::to_string(&tags).unwrap());

        (self.dist_fn)(ref_id, metric, value, tags_c_char);
    }

    fn error(&self, tag: String, error: String) {
        let ref_id = self.ob_client_ref.lock().unwrap().unwrap_or(0);
        let tag = string_to_c_char(tag);
        let error = string_to_c_char(error);
        (self.error_fn)(ref_id, tag, error)
    }

    fn should_enable_high_cardinality_for_this_tag(&self, tag: String) -> Option<bool> {
        let ref_id = self.ob_client_ref.lock().unwrap().unwrap_or(0);
        let tag = string_to_c_char(tag);
        let value = (self.should_enable_high_cardinality_for_this_tag_fn)(ref_id, tag);

        Some(value)
    }

    fn to_ops_stats_event_observer(
        self: std::sync::Arc<Self>,
    ) -> std::sync::Arc<dyn OpsStatsEventObserver> {
        self
    }
}

impl ObservabilityClientC {
    /// Set the ob client reference ID after creation
    pub fn set_observability_client_ref(&self, id: u64) {
        if let Ok(mut guard) = self.ob_client_ref.lock() {
            *guard = Some(id);
        }
    }
}

#[no_mangle]
pub extern "C" fn observability_client_create(
    init_fn: extern "C" fn(ob_client_ref: u64),
    increment_fn: extern "C" fn(
        ob_client_ref: u64,
        metric_name: *const c_char,
        value: f64,
        tags: *const c_char,
    ),
    gauge_fn: extern "C" fn(
        ob_client_ref: u64,
        metric_name: *const c_char,
        value: f64,
        tags: *const c_char,
    ),
    dist_fn: extern "C" fn(
        ob_client_ref: u64,
        metric_name: *const c_char,
        value: f64,
        tags: *const c_char,
    ),
    error_fn: extern "C" fn(ob_client_ref: u64, tag: *const c_char, error: *const c_char),
    should_enable_high_cardinality_for_this_tag_fn: extern "C" fn(
        ob_client_ref: u64,
        tag: *const c_char,
    ) -> bool,
) -> u64 {
    InstanceRegistry::register(ObservabilityClientC {
        ob_client_ref: Mutex::new(None),
        init_fn,
        increment_fn,
        gauge_fn,
        dist_fn,
        error_fn,
        should_enable_high_cardinality_for_this_tag_fn,
    })
    .unwrap_or_else(|| {
        log_e!(TAG, "Failed to create ObservabilityClientC");
        0
    })
}

#[no_mangle]
pub extern "C" fn observability_client_set_ref(ob_client_ref: u64, id: u64) {
    if let Some(obs_client) = InstanceRegistry::get::<ObservabilityClientC>(&ob_client_ref) {
        obs_client.set_observability_client_ref(id);
    }
}

#[no_mangle]
pub extern "C" fn observability_client_release(ob_client_ref: u64) {
    InstanceRegistry::remove(&ob_client_ref);
}
