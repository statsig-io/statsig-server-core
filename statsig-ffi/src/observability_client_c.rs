use std::{collections::HashMap, ffi::c_char};

use async_trait::async_trait;
use serde::Serialize;
use statsig_rust::{
    log_d, log_e, unwrap_or_noop, InstanceRegistry, ObservabilityClient, OpsStatsEventObserver,
};

use crate::ffi_utils::{c_char_to_string, string_to_c_char};

const TAG: &str = "ObservabilityClientC";

pub struct ObservabilityClientC {
    pub init_fn: extern "C" fn(),
    pub increment_fn: extern "C" fn(args_ptr: *const c_char, args_length: u64),
    pub gauge_fn: extern "C" fn(args_ptr: *const c_char, args_length: u64),
    pub dist_fn: extern "C" fn(args_ptr: *const c_char, args_length: u64),
    pub error_fn: extern "C" fn(args_ptr: *const c_char, args_length: u64),
    pub should_enable_high_cardinality_for_this_tag_fn:
        extern "C" fn(args_ptr: *const c_char, args_length: u64) -> bool,
}

#[derive(Serialize)]
struct ObsClientArgs {
    metric: String,
    value: f64,
    tags: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
struct ObsClientErrorArgs {
    tag: String,
    error: String,
}

// -------------------------------------------------------------------- [ Trait Impl ]

#[async_trait]
impl ObservabilityClient for ObservabilityClientC {
    fn init(&self) {
        (self.init_fn)();
    }

    fn increment(&self, metric: String, value: f64, tags: Option<HashMap<String, String>>) {
        if let Ok((args, args_len)) = parcel_args("increment", metric, value, tags) {
            (self.increment_fn)(args, args_len);
        }
    }

    fn gauge(&self, metric: String, value: f64, tags: Option<HashMap<String, String>>) {
        if let Ok((args, args_len)) = parcel_args("gauge", metric, value, tags) {
            (self.gauge_fn)(args, args_len);
        }
    }

    fn dist(&self, metric: String, value: f64, tags: Option<HashMap<String, String>>) {
        if let Ok((args, args_len)) = parcel_args("dist", metric, value, tags) {
            (self.dist_fn)(args, args_len);
        }
    }

    fn error(&self, tag: String, error: String) {
        if let Ok((args, args_len)) = parcel_error_args("error", tag, error) {
            (self.error_fn)(args, args_len);
        }
    }

    fn should_enable_high_cardinality_for_this_tag(&self, tag: String) -> Option<bool> {
        let tag_len = tag.len() as u64;
        let tag = string_to_c_char(tag);
        let value = (self.should_enable_high_cardinality_for_this_tag_fn)(tag, tag_len);

        Some(value)
    }

    fn to_ops_stats_event_observer(
        self: std::sync::Arc<Self>,
    ) -> std::sync::Arc<dyn OpsStatsEventObserver> {
        self
    }
}

// -------------------------------------------------------------------- [ C Extern ]

#[no_mangle]
pub extern "C" fn observability_client_create(
    init_fn: extern "C" fn(),
    increment_fn: extern "C" fn(args_ptr: *const c_char, args_length: u64),
    gauge_fn: extern "C" fn(args_ptr: *const c_char, args_length: u64),
    dist_fn: extern "C" fn(args_ptr: *const c_char, args_length: u64),
    error_fn: extern "C" fn(args_ptr: *const c_char, args_length: u64),
    should_enable_high_cardinality_for_this_tag_fn: extern "C" fn(
        args_ptr: *const c_char,
        args_length: u64,
    ) -> bool,
) -> u64 {
    InstanceRegistry::register(ObservabilityClientC {
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
pub extern "C" fn observability_client_release(data_store_ref: u64) {
    InstanceRegistry::remove(&data_store_ref);
}

#[no_mangle]
pub extern "C" fn __internal__test_observability_client(
    ob_client_ref: u64,
    action: *const c_char,
    metric_name: *const c_char,
    value: f64,
    tags: *const c_char,
) {
    let client = match InstanceRegistry::get::<ObservabilityClientC>(&ob_client_ref) {
        Some(observability_client) => observability_client,
        None => {
            log_d!(TAG, "TEST ObservabilityClient reference not found");
            return;
        }
    };

    let action = unwrap_or_noop!(c_char_to_string(action));
    let metric_name = unwrap_or_noop!(c_char_to_string(metric_name));
    let tags = c_char_to_string(tags)
        .filter(|tags| !tags.is_empty())
        .map(|tags| serde_json::from_str::<HashMap<String, String>>(&tags).unwrap());

    match action.as_str() {
        "init" => client.init(),
        "increment" => client.increment(metric_name, value, tags),
        "gauge" => client.gauge(metric_name, value, tags),
        "dist" => client.dist(metric_name, value, tags),
        "error" => client.error(
            metric_name,
            tags.unwrap().get("test_error").unwrap().to_string(),
        ),
        "should_enable_high_cardinality_for_this_tag" => {
            client.should_enable_high_cardinality_for_this_tag(metric_name);
        }
        _ => panic!("Invalid action: {action}"),
    }
}

// -------------------------------------------------------------------- [ Helpers ]

fn parcel_args(
    action: &'static str,
    metric: String,
    value: f64,
    tags: Option<HashMap<String, String>>,
) -> Result<(*mut c_char, u64), ()> {
    let args = ObsClientArgs {
        metric,
        value,
        tags,
    };

    jsonify_args(action, args)
}

fn parcel_error_args(
    action: &'static str,
    tag: String,
    error: String,
) -> Result<(*mut c_char, u64), ()> {
    let args = ObsClientErrorArgs { tag, error };

    jsonify_args(action, args)
}

fn jsonify_args<T: Serialize>(action: &'static str, args: T) -> Result<(*mut c_char, u64), ()> {
    let args_json = match serde_json::to_string(&args) {
        Ok(args_json) => args_json,
        Err(e) => {
            log_e!(
                TAG,
                "Failed to parcel ObservabilityClient.{action} args: {e}"
            );
            return Err(());
        }
    };
    let args_json_len = args_json.len() as u64;

    Ok((string_to_c_char(args_json), args_json_len))
}
