use crate::ffi_utils::parse_json_to_str_map;
use crate::jni::jni_utils::{
    convert_java_check_gate_options_to_rust, convert_java_client_init_response_options_to_rust,
    convert_java_get_dynamic_config_options_to_rust, convert_java_get_experiment_options_to_rust,
    convert_java_get_layer_options_to_rust, jboolean_to_bool_unchecked, jni_to_rust_json_map,
    jstring_to_string,
};
use crate::{get_instance_or_noop_jni, get_instance_or_return_jni};
use jni::sys::{jboolean, jclass, jdouble, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use serde_json::Value;
use statsig_rust::statsig_metadata::StatsigMetadata;
use std::collections::HashMap;

use super::jni_utils::serialize_json_to_jstring;
use crate::jni::jni_utils::{jni_to_rust_hashmap, string_to_jstring};
use crate::jni::statsig_options_jni::StatsigOptionsJNI;
use jni::objects::{JClass, JObject, JString};
use statsig_rust::{log_e, InstanceRegistry, Statsig, StatsigUser};

const TAG: &str = "StatsigJNI";

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCreate(
    mut env: JNIEnv,
    _class: JClass,
    sdk_key: JString,
    options_ref: JString,
    statsig_metadata: JString,
) -> jstring {
    // StatsigOptions::new(); // temp: enable logging

    let sdk_key: String = match env.get_string(&sdk_key) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let options_inst_id = jstring_to_string(&mut env, options_ref);

    let options_jni =
        InstanceRegistry::get_with_optional_id::<StatsigOptionsJNI>(options_inst_id.as_ref());

    let inst = Statsig::new(&sdk_key, options_jni.map(|o| o.inner.clone()));
    update_statsig_metadata(&mut env, statsig_metadata);

    let id = InstanceRegistry::register(inst);
    match id {
        Some(id) => match env.new_string(id) {
            Ok(java_str) => java_str.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRelease(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
) {
    if let Some(id) = jstring_to_string(&mut env, statsig_ref) {
        InstanceRegistry::remove(&id);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigInitialize(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    if callback.is_null() {
        log_e!(TAG, "Callback is null");
        return;
    }

    let vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(_) => {
            log_e!(TAG, "Failed to get Java VM");
            return;
        }
    };

    let global_callback = env
        .new_global_ref(callback)
        .expect("Failed to create global ref");

    statsig.statsig_runtime.get_handle().block_on(async move {
        if let Err(e) = statsig.initialize().await {
            log_e!(TAG, "Failed to initialize statsig: {}", e);
        }

        let mut env = vm.attach_current_thread().unwrap();

        let result = env.call_method(global_callback.as_obj(), "run", "()V", &[]);
        if result.is_err() {
            log_e!(TAG, "Failed to call callback");
        }

        drop(global_callback);
    });
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigShutdown(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    if callback.is_null() {
        log_e!(TAG, "Callback is null");
        return;
    }

    let vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(_) => {
            log_e!(TAG, "Failed to get Java VM");
            return;
        }
    };

    let global_callback = env
        .new_global_ref(callback)
        .expect("Failed to create global ref");

    statsig.statsig_runtime.get_handle().block_on(async move {
        if let Err(e) = statsig.shutdown().await {
            log_e!(TAG, "Failed to gracefully shutdown StatsigPy: {}", e);
        }

        let mut env = vm.attach_current_thread().unwrap();

        let result = env.call_method(global_callback.as_obj(), "run", "()V", &[]);
        if result.is_err() {
            log_e!(TAG, "Failed to call callback");
        }

        drop(global_callback);
    });
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetClientInitResponse(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    user_ref: JString,
    init_response_option: JObject,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let options = convert_java_client_init_response_options_to_rust(&mut env, init_response_option);

    let response = match options.as_ref() {
        Some(options) => statsig.get_client_init_response_with_options(&user, options),
        None => statsig.get_client_init_response(&user),
    };

    serialize_json_to_jstring(&mut env, &response)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCheckGate(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    gate_name: JString,
    options: JObject,
) -> jboolean {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, JNI_FALSE);
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, JNI_FALSE);

    let options = convert_java_check_gate_options_to_rust(&mut env, options);
    let gate_name: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return JNI_FALSE,
    };
    match options {
        Some(options) => {
            match statsig.check_gate_with_options(user.as_ref(), &gate_name, options) {
                true => JNI_TRUE,
                false => JNI_FALSE,
            }
        }
        None => match statsig.check_gate(user.as_ref(), &gate_name) {
            true => JNI_TRUE,
            false => JNI_FALSE,
        },
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFieldsNeededForGate(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    gate_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());

    let gate_name: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    serialize_json_to_jstring(
        &mut env,
        &statsig.get_fields_needed_for_gate(gate_name.as_str()),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetExperiment(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    exper_name: JString,
    options: JObject,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let exper_name: String = match env.get_string(&exper_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let options = convert_java_get_experiment_options_to_rust(&mut env, options);

    let result = match options {
        Some(options) => statsig.get_experiment_with_options(user.as_ref(), &exper_name, options),
        None => statsig.get_experiment(user.as_ref(), &exper_name),
    };

    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogExperimentExposure(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    exper_name: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);

    let exper_name: String = match env.get_string(&exper_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    statsig.manually_log_experiment_exposure(&user, &exper_name);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFieldsNeededForExperiment(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    exper_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());

    let exper_name: String = match env.get_string(&exper_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    serialize_json_to_jstring(
        &mut env,
        &statsig.get_fields_needed_for_experiment(exper_name.as_str()),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigManuallyLogLayerParamExposure(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    layer_name: JString,
    parameter_name: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);

    let layer_name: String = match env.get_string(&layer_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let parameter_name: String = match env.get_string(&parameter_name) {
        Ok(s) => s.into(),
        Err(_) => {
            log_e!(TAG, "Failed to convert parameter_name to Rust string");
            return;
        }
    };

    statsig.manually_log_layer_parameter_exposure(&user, &layer_name, parameter_name);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetLayer(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    layer_name: JString,
    options: JObject,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let layer_name: String = match env.get_string(&layer_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let options = convert_java_get_layer_options_to_rust(&mut env, options);

    let result = match options {
        Some(options) => statsig.get_layer_with_options(user.as_ref(), &layer_name, options),
        None => statsig.get_layer(user.as_ref(), &layer_name),
    };

    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFieldsNeededForLayer(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    layer_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());

    let layer_name: String = match env.get_string(&layer_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    serialize_json_to_jstring(
        &mut env,
        &statsig.get_fields_needed_for_layer(layer_name.as_str()),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetParameterStore(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    parameter_store_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());

    let parameter_store_name: String = match env.get_string(&parameter_store_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_parameter_store(&parameter_store_name);

    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetStringParameterFromParameterStore(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let parameter_store_name: String = match env.get_string(&parameter_store_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let parameter_name: String = match env.get_string(&parameter_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let default_value: Option<String> = match env.get_string(&default_value) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    let result = statsig.get_string_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value,
    );

    match result {
        Some(result) => string_to_jstring(&mut env, result),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetBooleanParameterFromParameterStore(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: jboolean,
) -> jboolean {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, default_value);
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, default_value);

    let parameter_store_name: String = match env.get_string(&parameter_store_name) {
        Ok(s) => s.into(),
        Err(_) => return default_value,
    };

    let parameter_name: String = match env.get_string(&parameter_name) {
        Ok(s) => s.into(),
        Err(_) => return default_value,
    };

    let default_value_bool: Option<bool> = match default_value {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };

    let result = statsig.get_boolean_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value_bool,
    );

    match result {
        Some(true) => JNI_TRUE,
        Some(false) => JNI_FALSE,
        None => default_value,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFloatParameterFromParameterStore(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: jdouble,
) -> jdouble {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, default_value);
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, default_value);

    let parameter_store_name: String = match env.get_string(&parameter_store_name) {
        Ok(s) => s.into(),
        Err(_) => return default_value,
    };

    let parameter_name: String = match env.get_string(&parameter_name) {
        Ok(s) => s.into(),
        Err(_) => return default_value,
    };

    let result = statsig.get_float_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        Some(default_value),
    );

    match result {
        Some(result) => result,
        None => default_value,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetIntegerParameterFromParameterStore(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: jlong,
) -> jlong {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, default_value);
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, default_value);

    let parameter_store_name: String = match env.get_string(&parameter_store_name) {
        Ok(s) => s.into(),
        Err(_) => return default_value,
    };

    let parameter_name: String = match env.get_string(&parameter_name) {
        Ok(s) => s.into(),
        Err(_) => return default_value,
    };

    let result = statsig.get_integer_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        Some(default_value),
    );

    match result {
        Some(result) => result,
        None => default_value,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetObjectParameterFromParameterStore(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let parameter_store_name: String = match env.get_string(&parameter_store_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let parameter_name: String = match env.get_string(&parameter_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let default_value_str: String = match env.get_string(&default_value) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let default_value_map: Option<HashMap<String, Value>> =
        serde_json::from_str(default_value_str.as_str()).ok();

    let result = statsig.get_object_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value_map,
    );

    match result {
        Some(result) => serialize_json_to_jstring(&mut env, &result),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetArrayParameterFromParameterStore(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let parameter_store_name: String = match env.get_string(&parameter_store_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let parameter_name: String = match env.get_string(&parameter_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let default_value_str: String = match env.get_string(&default_value) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let default_value_array: Option<Vec<Value>> =
        serde_json::from_str(default_value_str.as_str()).ok();

    let result = statsig.get_array_parameter_from_store(
        &user,
        &parameter_store_name,
        &parameter_name,
        default_value_array,
    );

    match result {
        Some(result) => serialize_json_to_jstring(&mut env, &result),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetDynamicConfig(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    config_name: JString,
    options: JObject,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let config_name: String = match env.get_string(&config_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let options = convert_java_get_dynamic_config_options_to_rust(&mut env, options);

    let result = match options {
        Some(options) => {
            statsig.get_dynamic_config_with_options(user.as_ref(), &config_name, options)
        }
        None => statsig.get_dynamic_config(user.as_ref(), &config_name),
    };

    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogDynamicConfigExposure(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    config_name: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);

    let config_name: String = match env.get_string(&config_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    statsig.manually_log_dynamic_config_exposure(&user, &config_name);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFieldsNeededForDynamicConfig(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    config_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());

    let config_name: String = match env.get_string(&config_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    serialize_json_to_jstring(
        &mut env,
        &statsig.get_fields_needed_for_dynamic_config(config_name.as_str()),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFeatureGate(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    gate_name: JString,
    options: JObject,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let gate_name: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    let options = convert_java_check_gate_options_to_rust(&mut env, options);

    let result = match options {
        Some(options) => statsig.get_feature_gate_with_options(user.as_ref(), &gate_name, options),
        None => statsig.get_feature_gate(user.as_ref(), &gate_name),
    };
    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetCMABRankedVariants(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    cmab_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let cmab_name: String = match env.get_string(&cmab_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_cmab_ranked_groups(user.as_ref(), &cmab_name);

    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogCMABExposure(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    cmab_name: JString,
    rule_id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);

    let cmab_name: String = match env.get_string(&cmab_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let rule_id: String = match env.get_string(&rule_id) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    statsig.log_cmab_exposure_for_group(user.as_ref(), &cmab_name, rule_id);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogGateExposure(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    gate_name: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);

    let gate_name: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    statsig.manually_log_gate_exposure(&user, &gate_name);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogLayerParamExposure(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    layer_json: JString,
    parameter_name: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let layer_json: String = match env.get_string(&layer_json) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let parameter_name: String = match env.get_string(&parameter_name) {
        Ok(s) => s.into(),
        Err(_) => {
            log_e!(TAG, "Failed to convert parameter_name to Rust string");
            return;
        }
    };

    statsig.log_layer_param_exposure_with_layer_json(layer_json, parameter_name);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogEvent(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    user_ref: JString,
    event_name: JString,
    value: JString,
    metadata: JObject,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);

    let event_name: String = match env.get_string(&event_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let value = if value.is_null() {
        None
    } else {
        match env.get_string(&value) {
            Ok(s) => Some(s.into()),
            Err(_) => None,
        }
    };

    let metadata = jni_to_rust_hashmap(env, metadata).ok();

    statsig.log_event(user.as_ref(), &event_name, value, metadata);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogEventWithLong(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    user_ref: JString,
    event_name: JString,
    value: i64,
    metadata: JObject,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);

    let event_name: String = match env.get_string(&event_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let value = Some(value as f64);

    let metadata = jni_to_rust_hashmap(env, metadata).ok();

    statsig.log_event_with_number(user.as_ref(), &event_name, value, metadata);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogEventWithDouble(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    user_ref: JString,
    event_name: JString,
    value: f64,
    metadata: JObject,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);

    let event_name: String = match env.get_string(&event_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let value = Some(value);

    let metadata = jni_to_rust_hashmap(env, metadata).ok();

    statsig.log_event_with_number(user.as_ref(), &event_name, value, metadata);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigIdentify(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    user_ref: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    let user = get_instance_or_noop_jni!(StatsigUser, &mut env, user_ref);
    statsig.identify(user.as_ref());
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigFlushEvents(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    if callback.is_null() {
        log_e!(TAG, "Callback is null");
        return;
    }

    let vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(_) => {
            log_e!(TAG, "Failed to get Java VM");
            return;
        }
    };

    let global_callback = env
        .new_global_ref(callback)
        .expect("Failed to create global ref");

    statsig.statsig_runtime.get_handle().block_on(async move {
        statsig.flush_events().await;

        let mut env = vm.attach_current_thread().unwrap();

        let result = env.call_method(global_callback.as_obj(), "run", "()V", &[]);
        if result.is_err() {
            log_e!(TAG, "Failed to call callback");
        }

        drop(global_callback);
    });
}

/// -------------------------
/// ---- local overrides ----
/// -------------------------
#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOverrideGate(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    gate_name: JString,
    gate_value: jboolean,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let gate_name_rust: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    let gate_value_rust = jboolean_to_bool_unchecked(gate_value);
    statsig.override_gate(&gate_name_rust, gate_value_rust, id_rust.as_deref());
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOverrideDynamicConfig(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    config_name: JString,
    value: JObject,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let config_name_rust: String = match env.get_string(&config_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    match jni_to_rust_json_map(&mut env, value) {
        Ok(value_rust) => {
            statsig.override_dynamic_config(&config_name_rust, value_rust, id_rust.as_deref());
        }
        Err(e) => {
            log_e!(
                TAG,
                "Override Dynamic Config, Failed to convert JSON map: {:?}",
                e
            );
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOverrideLayer(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    layer_name: JString,
    value: JObject,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let layer_name_rust: String = match env.get_string(&layer_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    match jni_to_rust_json_map(&mut env, value) {
        Ok(value_rust) => {
            statsig.override_layer(&layer_name_rust, value_rust, id_rust.as_deref());
        }
        Err(e) => {
            log_e!(TAG, "Override Layer, Failed to convert JSON map: {:?}", e);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOverrideExperiment(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    exp_name: JString,
    value: JObject,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let exp_name_rust: String = match env.get_string(&exp_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    match jni_to_rust_json_map(&mut env, value) {
        Ok(value_rust) => {
            statsig.override_experiment(&exp_name_rust, value_rust, id_rust.as_deref());
        }
        Err(e) => {
            log_e!(TAG, "Override Layer, Failed to convert JSON map: {:?}", e);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOverrideExperimentByGroupName(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    exp_name: JString,
    group_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let exp_name_rust: String = match env.get_string(&exp_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let group_name_rust: String = match env.get_string(&group_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    statsig.override_experiment_by_group_name(&exp_name_rust, &group_name_rust, id_rust.as_deref());
}

fn update_statsig_metadata(env: &mut JNIEnv, metadata: JString) {
    let metadata_str = jstring_to_string(env, metadata);
    if let Some(m) = parse_json_to_str_map(metadata_str) {
        let os = m.get("os").map_or("unknown".to_string(), |s| s.clone());
        let arch = m.get("arch").map_or("unknown".to_string(), |s| s.clone());
        let language_version = m.get("arch").map_or("unknown".to_string(), |s| s.clone());
        StatsigMetadata::update_values(
            "statsig-server-core-java".to_string(),
            os,
            arch,
            language_version,
        );
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFeatureGateList(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    serialize_json_to_jstring(&mut env, &statsig.get_feature_gate_list())
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetDynamicConfigList(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    serialize_json_to_jstring(&mut env, &statsig.get_dynamic_config_list())
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetExperimentList(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    serialize_json_to_jstring(&mut env, &statsig.get_experiment_list())
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetParameterStoreList(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    serialize_json_to_jstring(&mut env, &statsig.get_parameter_store_list())
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRemoveGateOverride(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    gate_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let gate_name_rust: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    statsig.remove_gate_override(&gate_name_rust, id_rust.as_deref());
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRemoveDynamicConfigOverride(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    config_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let config_name_rust: String = match env.get_string(&config_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    statsig.remove_dynamic_config_override(&config_name_rust, id_rust.as_deref());
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRemoveExperimentOverride(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    exp_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let exp_name_rust: String = match env.get_string(&exp_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    statsig.remove_experiment_override(&exp_name_rust, id_rust.as_deref());
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRemoveLayerOverride(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    layer_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

    let layer_name_rust: String = match env.get_string(&layer_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let id_rust: Option<String> = match env.get_string(&id) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    };

    statsig.remove_layer_override(&layer_name_rust, id_rust.as_deref());
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRemoveAllOverrides(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    statsig.remove_all_overrides();
}
