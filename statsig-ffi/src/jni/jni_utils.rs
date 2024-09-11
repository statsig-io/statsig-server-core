use jni::objects::{JObject, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use sigstat::log_e;
use std::collections::HashMap;

#[macro_export]
macro_rules! get_instance_or_noop_jni {
    ($instances:ident, $env:expr, $ref:expr) => {
        match jstring_to_string($env, $ref) {
            Some(id) => match $instances.get(&id) {
                Some(instance) => instance,
                None => return,
            },
            None => return,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_return_jni {
    ($instances:ident, $env:expr, $ref:expr, $ret_value:expr) => {
        match jstring_to_string($env, $ref) {
            Some(id) => match $instances.get(&id) {
                Some(instance) => instance,
                None => return $ret_value,
            },
            None => return $ret_value,
        }
    };
}

#[macro_export]
macro_rules! get_instance_or_else_jni {
    ($instances:ident, $env:expr, $ref:expr, $else:expr) => {
        match jstring_to_string($env, $ref) {
            Some(id) => match $instances.get(&id) {
                Some(instance) => instance,
                None => $else,
            },
            None => $else,
        }
    };
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
            log_e!("Failed to serialize value to JSON string");
            None
        }
    };

    match result {
        Some(result) => string_to_jstring(env, result),
        None => std::ptr::null_mut(),
    }
}

pub fn jni_to_rust_hashmap(
    mut env: JNIEnv,
    jmap: JObject,
) -> Result<HashMap<String, String>, jni::errors::Error> {
    let mut rust_map = HashMap::new();

    let entry_set = match env.call_method(jmap, "entrySet", "()Ljava/util/Set;", &[]) {
        Ok(method) => method.l()?,
        Err(e) => {
            log_e!("Failed to get entrySet");
            return Err(e);
        }
    };

    let iterator = match env.call_method(entry_set, "iterator", "()Ljava/util/Iterator;", &[]) {
        Ok(method) => method.l()?,
        Err(e) => {
            log_e!("Failed to call iterator method: {:?}", e);
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
                log_e!("Failed to call next method: {:?}", e);
                return Err(e);
            }
        };

        let key = match env.call_method(&entry, "getKey", "()Ljava/lang/Object;", &[]) {
            Ok(method) => method.l()?,
            Err(e) => {
                log_e!("Failed to call getKey method: {:?}", e);
                return Err(e);
            }
        };

        let value = match env.call_method(&entry, "getValue", "()Ljava/lang/Object;", &[]) {
            Ok(method) => method.l()?,
            Err(e) => {
                log_e!("Failed to call getValue method: {:?}", e);
                return Err(e);
            }
        };

        let key_str = match env.get_string(&JString::from(key)) {
            Ok(s) => s.into(),
            Err(e) => {
                log_e!("Failed to convert key to string: {:?}", e);
                return Err(e.into());
            }
        };

        let value_str = match env.get_string(&JString::from(value)) {
            Ok(s) => s.into(),
            Err(e) => {
                log_e!("Failed to convert value to string: {:?}", e);
                return Err(e.into());
            }
        };
        rust_map.insert(key_str, value_str);
    }
    Ok(rust_map)
}
