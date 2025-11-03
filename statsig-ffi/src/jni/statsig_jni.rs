use crate::ffi_utils::parse_json_to_str_map;
use crate::jni::jni_utils::{
    convert_java_check_gate_options_to_rust, convert_java_client_init_response_options_to_rust,
    convert_java_get_dynamic_config_options_to_rust, convert_java_get_experiment_options_to_rust,
    convert_java_get_layer_options_to_rust, jboolean_to_bool_unchecked, jni_to_rust_json_map,
    jstring_to_string,
};
use crate::jni::statsig_options_jni::StatsigOptionsJNI;
use crate::{get_instance_or_noop_c, get_instance_or_return_c};
use jni::objects::GlobalRef;
use jni::sys::{jboolean, jclass, jdouble, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use serde_json::Value;
use statsig_rust::statsig_metadata::StatsigMetadata;
use std::collections::HashMap;

use super::jni_utils::serialize_json_to_jstring;
use crate::jni::jni_utils::{jni_to_rust_hashmap, string_to_jstring};
use jni::{
    objects::{JClass, JObject, JString, JValue},
    JavaVM,
};
use statsig_rust::{log_d, log_e, InstanceRegistry, Statsig, StatsigUser};

const TAG: &str = "StatsigJNI";

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCreate(
    mut env: JNIEnv,
    _class: JClass,
    sdk_key: JString,
    options_ref: jlong,
) -> jlong {
    let sdk_key: String = match env.get_string(&sdk_key) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };

    let options_ref_u64 = options_ref as u64;
    let options_jni =
        InstanceRegistry::get::<StatsigOptionsJNI>(&options_ref_u64).map(|o| o.inner.clone());

    match options_jni {
        Some(_) => log_d!(TAG, "Loaded options for ref {}", options_ref_u64),
        None => log_d!(TAG, "No options loaded for ref {}", options_ref_u64),
    }

    let inst = Statsig::new(&sdk_key, options_jni);

    match InstanceRegistry::register(inst) {
        Some(id) => {
            log_d!(TAG, "Created Statsig instance with ID {}", id);
            id as jlong
        }
        None => {
            log_e!(TAG, "Failed to create Statsig instance");
            0
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRelease(
    _env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
) {
    let statsig_ref_u64 = statsig_ref as u64;
    InstanceRegistry::remove(&statsig_ref_u64);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigInitialize(
    env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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

    let global_callback = match try_new_global_ref(&env, callback) {
        Some(r) => r,
        None => return,
    };

    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(_) => {
            log_e!(TAG, "Failed to get runtime handle");
            fire_callback("initialize", &vm, global_callback);
            return;
        }
    };

    rt_handle.spawn(async move {
        if let Err(e) = statsig.initialize().await {
            log_e!(TAG, "Failed to initialize statsig: {}", e);
        }

        fire_callback("initialize", &vm, global_callback);
    });
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigInitializeWithDetails(
    env: JNIEnv,
    _class: jclass,
    statsig_ref: jlong,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

    let vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(_) => {
            log_e!(TAG, "Failed to get Java VM");
            return;
        }
    };

    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(_) => {
            log_e!(TAG, "Failed to get runtime handle");
            return;
        }
    };

    let global_callback = match try_new_global_ref(&env, callback) {
        Some(r) => r,
        None => return,
    };

    rt_handle.spawn(async move {
        let result = statsig.initialize_with_details().await;

        let mut env = match vm.attach_current_thread() {
            Ok(env) => env,
            Err(e) => {
                log_e!(TAG, "Failed to attach thread: {:?}", e);
                return;
            }
        };

        match result {
            Ok(details) => {
                let json_str = match serde_json::to_string(&details) {
                    Ok(s) => s,
                    Err(e) => {
                        log_e!(TAG, "Failed to serialize initialization details: {:?}", e);
                        return;
                    }
                };

                let jstr = match env.new_string(json_str) {
                    Ok(s) => s,
                    Err(e) => {
                        log_e!(TAG, "Failed to create JString: {:?}", e);
                        return;
                    }
                };

                let obj = JObject::from(jstr);
                let arg = JValue::Object(&obj);
                let _ = env.call_method(
                    global_callback.as_obj(),
                    "complete",
                    "(Ljava/lang/Object;)Z",
                    &[arg],
                );
            }
            Err(e) => {
                log_e!(TAG, "initialize with details failed: {:?}", e);

                let error_str = format!("Initialization failed: {e:?}");
                if let Ok(jstr) = env.new_string(error_str) {
                    let obj = JObject::from(jstr);
                    let _ = env.call_method(
                        global_callback.as_obj(),
                        "complete",
                        "(Ljava/lang/Object;)Z",
                        &[JValue::Object(&obj)],
                    );
                }
            }
        }
    });
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigShutdown(
    env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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

    let global_callback = match try_new_global_ref(&env, callback) {
        Some(r) => r,
        None => return,
    };

    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(_) => {
            log_e!(TAG, "Failed to get runtime handle");
            fire_callback("shutdown", &vm, global_callback);
            return;
        }
    };

    rt_handle.spawn(async move {
        if let Err(e) = statsig.shutdown().await {
            log_e!(TAG, "Failed to gracefully shutdown Statsig: {}", e);
        }

        match vm.attach_current_thread() {
            Ok(mut env) => {
                if let Err(e) = env.call_method(global_callback.as_obj(), "run", "()V", &[]) {
                    log_e!(TAG, "Failed to call callback: {:?}", e);
                }
            }
            Err(e) => {
                log_e!(TAG, "Failed to attach for callback: {:?}", e);
            }
        }
    });
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetClientInitResponse(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    user_ref: jlong,
    init_response_option: JObject,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

    let options = convert_java_client_init_response_options_to_rust(&mut env, init_response_option);

    let response = match options.as_ref() {
        Some(options) => statsig.get_client_init_response_with_options_as_string(&user, options),
        None => statsig.get_client_init_response_as_string(&user),
    };

    string_to_jstring(&mut env, response)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCheckGate(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jlong,
    user_ref: jlong,
    gate_name: JString,
    options: JObject,
) -> jboolean {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), JNI_FALSE);
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), JNI_FALSE);

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
    statsig_ref: jlong,
    gate_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    user_ref: jlong,
    exper_name: JString,
    options: JObject,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    user_ref: jlong,
    exper_name: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    let user = get_instance_or_noop_c!(StatsigUser, &(user_ref as u64));

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
    statsig_ref: jlong,
    exper_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());

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
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetExperimentByGroupName(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jlong,
    exper_name: JString,
    group_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let exper_name: String = match env.get_string(&exper_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    let group_name: String = match env.get_string(&group_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    let result = statsig.get_experiment_by_group_name(&exper_name, &group_name);
    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigManuallyLogLayerParamExposure(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jlong,
    user_ref: jlong,
    layer_name: JString,
    parameter_name: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    let user = get_instance_or_noop_c!(StatsigUser, &(user_ref as u64));

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
    statsig_ref: jlong,
    user_ref: jlong,
    layer_name: JString,
    options: JObject,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    layer_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    parameter_store_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    user_ref: jlong,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

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
        None,
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
    statsig_ref: jlong,
    user_ref: jlong,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: jboolean,
) -> jboolean {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), default_value);
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), default_value);

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
        None,
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
    statsig_ref: jlong,
    user_ref: jlong,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: jdouble,
) -> jdouble {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), default_value);
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), default_value);

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
        None,
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
    statsig_ref: jlong,
    user_ref: jlong,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: jlong,
) -> jlong {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), default_value);
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), default_value);

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
        None,
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
    statsig_ref: jlong,
    user_ref: jlong,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

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
        None,
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
    statsig_ref: jlong,
    user_ref: jlong,
    parameter_store_name: JString,
    parameter_name: JString,
    default_value: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

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
        None,
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
    statsig_ref: jlong,
    user_ref: jlong,
    config_name: JString,
    options: JObject,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    user_ref: jlong,
    config_name: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    let user = get_instance_or_noop_c!(StatsigUser, &(user_ref as u64));

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
    statsig_ref: jlong,
    config_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    user_ref: jlong,
    gate_name: JString,
    options: JObject,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    user_ref: jlong,
    cmab_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    let user = get_instance_or_return_c!(StatsigUser, &(user_ref as u64), std::ptr::null_mut());

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
    statsig_ref: jlong,
    user_ref: jlong,
    cmab_name: JString,
    rule_id: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    let user = get_instance_or_noop_c!(StatsigUser, &(user_ref as u64));

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
    statsig_ref: jlong,
    user_ref: jlong,
    gate_name: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    let user = get_instance_or_noop_c!(StatsigUser, &(user_ref as u64));

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
    statsig_ref: jlong,
    layer_json: JString,
    parameter_name: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    statsig_ref: jlong,
    user_ref: jlong,
    event_name: JString,
    value: JString,
    metadata: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    let user = get_instance_or_noop_c!(StatsigUser, &(user_ref as u64));

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

    let metadata = match jni_to_rust_hashmap(env, metadata) {
        Ok(m) => Some(m),
        Err(e) => {
            log_e!(TAG, "Failed to convert metadata: {:?}", e);
            return;
        }
    };

    statsig.log_event(user.as_ref(), &event_name, value, metadata);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogEventWithDouble(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    user_ref: jlong,
    event_name: JString,
    value: f64,
    metadata: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    let user = get_instance_or_noop_c!(StatsigUser, &(user_ref as u64));

    let event_name: String = match env.get_string(&event_name) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let value = Some(value);

    let metadata = match jni_to_rust_hashmap(env, metadata) {
        Ok(m) => Some(m),
        Err(e) => {
            log_e!(TAG, "Failed to convert metadata: {:?}", e);
            return;
        }
    };

    statsig.log_event_with_number(user.as_ref(), &event_name, value, metadata);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigIdentify(
    _env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    user_ref: jlong,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    let user = get_instance_or_noop_c!(StatsigUser, &(user_ref as u64));
    statsig.identify(user.as_ref());
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigFlushEvents(
    env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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

    let global_callback = match try_new_global_ref(&env, callback) {
        Some(r) => r,
        None => return,
    };

    let rt_handle = match statsig.statsig_runtime.get_handle() {
        Ok(handle) => handle,
        Err(_) => {
            log_e!(TAG, "Failed to get runtime handle");
            return;
        }
    };

    rt_handle.spawn(async move {
        statsig.flush_events().await;

        match vm.attach_current_thread() {
            Ok(mut env) => {
                if let Err(e) = env.call_method(global_callback.as_obj(), "run", "()V", &[]) {
                    log_e!(TAG, "Failed to call callback: {:?}", e);
                }
            }
            Err(e) => {
                log_e!(TAG, "Failed to attach for callback: {:?}", e);
            }
        }
    });
}

/// -------------------------
/// ---- local overrides ----
/// -------------------------
#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOverrideGate(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    gate_name: JString,
    id: JString,
    gate_value: jboolean,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    statsig_ref: jlong,
    config_name: JString,
    id: JString,
    value: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    statsig_ref: jlong,
    layer_name: JString,
    id: JString,
    value: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    statsig_ref: jlong,
    exp_name: JString,
    id: JString,
    value: JObject,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    statsig_ref: jlong,
    exp_name: JString,
    id: JString,
    group_name: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_updateStatsigMetadata(
    mut env: JNIEnv,
    _class: JClass,
    metadata: JString,
) {
    let metadata_str = jstring_to_string(&mut env, metadata);
    if let Some(m) = parse_json_to_str_map(metadata_str) {
        let os = m.get("os").map_or("unknown".to_string(), |s| s.clone());
        let arch = m.get("arch").map_or("unknown".to_string(), |s| s.clone());
        let language_version = m
            .get("languageVersion")
            .map_or("unknown".to_string(), |s| s.clone());
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
    statsig_ref: jlong,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    serialize_json_to_jstring(&mut env, &statsig.get_feature_gate_list())
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetDynamicConfigList(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jlong,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    serialize_json_to_jstring(&mut env, &statsig.get_dynamic_config_list())
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetExperimentList(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jlong,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    serialize_json_to_jstring(&mut env, &statsig.get_experiment_list())
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetParameterStoreList(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jlong,
) -> jstring {
    let statsig = get_instance_or_return_c!(Statsig, &(statsig_ref as u64), std::ptr::null_mut());
    serialize_json_to_jstring(&mut env, &statsig.get_parameter_store_list())
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRemoveGateOverride(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    gate_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    statsig_ref: jlong,
    config_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    statsig_ref: jlong,
    exp_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    statsig_ref: jlong,
    layer_name: JString,
    id: JString,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));

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
    _env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
) {
    let statsig = get_instance_or_noop_c!(Statsig, &(statsig_ref as u64));
    statsig.remove_all_overrides();
}

/// Attempts to create a GlobalRef from a local JObject. Logs error and returns None if it fails.
fn try_new_global_ref(env: &JNIEnv, obj: JObject) -> Option<GlobalRef> {
    match env.new_global_ref(obj) {
        Ok(r) => Some(r),
        Err(e) => {
            log_e!(TAG, "Failed to create global ref: {:?}", e);
            None
        }
    }
}

fn fire_callback(callback_name: &str, vm: &JavaVM, global_callback: GlobalRef) {
    let mut env = match vm.attach_current_thread() {
        Ok(env) => env,
        Err(e) => {
            log_e!(TAG, "Failed to attach current thread: {}", e);
            drop(global_callback);
            return;
        }
    };

    let result = env.call_method(global_callback.as_obj(), "run", "()V", &[]);
    if result.is_err() {
        log_e!(TAG, "Failed to call {} callback", callback_name);
    }

    drop(global_callback);
}
