use jni::objects::{JObject, JString, JValue};
use jni::sys::{jboolean, jlong, jstring};
use jni::JNIEnv;
use serde_json::{Number, Value};
use statsig_rust::networking::proxy_config::ProxyConfig;
use statsig_rust::{
    log_e, log_w, ClientInitResponseOptions, DynamicConfigEvaluationOptions,
    ExperimentEvaluationOptions, FeatureGateEvaluationOptions, GCIRResponseFormat, HashAlgorithm,
    LayerEvaluationOptions,
};
use std::collections::HashMap;

const TAG: &str = "JniUtils";

pub fn convert_java_proxy_config_to_rust(
    env: &mut JNIEnv,
    java_proxy_config: &JObject,
) -> Option<ProxyConfig> {
    if java_proxy_config.is_null() {
        return None;
    }

    let proxy_host: JString =
        match env.get_field(java_proxy_config, "proxyHost", "Ljava/lang/String;") {
            Ok(field) => field.l().unwrap().into(),
            Err(_) => return None,
        };

    let proxy_port = get_proxy_port(env, java_proxy_config);

    let proxy_auth: JString =
        match env.get_field(java_proxy_config, "proxyAuth", "Ljava/lang/String;") {
            Ok(field) => field.l().unwrap().into(),
            Err(_) => return None,
        };

    let proxy_protocol: JString =
        match env.get_field(java_proxy_config, "proxyProtocol", "Ljava/lang/String;") {
            Ok(field) => field.l().unwrap().into(),
            Err(_) => return None,
        };

    Some(ProxyConfig {
        proxy_host: jstring_to_string(env, proxy_host),
        proxy_port,
        proxy_auth: jstring_to_string(env, proxy_auth),
        proxy_protocol: jstring_to_string(env, proxy_protocol),
    })
}

