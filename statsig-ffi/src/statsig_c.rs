use crate::ffi_utils::{c_char_to_string, extract_opt_bool, string_to_c_char, SafeOptBool};
use crate::{get_instance_or_noop_c, get_instance_or_return_c};
use serde_json::json;
use serde_json::Value;
use statsig_rust::{
    log_d, log_e, unwrap_or_else, unwrap_or_noop, unwrap_or_return, ClientInitResponseOptions,
    DynamicConfigEvaluationOptions, ExperimentEvaluationOptions, FeatureGateEvaluationOptions,
    InstanceRegistry, LayerEvaluationOptions, ParameterStoreEvaluationOptions, Statsig,
    StatsigOptions, StatsigUser,
};
use std::collections::HashMap;
use std::os::raw::c_char;
use std::ptr::null_mut;

const TAG: &str = "StatsigC";

#[no_mangle]
pub extern "C" fn statsig_create(sdk_key: *const c_char, options_ref: u64) -> u64 {
    let sdk_key = unwrap_or_else!(c_char_to_string(sdk_key), {
        log_e!(TAG, "Failed to convert 'sdk_key' to Rust string");
        return 0;
    });

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

    let result = statsig
        .statsig_runtime
        .clone()
        .spawn(TAG, move |_| async move {
            if let Err(e) = statsig.initialize().await {
                log_e!(TAG, "Failed to initialize statsig: {e}");
            }

            callback();
        });

    if let Err(e) = result {
        log_e!(TAG, "Failed to spawn statsig initialize task: {e}");
        callback();
    }
}

#[no_mangle]
pub extern "C" fn statsig_initialize_with_details(
    statsig_ref: u64,
    callback: extern "C" fn(result: *mut c_char),
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(e) => {
            log_e!(TAG, "Failed to get runtime handle: {}", e);
            callback(null_mut());
            return;
        }
    };

    rt_handle.spawn(async move {
        let details = match statsig.initialize_with_details().await {
            Ok(d) => d,
            Err(e) => {
                log_e!(TAG, "Failed to init statsig with details: {}", e);
                callback(null_mut());
                return;
            }
        };

        let json_str = match serde_json::to_string(&details) {
            Ok(json_str) => json_str,
            Err(e) => {
                log_e!(TAG, "Failed to initialize statsig with details: {}", e);
                callback(null_mut());
                return;
            }
        };

        callback(string_to_c_char(json_str));
    });
}

#[no_mangle]
pub extern "C" fn statsig_initialize_blocking(statsig_ref: u64) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(e) => {
            log_e!(TAG, "Failed to get runtime handle: {}", e);
            return;
        }
    };

    rt_handle.block_on(async move {
        if let Err(e) = statsig.initialize().await {
            log_e!(TAG, "Failed to initialize statsig: {}", e);
        }
    });
}

#[no_mangle]
pub extern "C" fn statsig_shutdown(statsig_ref: u64, callback: extern "C" fn()) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(e) => {
            log_e!(TAG, "Failed to get runtime handle: {}", e);
            return;
        }
    };

    rt_handle.spawn(async move {
        if let Err(e) = statsig.shutdown().await {
            log_e!(TAG, "Failed to gracefully shutdown Statsig: {}", e);
        }
        callback();
    });
}

#[no_mangle]
pub extern "C" fn statsig_shutdown_blocking(statsig_ref: u64) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(e) => {
            log_e!(TAG, "Failed to get runtime handle: {}", e);
            return;
        }
    };

    rt_handle.block_on(async move {
        if let Err(e) = statsig.shutdown().await {
            log_e!(TAG, "Failed to shutdown statsig: {}", e);
        }
    });
}

#[no_mangle]
pub extern "C" fn statsig_flush_events(statsig_ref: u64, callback: extern "C" fn()) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);

    let result = statsig
        .statsig_runtime
        .clone()
        .spawn(TAG, move |_| async move {
            statsig.flush_events().await;

            callback();
        });

    if let Err(e) = result {
        log_e!(TAG, "Failed to spawn statsig flush events task: {e}");
        callback();
    }
}

