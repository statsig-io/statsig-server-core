use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::{get_instance_or_noop_c, get_instance_or_return_c};
use serde_json::json;
use serde_json::Value;
use statsig_rust::{
    log_d, log_e, unwrap_or_noop, unwrap_or_return, ClientInitResponseOptions,
    DynamicConfigEvaluationOptions, ExperimentEvaluationOptions, FeatureGateEvaluationOptions,
    InstanceRegistry, LayerEvaluationOptions, Statsig, StatsigOptions, StatsigUser,
};
use std::collections::HashMap;
use std::os::raw::c_char;
use std::ptr::null;

const TAG: &str = "StatsigC";

#[no_mangle]
pub extern "C" fn statsig_create(sdk_key: *const c_char, options_ref: u64) -> u64 {
    let sdk_key = c_char_to_string(sdk_key).unwrap();
    let options = InstanceRegistry::get_with_optional_id::<StatsigOptions>(Some(&options_ref));

    let inst = Statsig::new(&sdk_key, options);

    InstanceRegistry::register(inst).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create Statsig");
        0
    })
}

#[no_mangle]
pub extern "C" fn statsig_release(statsig_ref: u64) {
    InstanceRegistry::remove(&statsig_ref);
}

#[no_mangle]
pub extern "C" fn statsig_initialize(statsig_ref: u64, callback: extern "C" fn()) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);

    statsig
        .statsig_runtime
        .clone()
        .spawn(TAG, move |_| async move {
            if let Err(e) = statsig.initialize().await {
                log_e!(TAG, "Failed to initialize statsig: {}", e);
            }

            callback();
        });
}

#[no_mangle]
pub extern "C" fn statsig_initialize_blocking(statsig_ref: u64) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);

    statsig.statsig_runtime.get_handle().block_on(async move {
        if let Err(e) = statsig.initialize().await {
            log_e!(TAG, "Failed to initialize statsig: {}", e);
        }
    });
}

#[no_mangle]
pub extern "C" fn statsig_shutdown_blocking(statsig_ref: u64) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);

    statsig.statsig_runtime.get_handle().block_on(async move {
        if let Err(e) = statsig.shutdown().await {
            log_e!(TAG, "Failed to shutdown statsig: {}", e);
        }
    });
}

#[no_mangle]
pub extern "C" fn statsig_flush_events(statsig_ref: u64, callback: extern "C" fn()) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);

    statsig
        .statsig_runtime
        .clone()
        .spawn(TAG, move |_| async move {
            statsig.flush_events().await;

            callback();
        });
}

#[no_mangle]
pub extern "C" fn statsig_flush_events_blocking(statsig_ref: u64) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);

    statsig.statsig_runtime.get_handle().block_on(async move {
        log_d!(TAG, "Statsig flush events");
        statsig.flush_events().await;
        log_d!(TAG, "Flushed events");
    });
}

#[no_mangle]
pub extern "C" fn statsig_get_current_values(statsig_ref: u64) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null());

    let values = statsig.get_context().spec_store.get_current_values();
    let data = json!(values).to_string();
    string_to_c_char(data)
}

#[no_mangle]
pub extern "C" fn statsig_log_event(statsig_ref: u64, user_ref: u64, event_json: *const c_char) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let user = get_instance_or_noop_c!(StatsigUser, &user_ref);
    let event_json = unwrap_or_noop!(c_char_to_string(event_json));
    let event = match serde_json::from_str::<HashMap<String, Value>>(&event_json) {
        Ok(map) => map,
        Err(_) => return,
    };

    let event_name = unwrap_or_noop!(event.get("name").and_then(|n| n.as_str()));
    let event_metadata: Option<HashMap<String, String>> = event
        .get("metadata")
        .and_then(|m| m.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or("").to_string()))
                .collect()
        });

    match event.get("value") {
        Some(Value::String(value)) => {
            statsig.log_event(&user, event_name, Some(value.to_string()), event_metadata)
        }
        _ => statsig.log_event_with_number(
            &user,
            event_name,
            event.get("value").and_then(|v| v.as_f64()),
            event_metadata,
        ),
    }
}

#[no_mangle]
pub extern "C" fn statsig_identify(statsig_ref: u64, user_ref: u64) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let user = get_instance_or_noop_c!(StatsigUser, &user_ref);
    statsig.identify(user.as_ref());
}

#[no_mangle]
pub extern "C" fn statsig_get_client_init_response(
    statsig_ref: u64,
    user_ref: u64,
    options_json: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null());

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ClientInitResponseOptions>(&opts) {
            Ok(options) => options,
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null();
            }
        },
        None => ClientInitResponseOptions::default(),
    };

    let result = statsig.get_client_init_response_with_options(&user, &options);
    string_to_c_char(json!(result).to_string())
}

// ------------------------
// Feature Gate Functions
// ------------------------