fn get_proxy_port(env: &mut JNIEnv, obj: &JObject) -> Option<u16> {
    env.get_field(obj, "proxyPort", "I")
        .ok()
        .and_then(|val| val.i().ok())
        .and_then(|port| if port > 0 { Some(port as u16) } else { None })
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

    let include_local_overrides_field: jboolean =
        match env.get_field(&java_gcir_option, "includeLocalOverrides", "Z") {
            Ok(field) => field.z().unwrap().into(),
            Err(_) => return None,
        };

    let remove_id_type_field: jboolean = match env.get_field(&java_gcir_option, "removeIDType", "Z")
    {
        Ok(field) => field.z().unwrap().into(),
        Err(_) => return None,
    };

    let response_format_field: JString = match env.get_field(
        &java_gcir_option,
        "responseFormatInternal",
        "Ljava/lang/String;",
    ) {
        Ok(field) => field.l().unwrap().into(),
        Err(_) => return None,
    };

    let hash_algo = jstring_to_string(env, hash_algo_field);
    let client_sdk_key = jstring_to_string(env, client_sdk_key_field);
    let include_local_overrides = jboolean_to_bool(include_local_overrides_field);
    let response_format = jstring_to_string(env, response_format_field);
    let remove_id_type = jboolean_to_bool(remove_id_type_field);

    let hash_algo = hash_algo.and_then(|s| HashAlgorithm::from_string(s.as_str()));
    let response_format = response_format.and_then(|s| GCIRResponseFormat::from_string(s.as_str()));
    Some(ClientInitResponseOptions {
        hash_algorithm: hash_algo,
        client_sdk_key,
        include_local_overrides,
        feature_gate_filter: None, // TODO(weihao)
        experiment_filter: None,   // TODO(weihao)
        dynamic_config_filter: None,
        layer_filter: None,
        param_store_filter: None,
        response_format,
        remove_id_type,
        remove_default_value_gates: None,
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

/// Converts a Java `java.util.HashMap<String, Object>` into Rust `HashMap<String, Value>`
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

pub fn get_string_field(env: &mut JNIEnv, obj: &JObject, field: &str) -> Option<String> {
    let raw_field = match env.get_field(obj, field, "Ljava/lang/String;") {
        Ok(value) => value,
        Err(e) => {
            log_w!(
                TAG,
                "Failed to access string field {} from Java object: {:?}",
                field,
                e
            );
            return None;
        }
    };

    let field_obj = match raw_field.l() {
        Ok(obj) => obj,
        Err(e) => {
            log_w!(
                TAG,
                "Failed to read string field {} from Java object: {:?}",
                field,
                e
            );
            return None;
        }
    };

    if field_obj.is_null() {
        return None;
    }

    let jstring: JString = field_obj.into();
    jstring_to_string(env, jstring)
}

pub fn get_long_field(env: &mut JNIEnv, obj: &JObject, field: &str) -> Option<jlong> {
    let raw_field = match env.get_field(obj, field, "J") {
        Ok(value) => value,
        Err(e) => {
            log_w!(
                TAG,
                "Failed to access long field {} from Java object: {:?}",
                field,
                e
            );
            return None;
        }
    };

    match raw_field.j() {
        Ok(val) => Some(val),
        Err(e) => {
            log_w!(
                TAG,
                "Failed to read long field {} from Java object: {:?}",
                field,
                e
            );
            None
        }
    }
}

fn java_object_to_json_value(env: &mut JNIEnv, obj: JObject) -> Result<Value, jni::errors::Error> {
    // 1) null -> JSON null
    if obj.is_null() {
        return Ok(Value::Null);
    }
    // 2) String
    if env.is_instance_of(&obj, "java/lang/String")? {
        let jstr = JString::from(obj);
        let rust_string: String = env.get_string(&jstr)?.into();
        return Ok(Value::String(rust_string));
    }

    // 3) Boolean
    if env.is_instance_of(&obj, "java/lang/Boolean")? {
        let b = env.call_method(obj, "booleanValue", "()Z", &[])?.z()?;
        return Ok(Value::Bool(b));
    }

    // 4) Integer/Long
    if env.is_instance_of(&obj, "java/lang/Integer")?
        || env.is_instance_of(&obj, "java/lang/Long")?
    {
        let n = env.call_method(obj, "longValue", "()J", &[])?.j()?;
        return Ok(Value::Number(Number::from(n)));
    }

    // 5) Double/Float
    if env.is_instance_of(&obj, "java/lang/Double")?
        || env.is_instance_of(&obj, "java/lang/Float")?
    {
        let d = env.call_method(obj, "doubleValue", "()D", &[])?.d()?;
        return Ok(Number::from_f64(d)
            .map(Value::Number)
            .unwrap_or(Value::Null));
    }

    // 6) Map
    if env.is_instance_of(&obj, "java/util/Map")? {
        let hm: HashMap<String, Value> = jni_to_rust_json_map(env, obj)?;
        let json_map: serde_json::Map<String, Value> = hm.into_iter().collect();
        return Ok(Value::Object(json_map));
    }

    // 7) List
    if env.is_instance_of(&obj, "java/util/List")? {
        let size = env.call_method(&obj, "size", "()I", &[])?.i()?;
        let mut vec = Vec::with_capacity(size as usize);
        for i in 0..size {
            let element = env
                .call_method(&obj, "get", "(I)Ljava/lang/Object;", &[JValue::from(i)])?
                .l()?;
            vec.push(java_object_to_json_value(env, element)?);
        }
        return Ok(Value::Array(vec));
    }

    // 8) others
    let cls = env.get_object_class(obj)?;
    let name_obj = env
        .call_method(cls, "getName", "()Ljava/lang/String;", &[])?
        .l()?;
    let name: String = env.get_string(&JString::from(name_obj))?.into();
    log_e!(TAG, "Unhandled Java object type: {}", name);
    Ok(Value::Null)
}

/// Converts a Java `java.util.HashMap<String, String>` into Rust `HashMap<String, String>`
pub fn jni_to_rust_hashmap(
    mut env: JNIEnv,
    jmap: JObject,
) -> Result<HashMap<String, String>, jni::errors::Error> {
    let mut rust_map = HashMap::new();

    if jmap.is_null() {
        return Ok(rust_map);
    }

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