#[no_mangle]
pub extern "C" fn statsig_flush_events_blocking(statsig_ref: u64) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);

    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(e) => {
            log_e!(TAG, "Failed to get runtime handle: {}", e);
            return;
        }
    };

    rt_handle.block_on(async move {
        log_d!(TAG, "Statsig flush events");
        statsig.flush_events().await;
        log_d!(TAG, "Flushed events");
    });
}

#[no_mangle]
pub extern "C" fn statsig_get_current_values(statsig_ref: u64) -> *mut c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null_mut());

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
) -> *mut c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null_mut());

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ClientInitResponseOptions>(&opts) {
            Ok(options) => options,
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null_mut();
            }
        },
        None => ClientInitResponseOptions::default(),
    };

    let result = statsig.get_client_init_response_with_options_as_string(&user, &options);
    string_to_c_char(result)
}

// ------------------------------
// Get Parameter Store Functions
// ------------------------------

#[no_mangle]
pub extern "C" fn statsig_get_parameter_store_with_options(
    statsig_ref: u64,
    parameter_store_name: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null_mut());
    let param_store_name = unwrap_or_return!(c_char_to_string(parameter_store_name), null_mut());

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ParameterStoreEvaluationOptions>(&opts) {
            Ok(options) => options,
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null_mut();
            }
        },
        None => ParameterStoreEvaluationOptions {
            disable_exposure_logging: (false),
        },
    };

    let result = statsig.get_parameter_store_with_options(&param_store_name, options);
    string_to_c_char(json!(result).to_string())
}

#[no_mangle]
pub extern "C" fn statsig_get_string_parameter_from_parameter_store(
    statsig_ref: u64,
    user_ref: u64,
    parameter_store_name: *const c_char,
    param_name: *const c_char,
    default_value: *mut c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, default_value);
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, default_value);

    let parameter_store_name =
        unwrap_or_return!(c_char_to_string(parameter_store_name), default_value);

    let parameter_name = unwrap_or_return!(c_char_to_string(param_name), default_value);

    let default_value: Option<String> = c_char_to_string(default_value);

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ParameterStoreEvaluationOptions>(&opts) {
            Ok(options) => Some(options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                None
            }
        },
        None => Some(ParameterStoreEvaluationOptions {
            disable_exposure_logging: (false),
        }),
    };

    let result = statsig.get_string_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value,
        options,
    );

    match result {
        Some(result) => string_to_c_char(result),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn statsig_get_bool_parameter_from_parameter_store(
    statsig_ref: u64,
    user_ref: u64,
    parameter_store_name: *const c_char,
    param_name: *const c_char,
    default: SafeOptBool,
    options_json: *const c_char,
) -> bool {
    let default_val = unwrap_or_return!(extract_opt_bool(default), false);
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, default_val);
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, default_val);

    let parameter_store_name =
        unwrap_or_return!(c_char_to_string(parameter_store_name), default_val);

    let parameter_name = unwrap_or_return!(c_char_to_string(param_name), default_val);

    let default_value: Option<bool> = Some(default_val);

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ParameterStoreEvaluationOptions>(&opts) {
            Ok(options) => Some(options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                None
            }
        },
        None => Some(ParameterStoreEvaluationOptions {
            disable_exposure_logging: (false),
        }),
    };

    let result = statsig.get_boolean_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value,
        options,
    );

    match result {
        Some(result) => result,
        None => default_val,
    }
}

#[no_mangle]
pub extern "C" fn statsig_get_float64_parameter_from_parameter_store(
    statsig_ref: u64,
    user_ref: u64,
    parameter_store_name: *const c_char,
    param_name: *const c_char,
    default: f64,
    options_json: *const c_char,
) -> f64 {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, default);
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, default);

    let parameter_store_name = unwrap_or_return!(c_char_to_string(parameter_store_name), default);

    let parameter_name = unwrap_or_return!(c_char_to_string(param_name), default);

    let default_value: Option<f64> = Some(default);

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ParameterStoreEvaluationOptions>(&opts) {
            Ok(options) => Some(options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                None
            }
        },
        None => Some(ParameterStoreEvaluationOptions {
            disable_exposure_logging: (false),
        }),
    };

    let result = statsig.get_float_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value,
        options,
    );

    match result {
        Some(result) => result,
        None => default,
    }
}

