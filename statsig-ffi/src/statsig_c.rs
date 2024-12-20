use crate::ffi_utils::{c_char_to_string, string_to_c_char};
use crate::{get_instance_or_noop_c, get_instance_or_return_c};
use serde_json::json;
use serde_json::Value;
use sigstat::instance_store::INST_STORE;
use sigstat::{get_instance_or_noop, log_d, log_e, unwrap_or_noop, unwrap_or_return, Statsig, StatsigOptions, StatsigRuntime, StatsigUser};
use std::collections::HashMap;
use std::os::raw::c_char;
use std::ptr::null;

const TAG: &str = "StatsigC";

#[no_mangle]
pub extern "C" fn statsig_create(
    sdk_key: *const c_char,
    options_ref: *const c_char,
) -> *const c_char {
    let sdk_key = c_char_to_string(sdk_key).unwrap();
    let options_ref = c_char_to_string(options_ref);
    let options = INST_STORE.get_with_optional_id::<StatsigOptions>(options_ref.as_ref());

    let inst = Statsig::new(&sdk_key, options);

    let ref_id = INST_STORE.add(inst).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create Statsig");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn statsig_release(statsig_ref: *const c_char) {
    if let Some(id) = c_char_to_string(statsig_ref) {
        INST_STORE.remove(&id);
    }
}

#[no_mangle]
pub extern "C" fn statsig_initialize(statsig_ref: *const c_char, callback: extern "C" fn()) {
    let statsig_ref = unwrap_or_noop!(c_char_to_string(statsig_ref));
    let statsig = get_instance_or_noop!(Statsig, &statsig_ref);

    let statsig_rt = StatsigRuntime::get_runtime();
    statsig_rt.runtime_handle.spawn(async move {
        if let Err(e) = statsig.initialize().await {
            log_e!(TAG, "Failed to initialize statsig: {}", e);
        }

        callback();
    });
}

#[no_mangle]
pub extern "C" fn statsig_initialize_blocking(statsig_ref: *const c_char) {
    let statsig_ref = unwrap_or_noop!(c_char_to_string(statsig_ref));
    let statsig = get_instance_or_noop!(Statsig, &statsig_ref);

    let statsig_rt = StatsigRuntime::get_runtime();
    statsig_rt.runtime_handle.block_on(async move {
        if let Err(e) = statsig.initialize().await {
            log_e!(TAG, "Failed to initialize statsig: {}", e);
        }
    });
}

#[no_mangle]
pub extern "C" fn statsig_flush_events(statsig_ref: *const c_char, callback: extern "C" fn()) {
    let statsig = get_instance_or_noop_c!(Statsig, statsig_ref);

    let statsig_rt = StatsigRuntime::get_runtime();
    statsig_rt.runtime_handle.spawn(async move {
        statsig.flush_events().await;

        callback();
    });
}

#[no_mangle]
pub extern "C" fn statsig_flush_events_blocking(statsig_ref: *const c_char) {
    let statsig = get_instance_or_noop_c!(Statsig, statsig_ref);

    let statsig_rt = StatsigRuntime::get_runtime();
    statsig_rt.runtime_handle.block_on(async move {
        log_d!(TAG, "Statsig flush events");
        statsig.flush_events().await;
        log_d!(TAG, "Flushed events");
    });
}

#[no_mangle]
pub extern "C" fn statsig_get_current_values(statsig_ref: *const c_char) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, statsig_ref, null());

    let values = statsig.get_current_values();
    let data = json!(values).to_string();
    string_to_c_char(data)
}

#[no_mangle]
pub extern "C" fn statsig_log_event(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    event_json: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, statsig_ref);
    let user = get_instance_or_noop_c!(StatsigUser, user_ref);
    let event_json = unwrap_or_noop!(c_char_to_string(event_json));
    let event = match serde_json::from_str::<HashMap<String, Value>>(&event_json) {
        Ok(map) => map,
        Err(_) => return,
    };
    let event_name = unwrap_or_noop!(event.get("name").and_then(|n| n.as_str()));

    let event_value_str = event.get("value").and_then(|n| n.as_str());

    let event_value = event_value_str.map(|v| v.to_string());

    let event_metadata: Option<HashMap<String, String>> = event
        .get("metadata")
        .and_then(|m| m.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or("").to_string()))
                .collect()
        });
    statsig.log_event(&user, event_name, event_value, event_metadata);
}

#[no_mangle]
pub extern "C" fn statsig_check_gate(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    gate_name: *const c_char,
) -> bool {
    let statsig = get_instance_or_return_c!(Statsig, statsig_ref, false);
    let user = get_instance_or_return_c!(StatsigUser, user_ref, false);
    let gate_name = unwrap_or_return!(c_char_to_string(gate_name), false);

    statsig.check_gate(&user, &gate_name)
}

#[no_mangle]
pub extern "C" fn statsig_get_feature_gate(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    gate_name: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, user_ref, null());
    let gate_name = unwrap_or_return!(c_char_to_string(gate_name), null());

    let result = json!(statsig.get_feature_gate(&user, &gate_name)).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_get_dynamic_config(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    config_name: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, user_ref, null());
    let config_name = unwrap_or_return!(c_char_to_string(config_name), null());

    let result = json!(statsig.get_dynamic_config(&user, &config_name)).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_get_experiment(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    experiment_name: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, user_ref, null());
    let experiment_name = unwrap_or_return!(c_char_to_string(experiment_name), null());

    let result = json!(statsig.get_experiment(&user, &experiment_name)).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_get_layer(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
    layer_name: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, user_ref, null());
    let layer_name = unwrap_or_return!(c_char_to_string(layer_name), null());

    let result = json!(statsig.get_layer(&user, &layer_name)).to_string();
    string_to_c_char(result)
}

#[no_mangle]
pub extern "C" fn statsig_log_layer_param_exposure(
    statsig_ref: *const c_char,
    layer_json: *const c_char,
    param_name: *const c_char,
) {
    let statsig = get_instance_or_noop_c!(Statsig, statsig_ref);

    let param_name = unwrap_or_noop!(c_char_to_string(param_name));
    let layer_json = unwrap_or_noop!(c_char_to_string(layer_json));

    statsig.log_layer_param_exposure(layer_json, param_name);
}

#[no_mangle]
pub extern "C" fn statsig_get_client_init_response(
    statsig_ref: *const c_char,
    user_ref: *const c_char,
) -> *const c_char {
    let statsig = get_instance_or_return_c!(Statsig, statsig_ref, null());
    let user = get_instance_or_return_c!(StatsigUser, user_ref, null());

    let result = statsig.get_client_init_response(&user);
    string_to_c_char(json!(result).to_string())
}