#[no_mangle]
pub extern "C" fn statsig_check_gate(
    statsig_ref: u64,
    user_ref: u64,
    gate_name: *const c_char,
    options_json: *const c_char,
) -> bool {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, false);
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, false);
    let gate_name = unwrap_or_return!(c_char_to_string(gate_name), false);

    if let Some(opts) = c_char_to_string(options_json) {
        match serde_json::from_str::<FeatureGateEvaluationOptions>(&opts) {
            Ok(options) => return statsig.check_gate_with_options(&user, &gate_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return false;
            }
        }
    }

    statsig.check_gate(&user, &gate_name)
}

#[no_mangle]
pub extern "C" fn statsig_get_feature_gate(
    statsig_ref: u64,
    user_ref: u64,
    gate_name: *const c_char,
    options_json: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null());
    let gate_name = unwrap_or_return!(c_char_to_string(gate_name), null());

    let gate = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<FeatureGateEvaluationOptions>(&opts) {
            Ok(options) => statsig.get_feature_gate_with_options(&user, &gate_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null();
            }
        },
        None => statsig.get_feature_gate(&user, &gate_name),
    };

    let result = json!(gate).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_manually_log_gate_exposure(
    statsig_ref: u64,
    user_ref: u64,
    gate_name: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let user = get_instance_or_noop_c!(StatsigUser, &user_ref);
    let gate_name = unwrap_or_noop!(c_char_to_string(gate_name));

    statsig.manually_log_gate_exposure(&user, &gate_name);
}

// ------------------------
// Dynamic Config Functions
// ------------------------

#[no_mangle]
pub extern "C" fn statsig_get_dynamic_config(
    statsig_ref: u64,
    user_ref: u64,
    config_name: *const c_char,
    options_json: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null());
    let config_name = unwrap_or_return!(c_char_to_string(config_name), null());

    let config = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<DynamicConfigEvaluationOptions>(&opts) {
            Ok(options) => statsig.get_dynamic_config_with_options(&user, &config_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null();
            }
        },
        None => statsig.get_dynamic_config(&user, &config_name),
    };

    let result = json!(config).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_manually_log_dynamic_config_exposure(
    statsig_ref: u64,
    user_ref: u64,
    config_name: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let user = get_instance_or_noop_c!(StatsigUser, &user_ref);
    let config_name = unwrap_or_noop!(c_char_to_string(config_name));

    statsig.manually_log_dynamic_config_exposure(&user, &config_name);
}

// ------------------------
// Experiment Functions
// ------------------------

#[no_mangle]
pub extern "C" fn statsig_get_experiment(
    statsig_ref: u64,
    user_ref: u64,
    experiment_name: *const c_char,
    options_json: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null());
    let experiment_name = unwrap_or_return!(c_char_to_string(experiment_name), null());

    let experiment = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ExperimentEvaluationOptions>(&opts) {
            Ok(options) => statsig.get_experiment_with_options(&user, &experiment_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null();
            }
        },
        None => statsig.get_experiment(&user, &experiment_name),
    };

    let result = json!(experiment).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_manually_log_experiment_exposure(
    statsig_ref: u64,
    user_ref: u64,
    experiment_name: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let user = get_instance_or_noop_c!(StatsigUser, &user_ref);
    let experiment_name = unwrap_or_noop!(c_char_to_string(experiment_name));

    statsig.manually_log_experiment_exposure(&user, &experiment_name);
}

// ------------------------
// Layer Functions
// ------------------------

#[no_mangle]
pub extern "C" fn statsig_get_layer(
    statsig_ref: u64,
    user_ref: u64,
    layer_name: *const c_char,
    options_json: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null());
    let layer_name = unwrap_or_return!(c_char_to_string(layer_name), null());

    let layer = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<LayerEvaluationOptions>(&opts) {
            Ok(options) => statsig.get_layer_with_options(&user, &layer_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null();
            }
        },
        None => statsig.get_layer(&user, &layer_name),
    };

    let result = json!(layer).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_log_layer_param_exposure(
    statsig_ref: u64,
    layer_json: *const c_char,
    param_name: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);

    let param_name = unwrap_or_noop!(c_char_to_string(param_name));
    let layer_json = unwrap_or_noop!(c_char_to_string(layer_json));

    statsig.log_layer_param_exposure_with_layer_json(layer_json, param_name);
}

#[no_mangle]
pub extern "C" fn statsig_manually_log_layer_parameter_exposure(
    statsig_ref: u64,
    user_ref: u64,
    layer_name: *const c_char,
    param_name: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let user = get_instance_or_noop_c!(StatsigUser, &user_ref);

    let param_name = unwrap_or_noop!(c_char_to_string(param_name));
    let layer_name = unwrap_or_noop!(c_char_to_string(layer_name));

    statsig.manually_log_layer_parameter_exposure(&user, &layer_name, param_name);
}