#[no_mangle]
pub extern "C" fn statsig_get_int_parameter_from_parameter_store(
    statsig_ref: u64,
    user_ref: u64,
    parameter_store_name: *const c_char,
    param_name: *const c_char,
    default: i64,
    options_json: *const c_char,
) -> i64 {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, default);
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, default);

    let parameter_store_name = unwrap_or_return!(c_char_to_string(parameter_store_name), default);

    let parameter_name = unwrap_or_return!(c_char_to_string(param_name), default);

    let default_value: Option<i64> = Some(default);

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ParameterStoreEvaluationOptions>(&opts) {
            Ok(options) => Some(options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                None
            }
        },
        None => Some(ParameterStoreEvaluationOptions {
            disable_exposure_logging: (false),
        }),
    };

    let result = statsig.get_integer_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value,
        options,
    );

    match result {
        Some(result) => result,
        None => default,
    }
}

#[no_mangle]
pub extern "C" fn statsig_get_object_parameter_from_parameter_store(
    statsig_ref: u64,
    user_ref: u64,
    parameter_store_name: *const c_char,
    param_name: *const c_char,
    default: *const c_char,
    options_json: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, std::ptr::null_mut());

    let parameter_store_name =
        unwrap_or_return!(c_char_to_string(parameter_store_name), std::ptr::null_mut());

    let parameter_name = unwrap_or_return!(c_char_to_string(param_name), std::ptr::null_mut());

    let default_value_str = unwrap_or_return!(c_char_to_string(default), std::ptr::null_mut());

    let default_value: Option<HashMap<String, Value>> =
        serde_json::from_str(default_value_str.as_str()).ok();

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ParameterStoreEvaluationOptions>(&opts) {
            Ok(options) => Some(options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                None
            }
        },
        None => Some(ParameterStoreEvaluationOptions {
            disable_exposure_logging: (false),
        }),
    };

    let result = statsig.get_object_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value,
        options,
    );

    let result = json!(result).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_get_array_parameter_from_parameter_store(
    statsig_ref: u64,
    user_ref: u64,
    parameter_store_name: *const c_char,
    param_name: *const c_char,
    default: *const c_char,
    options_json: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, std::ptr::null_mut());

    let parameter_store_name =
        unwrap_or_return!(c_char_to_string(parameter_store_name), std::ptr::null_mut());

    let parameter_name = unwrap_or_return!(c_char_to_string(param_name), std::ptr::null_mut());

    let default_value_str = unwrap_or_return!(c_char_to_string(default), std::ptr::null_mut());

    let default_value: Option<Vec<Value>> = serde_json::from_str(default_value_str.as_str()).ok();

    let options = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ParameterStoreEvaluationOptions>(&opts) {
            Ok(options) => Some(options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                None
            }
        },
        None => Some(ParameterStoreEvaluationOptions {
            disable_exposure_logging: (false),
        }),
    };

    let result = statsig.get_array_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value,
        options,
    );

    let result = json!(result).to_string();
    string_to_c_char(result)
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
) -> *mut c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null_mut());
    let gate_name = unwrap_or_return!(c_char_to_string(gate_name), null_mut());

    let gate = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<FeatureGateEvaluationOptions>(&opts) {
            Ok(options) => statsig.get_feature_gate_with_options(&user, &gate_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null_mut();
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
) -> *mut c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null_mut());
    let config_name = unwrap_or_return!(c_char_to_string(config_name), null_mut());

    let config = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<DynamicConfigEvaluationOptions>(&opts) {
            Ok(options) => statsig.get_dynamic_config_with_options(&user, &config_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null_mut();
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
) -> *mut c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null_mut());
    let experiment_name = unwrap_or_return!(c_char_to_string(experiment_name), null_mut());

    let experiment = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<ExperimentEvaluationOptions>(&opts) {
            Ok(options) => statsig.get_experiment_with_options(&user, &experiment_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null_mut();
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
) -> *mut c_char {
    let statsig = get_instance_or_return_c!(Statsig, &statsig_ref, null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &user_ref, null_mut());
    let layer_name = unwrap_or_return!(c_char_to_string(layer_name), null_mut());

    let layer = match c_char_to_string(options_json) {
        Some(opts) => match serde_json::from_str::<LayerEvaluationOptions>(&opts) {
            Ok(options) => statsig.get_layer_with_options(&user, &layer_name, options),
            Err(e) => {
                log_e!(TAG, "Failed to parse options: {}", e);
                return null_mut();
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

// ------------------------
// Override Functions
// ------------------------

#[no_mangle]
pub extern "C" fn statsig_override_gate(
    statsig_ref: u64,
    gate_name: *const c_char,
    value: bool,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let gate_name = unwrap_or_noop!(c_char_to_string(gate_name));
    let id = c_char_to_string(id);

    statsig.override_gate(&gate_name, value, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_override_dynamic_config(
    statsig_ref: u64,
    config_name: *const c_char,
    value_json: *const c_char,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let config_name = unwrap_or_noop!(c_char_to_string(config_name));
    let value_json = unwrap_or_noop!(c_char_to_string(value_json));
    let id = c_char_to_string(id);

    let value = match serde_json::from_str::<HashMap<String, Value>>(&value_json) {
        Ok(map) => map,
        Err(e) => {
            log_e!(
                TAG,
                "Failed to parse value JSON for override_dynamic_config: {}",
                e
            );
            return;
        }
    };

    statsig.override_dynamic_config(&config_name, value, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_override_experiment(
    statsig_ref: u64,
    experiment_name: *const c_char,
    value_json: *const c_char,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let experiment_name = unwrap_or_noop!(c_char_to_string(experiment_name));
    let value_json = unwrap_or_noop!(c_char_to_string(value_json));
    let id = c_char_to_string(id);

    let value = match serde_json::from_str::<HashMap<String, Value>>(&value_json) {
        Ok(map) => map,
        Err(e) => {
            log_e!(
                TAG,
                "Failed to parse value JSON for override_experiment: {}",
                e
            );
            return;
        }
    };

    statsig.override_experiment(&experiment_name, value, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_override_experiment_by_group_name(
    statsig_ref: u64,
    experiment_name: *const c_char,
    group_name: *const c_char,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let experiment_name = unwrap_or_noop!(c_char_to_string(experiment_name));
    let group_name = unwrap_or_noop!(c_char_to_string(group_name));
    let id = c_char_to_string(id);

    statsig.override_experiment_by_group_name(&experiment_name, &group_name, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_override_layer(
    statsig_ref: u64,
    layer_name: *const c_char,
    value_json: *const c_char,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let layer_name = unwrap_or_noop!(c_char_to_string(layer_name));
    let value_json = unwrap_or_noop!(c_char_to_string(value_json));
    let id = c_char_to_string(id);

    let value = match serde_json::from_str::<HashMap<String, Value>>(&value_json) {
        Ok(map) => map,
        Err(e) => {
            log_e!(TAG, "Failed to parse value JSON for override_layer: {}", e);
            return;
        }
    };

    statsig.override_layer(&layer_name, value, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_remove_gate_override(
    statsig_ref: u64,
    gate_name: *const c_char,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let gate_name = unwrap_or_noop!(c_char_to_string(gate_name));
    let id = c_char_to_string(id);

    statsig.remove_gate_override(&gate_name, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_remove_dynamic_config_override(
    statsig_ref: u64,
    config_name: *const c_char,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let config_name = unwrap_or_noop!(c_char_to_string(config_name));
    let id = c_char_to_string(id);

    statsig.remove_dynamic_config_override(&config_name, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_remove_experiment_override(
    statsig_ref: u64,
    experiment_name: *const c_char,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let experiment_name = unwrap_or_noop!(c_char_to_string(experiment_name));
    let id = c_char_to_string(id);

    statsig.remove_experiment_override(&experiment_name, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_remove_layer_override(
    statsig_ref: u64,
    layer_name: *const c_char,
    id: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    let layer_name = unwrap_or_noop!(c_char_to_string(layer_name));
    let id = c_char_to_string(id);

    statsig.remove_layer_override(&layer_name, id.as_deref());
}

#[no_mangle]
pub extern "C" fn statsig_remove_all_overrides(statsig_ref: u64) {
    let statsig = get_instance_or_noop_c!(Statsig, &statsig_ref);
    statsig.remove_all_overrides();
}
