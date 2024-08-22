use std::collections::HashMap;
use jni::JNIEnv;
use jni::objects::{JMap, JObject, JString};
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
    jmap: JMap
) -> Result<HashMap<String, String>, jni::errors::Error> {
    let mut rust_map = HashMap::new();

    let entry_set = env.call_method(jmap, "entrySet", "()Ljava/util/Set;", &[])?.l()?;

    let iterator = env.call_method(entry_set, "iterator", "()Ljava/util/Iterator;", &[])?.l()?;

    while let Ok(has_next) = env.call_method(&iterator, "hasNext", "()Z", &[])?.z() {
        if !has_next {
            break;
        }
        let entry = env.call_method(&iterator, "next", "()Ljava/lang/Object;", &[])?.l()?;

        let key = env.call_method(&entry, "getKey", "()Ljava/lang/Object;", &[])?.l()?;
        let value: JObject = env.call_method(&entry, "getValue", "()Ljava/lang/Object;", &[])?.l()?;

        let key_str: String = env.get_string(&JString::from(key))?.into();
        let value_str: String = env.get_string(&JString::from(value))?.into();

        rust_map.insert(key_str, value_str);
    }

    Ok(rust_map)
}

