use std::ptr::null;
use jni::JNIEnv;
use jni::sys::{jboolean, jclass, jlong, JNI_FALSE, JNI_TRUE, jobject, jstring};
use lazy_static::lazy_static;
use crate::statsig_c::statsig_check_gate;

use std::sync::{Arc, Mutex};
use jni::objects::{JClass, JObject, JString};
use statsig::{log_d, log_e, Statsig, StatsigOptions, StatsigUser};
use crate::instance_manager::{InstanceManager, OPTIONS_INSTANCES, STATSIG_INSTANCES, USER_INSTANCES};

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCreate(
    mut env: JNIEnv,
    _class: JClass,
    sdk_key: JString,
    options_ref: jlong,
) -> jlong {
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
    id: jlong,
) {
    STATSIG_INSTANCES.release(id);
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigInitialize(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    callback: JObject,
) {
    log_d!("Statsig Init {}", statsig_ref);

    let statsig = match STATSIG_INSTANCES.get(statsig_ref) {
        Some(s) => s,
        None => return,
    };

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

    let global_callback = env.new_global_ref(callback).expect("Failed to create global ref");

    statsig.initialize_with_callback(move || {
        let mut env = vm.attach_current_thread().unwrap();

        let result = env.call_method(global_callback.as_obj(), "run", "()V", &[]);
        if result.is_err() {
            log_e!("Failed to call callback");
        }
    });
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetClientInitResponse(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: jlong,
    statsig_user_ref: jlong
) -> jstring {
    let statsig = match STATSIG_INSTANCES.get(statsig_ref) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let user = match USER_INSTANCES.get(statsig_user_ref) {
        Some(u) => u,
        None => return std::ptr::null_mut(),
    };

    let response = statsig.get_client_init_response(user.as_ref());

    let response_str = match serde_json::to_string(&response) {
        Ok(json_str) => json_str,
        Err(_) => return std::ptr::null_mut(),
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
    statsig_ref: jlong,
    callback: JObject,
) {
    log_d!("Statsig Shutdown {}", statsig_ref);

    let statsig = match STATSIG_INSTANCES.get(statsig_ref) {
        Some(s) => s,
        None => return,
    };

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
    class: jclass,
    statsig_ref: jlong,
    user_ref: jlong,
    gate_name: JString,
) -> jboolean {
    let statsig = match STATSIG_INSTANCES.get(statsig_ref) {
        Some(s) => s,
        None => return JNI_FALSE,
    };

    let user = match USER_INSTANCES.get(user_ref) {
        Some(u) => u,
        None => return JNI_FALSE,
    };

    let gate_name: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return JNI_FALSE,
    };

    match statsig.check_gate(user.as_ref(), &gate_name) {
        true => JNI_TRUE,
        false => JNI_FALSE
    }
}
