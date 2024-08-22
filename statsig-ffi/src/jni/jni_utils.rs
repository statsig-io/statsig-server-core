use std::collections::HashMap;
use jni::errors::Error;
use jni::JNIEnv;
use jni::objects::{JMap, JObject, JString};
use jni::sys::jobject;
use statsig::log_e;

pub fn jstring_to_string(env: &mut JNIEnv, input: JString) -> Option<String> {
    match env.get_string(&input) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    }
}

pub fn serialize_to_json_string<T: serde::Serialize>(value: &T) -> Option<String> {
    match serde_json::to_string(value) {
        Ok(json_str) => Some(json_str),
        Err(_) => {
            log_e!("Failed to serialize value to JSON string");
            None
        }
    }
}

pub fn jni_to_rust_hashmap(
    mut env: JNIEnv,
    jmap: JObject
) -> Result<HashMap<String, String>, jni::errors::Error> {
    let mut rust_map = HashMap::new();

    let entry_set = match env.call_method(jmap, "entrySet", "()Ljava/util/Set;", &[]) {
        Ok(method) => {
            method.l()?
        },
        Err(e) => {
            log_e!("Failed to get entrySet");
            return Err(e);
        }
    };

    let iterator = match env.call_method(entry_set, "iterator", "()Ljava/util/Iterator;", &[]) {
        Ok(method) => {
            method.l()?
        },
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

