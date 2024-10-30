use crate::jni::jni_utils::jstring_to_string;
use crate::{get_instance_or_noop_jni, get_instance_or_return_jni};
use jni::sys::{jboolean, jclass, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;

use crate::jni::jni_utils::jni_to_rust_hashmap;
use jni::objects::{JClass, JObject, JString};
use sigstat::{instance_store::INST_STORE, log_e, Statsig, StatsigOptions, StatsigUser};

use super::jni_utils::serialize_json_to_jstring;

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCreate(
    mut env: JNIEnv,
    _class: JClass,
    sdk_key: JString,
    options_ref: JString,
) -> jstring {
    // StatsigOptions::new(); // temp: enable logging

    let sdk_key: String = match env.get_string(&sdk_key) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let options_inst_id = jstring_to_string(&mut env, options_ref);

    let options = INST_STORE.get_with_optional_id::<StatsigOptions>(options_inst_id.as_ref());

    let inst = Statsig::new(&sdk_key, options);

    let id = INST_STORE.add(inst);
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
        INST_STORE.remove(&id);
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
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigSequencedShutdownPrepare(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

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

    statsig.sequenced_shutdown_prepare(move || {
        let mut env = vm.attach_current_thread().unwrap();

        let result = env.call_method(global_callback.as_obj(), "run", "()V", &[]);
        if result.is_err() {
            log_e!("Failed to call callback");
        }

        drop(global_callback);
    });
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigFinalizeShutdown(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);
    statsig.finalize_shutdown();
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetClientInitResponse(
    mut env: JNIEnv,
    _class: JClass,
    statsig_ref: JString,
    user_ref: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let response = statsig.get_client_init_response(user.as_ref());

    serialize_json_to_jstring(&mut env, &response)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigCheckGate(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    gate_name: JString,
) -> jboolean {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, JNI_FALSE);
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, JNI_FALSE);

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
    statsig_ref: JString,
    user_ref: JString,
    exper_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let exper_name: String = match env.get_string(&exper_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_experiment(user.as_ref(), &exper_name);

    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetLayer(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    layer_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let layer_name: String = match env.get_string(&layer_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_layer(user.as_ref(), &layer_name);

    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetDynamicConfig(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    config_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let config_name: String = match env.get_string(&config_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_dynamic_config(user.as_ref(), &config_name);

    serialize_json_to_jstring(&mut env, &result)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigGetFeatureGate(
    mut env: JNIEnv,
    _class: jclass,
    statsig_ref: JString,
    user_ref: JString,
    gate_name: JString,
) -> jstring {
    let statsig = get_instance_or_return_jni!(Statsig, &mut env, statsig_ref, std::ptr::null_mut());
    let user = get_instance_or_return_jni!(StatsigUser, &mut env, user_ref, std::ptr::null_mut());

    let gate_name: String = match env.get_string(&gate_name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = statsig.get_feature_gate(user.as_ref(), &gate_name);
    serialize_json_to_jstring(&mut env, &result)
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
            log_e!("Failed to convert parameter_name to Rust string");
            return;
        }
    };

    statsig.log_layer_param_exposure(layer_json, parameter_name);
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
    statsig_ref: JString,
    callback: JObject,
) {
    let statsig = get_instance_or_noop_jni!(Statsig, &mut env, statsig_ref);

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

    statsig.flush_events_with_callback(move || {
        let mut env = vm.attach_current_thread().unwrap();

        let result = env.call_method(global_callback.as_obj(), "run", "()V", &[]);
        if result.is_err() {
            log_e!("Failed to call callback");
        }

        drop(global_callback);
    });
}
