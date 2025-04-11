use jni::objects::{JObject, JString, JValue};
use jni::sys::{jboolean, jstring};
use jni::JNIEnv;
use serde_json::Value;
use statsig_rust::{
    log_e, ClientInitResponseOptions, DynamicConfigEvaluationOptions, ExperimentEvaluationOptions,
    FeatureGateEvaluationOptions, HashAlgorithm, LayerEvaluationOptions,
};
use std::collections::HashMap;

const TAG: &str = "JniUtils";

#[macro_export]
macro_rules! get_instance_or_noop_jni {
    ($type:ty, $env:expr, $ref:expr) => {
        match jstring_to_string($env, $ref) {
            Some(id) => statsig_rust::get_instance_or_noop!($type, &id),
            None => return,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_return_jni {
    ($type:ty, $env:expr, $ref:expr, $ret_value:expr) => {
        match jstring_to_string($env, $ref) {
            Some(id) => statsig_rust::get_instance_or_return!($type, &id, $ret_value),
            None => return $ret_value,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_else_jni {
    ($type:ty, $env:expr, $ref:expr, $else:expr) => {
        match jstring_to_string($env, $ref) {
            Some(id) => match InstanceRegistry::get::<$type>(&id) {
                Some(instance) => instance,
                None => $else,
            },
            None => $else,
        }
    };
}

pub fn convert_java_client_init_response_options_to_rust(
    env: &mut JNIEnv,
    java_gcir_option: JObject,
) -> Option<ClientInitResponseOptions> {
    if java_gcir_option.is_null() {
        return None;
    }

    let hash_algo_field: JString =
        match env.get_field(&java_gcir_option, "hashAlgoInternal", "Ljava/lang/String;") {
            Ok(field) => field.l().unwrap().into(),
            Err(_) => return None,
        };

    let client_sdk_key_field: JString =
        match env.get_field(&java_gcir_option, "clientSDKKey", "Ljava/lang/String;") {
            Ok(field) => field.l().unwrap().into(),
            Err(_) => return None,
        };

    let hash_algo = jstring_to_string(env, hash_algo_field);
    let client_sdk_key = jstring_to_string(env, client_sdk_key_field);

    let hash_algo = hash_algo.and_then(|s| HashAlgorithm::from_string(s.as_str()));
    Some(ClientInitResponseOptions {
        hash_algorithm: hash_algo,
        client_sdk_key,
        ..ClientInitResponseOptions::default()
    })
}

pub fn convert_java_check_gate_options_to_rust(
    env: &mut JNIEnv,
    options: JObject,
) -> Option<FeatureGateEvaluationOptions> {
    if options.is_null() {
        return None;
    }

    let disable_exposure_logging_field: jboolean =
        match env.get_field(&options, "disableExposureLogging", "Z") {
            Ok(field) => field.z().unwrap().into(),
            Err(_) => return None,
        };

    let disable_exposure_logging = jboolean_to_bool(disable_exposure_logging_field);

    disable_exposure_logging.map(|disable_exposure_logging| FeatureGateEvaluationOptions {
        disable_exposure_logging,
    })
}

pub fn convert_java_get_dynamic_config_options_to_rust(
    env: &mut JNIEnv,
    options: JObject,
) -> Option<DynamicConfigEvaluationOptions> {
    if options.is_null() {
        return None;
    }

    let disable_exposure_logging_field: jboolean =
        match env.get_field(&options, "disableExposureLogging", "Z") {
            Ok(field) => field.z().unwrap().into(),
            Err(_) => return None,
        };

    let disable_exposure_logging = jboolean_to_bool(disable_exposure_logging_field);

    disable_exposure_logging.map(|disable_exposure_logging| DynamicConfigEvaluationOptions {
        disable_exposure_logging,
    })
}

pub fn convert_java_get_experiment_options_to_rust(
    env: &mut JNIEnv,
    options: JObject,
) -> Option<ExperimentEvaluationOptions> {
    if options.is_null() {
        return None;
    }

    let disable_exposure_logging_field: jboolean =
        match env.get_field(&options, "disableExposureLogging", "Z") {
            Ok(field) => field.z().unwrap().into(),
            Err(_) => return None,
        };

    let disable_exposure_logging = jboolean_to_bool(disable_exposure_logging_field);

    disable_exposure_logging.map(|disable_exposure_logging| ExperimentEvaluationOptions {
        disable_exposure_logging,
        user_persisted_values: None,
    })
}

pub fn convert_java_get_layer_options_to_rust(
    env: &mut JNIEnv,
    options: JObject,
) -> Option<LayerEvaluationOptions> {
    if options.is_null() {
        return None;
    }

    let disable_exposure_logging_field: jboolean =
        match env.get_field(&options, "disableExposureLogging", "Z") {
            Ok(field) => field.z().unwrap().into(),
            Err(_) => return None,
        };

    let disable_exposure_logging = jboolean_to_bool(disable_exposure_logging_field);

    disable_exposure_logging.map(|disable_exposure_logging| LayerEvaluationOptions {
        disable_exposure_logging,
        user_persisted_values: None,
    })
}

pub fn jstring_to_string(env: &mut JNIEnv, input: JString) -> Option<String> {
    match env.get_string(&input) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    }
}

pub fn string_to_jstring(env: &mut JNIEnv, input: String) -> jstring {
    match env.new_string(input) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

pub fn serialize_json_to_jstring<T: serde::Serialize>(env: &mut JNIEnv, value: &T) -> jstring {
    let result = match serde_json::to_string(value) {
        Ok(json_str) => Some(json_str),
        Err(_) => {
            log_e!(TAG, "Failed to serialize value to JSON string");
            None
        }
    };

    match result {
        Some(result) => string_to_jstring(env, result),
        None => std::ptr::null_mut(),
    }
}

pub fn jboolean_to_bool(input: jboolean) -> Option<bool> {
    match input {
        1 => Some(true),
        0 => Some(false),
        _ => None,
    }
}

// Assumes input is valid (non-zero = true), for typical JNI use
pub fn jboolean_to_bool_unchecked(input: jboolean) -> bool {
    input != 0
}

/// Converts a Java `java.util.HashMap<String, String>` into Rust `HashMap<String, Value>`
pub fn jni_to_rust_json_map(
    env: &mut JNIEnv,
    jmap: JObject,
) -> Result<HashMap<String, Value>, jni::errors::Error> {
    let mut rust_map = HashMap::new();

    let entry_set = env
        .call_method(jmap, "entrySet", "()Ljava/util/Set;", &[])?
        .l()?;
    let iterator = env
        .call_method(entry_set, "iterator", "()Ljava/util/Iterator;", &[])?
        .l()?;

    while env.call_method(&iterator, "hasNext", "()Z", &[])?.z()? {
        let entry = env
            .call_method(&iterator, "next", "()Ljava/lang/Object;", &[])?
            .l()?;
        let key_obj = env
            .call_method(&entry, "getKey", "()Ljava/lang/Object;", &[])?
            .l()?;
        let value_obj = env
            .call_method(&entry, "getValue", "()Ljava/lang/Object;", &[])?
            .l()?;

        let key = env.get_string(&JString::from(key_obj))?.into();

        let value = java_object_to_json_value(env, value_obj)?;
        rust_map.insert(key, value);
    }

    Ok(rust_map)
}

fn java_object_to_json_value(env: &mut JNIEnv, obj: JObject) -> Result<Value, jni::errors::Error> {
    if obj.is_null() {
        return Ok(Value::Null);
    }

    let class = env.get_object_class(&obj)?;
    let class_name = env
        .call_method(class, "getName", "()Ljava/lang/String;", &[])?
        .l()?;
    let class_str: String = env.get_string(&JString::from(class_name))?.into();

    match class_str.as_str() {
        "java.lang.String" => {
            let str_val = env.get_string(&JString::from(obj))?.into();
            Ok(Value::String(str_val))
        }
        "java.lang.Boolean" => {
            let bool_val = env.call_method(obj, "booleanValue", "()Z", &[])?.z()?;
            Ok(Value::Bool(bool_val))
        }
        "java.lang.Integer" | "java.lang.Long" => {
            let long_val = env.call_method(obj, "longValue", "()J", &[])?.j()?;
            Ok(Value::Number(long_val.into()))
        }
        "java.lang.Double" | "java.lang.Float" => {
            let double_val = env.call_method(obj, "doubleValue", "()D", &[])?.d()?;
            Ok(serde_json::Number::from_f64(double_val)
                .map(Value::Number)
                .unwrap_or(Value::Null))
        }
        "java.util.Map" => {
            let inner_map = jni_to_rust_json_map(env, obj)?;
            let mut json_map = serde_json::Map::new();
            json_map.extend(inner_map);
            Ok(Value::Object(json_map))
        }
        "java.util.List" => {
            let list_size = env.call_method(&obj, "size", "()I", &[])?.i()?;
            let mut list = Vec::with_capacity(list_size as usize);
            for i in 0..list_size {
                let item = env
                    .call_method(&obj, "get", "(I)Ljava/lang/Object;", &[JValue::from(i)])?
                    .l()?;
                list.push(java_object_to_json_value(env, item)?);
            }
            Ok(Value::Array(list))
        }
        _ => {
            log_e!(TAG, "Unhandled Java object type: {}", class_str);
            Ok(Value::Null)
        }
    }
}

/// Converts a Java `java.util.HashMap<String, String>` into Rust `HashMap<String, String>`
pub fn jni_to_rust_hashmap(
    mut env: JNIEnv,
    jmap: JObject,
) -> Result<HashMap<String, String>, jni::errors::Error> {
    let mut rust_map = HashMap::new();

    let entry_set = match env.call_method(jmap, "entrySet", "()Ljava/util/Set;", &[]) {
        Ok(method) => method.l()?,
        Err(e) => {
            log_e!(TAG, "Failed to get entrySet");
            return Err(e);
        }
    };

    let iterator = match env.call_method(entry_set, "iterator", "()Ljava/util/Iterator;", &[]) {
        Ok(method) => method.l()?,
        Err(e) => {
            log_e!(TAG, "Failed to call iterator method: {:?}", e);
            return Err(e);
        }
    };

    while let Ok(has_next) = env.call_method(&iterator, "hasNext", "()Z", &[])?.z() {
        if !has_next {
            break;
        }
        let entry = match env.call_method(&iterator, "next", "()Ljava/lang/Object;", &[]) {
            Ok(method) => method.l()?,
            Err(e) => {
                log_e!(TAG, "Failed to call next method: {:?}", e);
                return Err(e);
            }
        };

        let key = match env.call_method(&entry, "getKey", "()Ljava/lang/Object;", &[]) {
            Ok(method) => method.l()?,
            Err(e) => {
                log_e!(TAG, "Failed to call getKey method: {:?}", e);
                return Err(e);
            }
        };

        let value = match env.call_method(&entry, "getValue", "()Ljava/lang/Object;", &[]) {
            Ok(method) => method.l()?,
            Err(e) => {
                log_e!(TAG, "Failed to call getValue method: {:?}", e);
                return Err(e);
            }
        };

        let key_str = match env.get_string(&JString::from(key)) {
            Ok(s) => s.into(),
            Err(e) => {
                log_e!(TAG, "Failed to convert key to string: {:?}", e);
                return Err(e);
            }
        };

        let value_str = match env.get_string(&JString::from(value)) {
            Ok(s) => s.into(),
            Err(e) => {
                log_e!(TAG, "Failed to convert value to string: {:?}", e);
                return Err(e);
            }
        };
        rust_map.insert(key_str, value_str);
    }
    Ok(rust_map)
}

/// Put Content of Rust `HashMap<String, String>` into a Java `java.util.HashMap<String, String>`
/// This is better than returning a `JObject` because it avoids unnecessary object creation and reference management in the JNI layer, reducing overhead and potential memory leaks.
pub fn put_all_into_java_map(
    env: &mut JNIEnv,
    jmap: &JObject,
    map: &HashMap<String, String>,
) -> jni::errors::Result<()> {
    for (key, value) in map.iter() {
        let j_key = env.new_string(key)?;
        let j_value = env.new_string(value)?;

        env.call_method(
            jmap,
            "put",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            &[
                JValue::Object(&JObject::from(j_key)),
                JValue::Object(&JObject::from(j_value)),
            ],
        )?;
    }
    Ok(())
}
