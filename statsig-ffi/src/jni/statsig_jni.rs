use jni::sys::{jboolean, jclass, jint, jlong, jobject, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use std::collections::HashMap;

use jni::objects::{JClass, JMap, JObject, JString};
use serde::Deserialize;

use crate::jni::jni_utils::{jni_to_rust_hashmap, serialize_to_json_string};
use sigstat::instance_store::{OPTIONS_INSTANCES, STATSIG_INSTANCES, USER_INSTANCES};
use sigstat::{
    get_instance_or_else, get_instance_or_noop, get_instance_or_return, log_d, log_e, Statsig,
};

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCreate(
    mut env: JNIEnv,
    _class: JClass,
    sdk_key: JString,
    options_ref: jint,
) -> jint {
    // StatsigOptions::new(); // temp: enable logging

    let sdk_key: String = match env.get_string(&sdk_key) {
        Ok(s) => s.into(),
        Err(_) => return -1,
    };

    let options = match OPTIONS_INSTANCES.get(options_ref) {
        Some(opt) => Some(opt),
        None => None,
    };

    let inst = Statsig::new(&sdk_key, options);

    let id = STATSIG_INSTANCES.add(inst);
    id
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigRelease(
    _env: JNIEnv,
    _class: JClass,
    id: jint,
) {
    STATSIG_INSTANCES.release(id);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigInitialize(
    env: JNIEnv,
    _class: JClass,
    statsig_ref: jint,
    callback: JObject,
) {
    log_d!("Statsig Init {}", statsig_ref);

    let statsig = get_instance_or_noop!(STATSIG_INSTANCES, statsig_ref);

    if callback.is_null() {
        log_e!("Callback is null");
        return;
    }

    let vm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(_) => {
            log_e!("Failed to get Java VM");
            return;
        }
    };

    let global_callback = env
        .new_global_ref(callback)
        .expect("Failed to create global ref");

    statsig.initialize_with_callback(move || {
        let mut env = vm.attach_current_thread().unwrap();

        let result = env.call_method(global_callback.as_obj(), "run", "()V", &[]);
        if result.is_err() {
            log_e!("Failed to call callback");
        }

        drop(global_callback);
    });
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetClientInitResponse(
    env: JNIEnv,
    _class: JClass,
    statsig_ref: jint,
    user_ref: jint,
) -> jstring {
    let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
        return std::ptr::null_mut();
    });
    let user = get_instance_or_else!(USER_INSTANCES, user_ref, { return std::ptr::null_mut() });

    let response = statsig.get_client_init_response(user.as_ref());

    let response_str = match serialize_to_json_string(&response) {
        Some(json_str) => json_str,
        None => return std::ptr::null_mut(),
    };

    match env.new_string(response_str) {
        Ok(java_str) => java_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigShutdown(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jint,
    callback: JObject,
) {
    log_d!("Statsig Shutdown {}", statsig_ref);

    let statsig = get_instance_or_noop!(STATSIG_INSTANCES, statsig_ref);

    if callback.is_null() {
        log_e!("Callback is null");
        return;
    }

    let _ = statsig.shutdown();
    let _ = env.call_method(callback, "run", "()V", &[]);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCheckGate(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jint,
    user_ref: jint,
    gate_name: JString,
) -> jboolean {
    let statsig = get_instance_or_return!(STATSIG_INSTANCES, statsig_ref, JNI_FALSE);
    let user = get_instance_or_return!(USER_INSTANCES, user_ref, JNI_FALSE);

    let gate_name: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return JNI_FALSE,
    };

    match statsig.check_gate(user.as_ref(), &gate_name) {
        true => JNI_TRUE,
        false => JNI_FALSE,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetExperiment(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jint,
    user_ref: jint,
    exper_name: JString,
) -> jstring {
    let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
        return std::ptr::null_mut();
    });
    let user = get_instance_or_else!(USER_INSTANCES, user_ref, { return std::ptr::null_mut() });

    let exper_name: String = match env.get_string(&exper_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_experiment(user.as_ref(), &exper_name);

    let result_str = match serialize_to_json_string(&result) {
        Some(json_str) => json_str,
        None => return std::ptr::null_mut(),
    };

    match env.new_string(result_str) {
        Ok(java_str) => java_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetLayer(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jint,
    user_ref: jint,
    layer_name: JString,
) -> jstring {
    let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
        return std::ptr::null_mut();
    });
    let user = get_instance_or_else!(USER_INSTANCES, user_ref, { return std::ptr::null_mut() });

    let layer_name: String = match env.get_string(&layer_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_layer(user.as_ref(), &layer_name);

    let result_str = match serialize_to_json_string(&result) {
        Some(json_str) => json_str,
        None => return std::ptr::null_mut(),
    };

    match env.new_string(result_str) {
        Ok(java_str) => java_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetDynamicConfig(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jint,
    user_ref: jint,
    config_name: JString,
) -> jstring {
    let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
        return std::ptr::null_mut();
    });
    let user = get_instance_or_else!(USER_INSTANCES, user_ref, { return std::ptr::null_mut() });

    let config_name: String = match env.get_string(&config_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_dynamic_config(user.as_ref(), &config_name);

    let result_str = match serialize_to_json_string(&result) {
        Some(json_str) => json_str,
        None => return std::ptr::null_mut(),
    };

    match env.new_string(result_str) {
        Ok(java_str) => java_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFeatureGate(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: jint,
    user_ref: jint,
    gate_name: JString,
) -> jstring {
    let statsig = get_instance_or_else!(STATSIG_INSTANCES, statsig_ref, {
        return std::ptr::null_mut();
    });
    let user = get_instance_or_else!(USER_INSTANCES, user_ref, { return std::ptr::null_mut() });

    let gate_name: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_feature_gate(user.as_ref(), &gate_name);

    let result_str = match serialize_to_json_string(&result) {
        Some(json_str) => json_str,
        None => return std::ptr::null_mut(),
    };

    match env.new_string(result_str) {
        Ok(java_str) => java_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigLogEvent(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jint,
    user_ref: jint,
    event_name: JString,
    value: JString,
    metadata: JObject,
) {
    let statsig = get_instance_or_noop!(STATSIG_INSTANCES, statsig_ref);
    let user = get_instance_or_noop!(USER_INSTANCES, user_ref);

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
        Ok(map) => Some(map),
        Err(_) => None,
    };

    statsig.log_event(user.as_ref(), &event_name, value, metadata);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigFlushEvents(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jint,
    callback: JObject,
) {
    let statsig = get_instance_or_noop!(STATSIG_INSTANCES, statsig_ref);

    if callback.is_null() {
        log_e!("Callback is null");
        return;
    }

    let _ = statsig.flush_events();
    let _ = env.call_method(callback, "run", "()V", &[]);
}
