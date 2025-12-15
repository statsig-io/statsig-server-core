use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use statsig_rust::{log_e, PersistentStorage, StickyValues, UserPersistedValues};
use std::sync::Arc;

const TAG: &str = "StatsigPersistentStorageJNI";

pub struct PersistentStorageJNI {
    pub(crate) java_vm: JavaVM,
    pub(crate) java_ref: GlobalRef,
}

pub fn convert_to_persistent_storage_rust(
    env: &JNIEnv,
    ps_java: JObject,
) -> Option<Arc<dyn PersistentStorage>> {
    if ps_java.is_null() {
        return None;
    }

    let global_ref = match env.new_global_ref(ps_java).ok() {
        Some(g) => g,
        None => {
            log_e!(
                TAG,
                "Failed to create GlobalRef for Statsig PersistentStorage"
            );
            return None;
        }
    };

    let java_vm = match env.get_java_vm().ok() {
        Some(vm) => vm,
        None => {
            log_e!(TAG, "Failed to get Java VM for Statsig PersistentStorage");
            return None;
        }
    };

    let ps_jni = PersistentStorageJNI {
        java_vm,
        java_ref: global_ref,
    };

    Some(Arc::new(ps_jni))
}

impl PersistentStorage for PersistentStorageJNI {
    fn load(&self, key: String) -> Option<UserPersistedValues> {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return None;
        };

        let j_key = match env.new_string(key) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to create Java string for key: {:?}", e);
                return None;
            }
        };

        // Call Java load() method which returns Map<String, StickyValues>
        let result = match env.call_method(
            &self.java_ref,
            "load",
            "(Ljava/lang/String;)Ljava/util/Map;",
            &[JValue::Object(&j_key)],
        ) {
            Ok(r) => r,
            Err(e) => {
                log_e!(TAG, "Failed to call load(): {:?}", e);
                return None;
            }
        };

        let result_obj = match result.l() {
            Ok(obj) => obj,
            Err(e) => {
                log_e!(TAG, "Failed to get result object: {:?}", e);
                return None;
            }
        };

        if result_obj.is_null() {
            return None;
        }

        let json_class = match env.find_class("com/alibaba/fastjson2/JSON") {
            Ok(cls) => cls,
            Err(e) => {
                log_e!(TAG, "Failed to find JSON class: {:?}", e);
                return None;
            }
        };

        let json_str_result = env.call_static_method(
            json_class,
            "toJSONString",
            "(Ljava/lang/Object;)Ljava/lang/String;",
            &[JValue::Object(&result_obj)],
        );

        let json_str_obj = match json_str_result {
            Ok(r) => match r.l() {
                Ok(obj) => obj,
                Err(e) => {
                    log_e!(TAG, "Failed to get JSON string object: {:?}", e);
                    return None;
                }
            },
            Err(e) => {
                log_e!(TAG, "Failed to call JSON.toJSONString(): {:?}", e);
                return None;
            }
        };

        let json_str: JString = json_str_obj.into();
        let json_str: String = match env.get_string(&json_str) {
            Ok(java_str) => java_str.into(),
            Err(e) => {
                log_e!(TAG, "Failed to get string from Java String: {:?}", e);
                return None;
            }
        };

        // Deserialize JSON string to UserPersistedValues
        match serde_json::from_str::<UserPersistedValues>(&json_str) {
            Ok(values) => Some(values),
            Err(e) => {
                log_e!(TAG, "Failed to deserialize JSON from load result: {:?}", e);
                None
            }
        }
    }

    fn save(&self, key: &str, config_name: &str, data: StickyValues) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        let j_key = match env.new_string(key) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to create Java string for key: {:?}", e);
                return;
            }
        };

        let j_config_name = match env.new_string(config_name) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to create Java string for config_name: {:?}", e);
                return;
            }
        };

        // Serialize StickyValues to JSON string first
        let data_json = match serde_json::to_string(&data) {
            Ok(json) => json,
            Err(e) => {
                log_e!(TAG, "Failed to serialize StickyValues to JSON: {:?}", e);
                return;
            }
        };

        // Parse JSON string to StickyValues Java object using fastjson2
        let json_class = match env.find_class("com/alibaba/fastjson2/JSON") {
            Ok(cls) => cls,
            Err(e) => {
                log_e!(TAG, "Failed to find JSON class: {:?}", e);
                return;
            }
        };

        let j_data_json = match env.new_string(data_json) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to create Java string for data JSON: {:?}", e);
                return;
            }
        };

        let sticky_values_class = match env.find_class("com/statsig/StickyValues") {
            Ok(cls) => cls,
            Err(e) => {
                log_e!(TAG, "Failed to find StickyValues class: {:?}", e);
                return;
            }
        };

        let j_data = match env.call_static_method(
            json_class,
            "parseObject",
            "(Ljava/lang/String;Ljava/lang/Class;)Ljava/lang/Object;",
            &[
                JValue::Object(&j_data_json),
                JValue::Object(&sticky_values_class),
            ],
        ) {
            Ok(r) => match r.l() {
                Ok(obj) => obj,
                Err(e) => {
                    log_e!(TAG, "Failed to parse StickyValues object: {:?}", e);
                    return;
                }
            },
            Err(e) => {
                log_e!(TAG, "Failed to call JSON.parseObject(): {:?}", e);
                return;
            }
        };

        if let Err(e) = env.call_method(
            &self.java_ref,
            "save",
            "(Ljava/lang/String;Ljava/lang/String;Lcom/statsig/StickyValues;)V",
            &[
                JValue::Object(&j_key),
                JValue::Object(&j_config_name),
                JValue::Object(&j_data),
            ],
        ) {
            log_e!(TAG, "Failed to call save(): {:?}", e);
        }
    }

    fn delete(&self, key: &str, config_name: &str) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        let j_key = match env.new_string(key) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to create Java string for key: {:?}", e);
                return;
            }
        };

        let j_config_name = match env.new_string(config_name) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to create Java string for config_name: {:?}", e);
                return;
            }
        };

        if let Err(e) = env.call_method(
            &self.java_ref,
            "delete",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&j_key), JValue::Object(&j_config_name)],
        ) {
            log_e!(TAG, "Failed to call delete(): {:?}", e);
        }
    }
}
