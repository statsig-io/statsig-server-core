use async_trait::async_trait;
use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use statsig_rust::StatsigErr::DataStoreFailure;
use statsig_rust::{
    data_store_interface::{
        DataStoreResponse as DataStoreResponseActual, DataStoreTrait, RequestPath,
    },
    log_d, StatsigErr,
};
use std::sync::Arc;

const TAG: &str = "StatsigDataStoreJNI";

pub struct DataStoreJNI {
    pub java_vm: JavaVM,
    pub java_ref: GlobalRef,
}

pub fn convert_to_data_store_rust(env: &JNIEnv, obj: JObject) -> Option<Arc<dyn DataStoreTrait>> {
    if obj.is_null() {
        return None;
    }

    let java_vm = env.get_java_vm().ok()?;
    let global_ref = env.new_global_ref(obj).ok()?;

    log_d!(TAG, "Created GlobalRef to DataStore Java object");

    Some(Arc::new(DataStoreJNI {
        java_vm,
        java_ref: global_ref,
    }))
}

#[async_trait]
impl DataStoreTrait for DataStoreJNI {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        let mut env = self
            .java_vm
            .attach_current_thread()
            .map_err(|_| DataStoreFailure("Failed to attach thread in initialize".to_string()))?;

        let method_result = env
            .call_method(
                &self.java_ref,
                "initialize",
                "()Ljava/util/concurrent/CompletableFuture;",
                &[],
            )
            .map_err(|e| DataStoreFailure(format!("initialize() error: {e:?}")))?;

        let future_obj = method_result
            .l()
            .map_err(|e| DataStoreFailure(format!("Future cast failed: {e:?}")))?;

        let _ = env
            .call_method(&future_obj, "get", "()Ljava/lang/Object;", &[])
            .map_err(|e| DataStoreFailure(format!("Future.get() failed: {e:?}")))?
            .l()
            .map_err(|e| DataStoreFailure(format!("Future.get() unwrap failed: {e:?}")))?;

        Ok(())
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        let mut env = self.java_vm.attach_current_thread().map_err(|_| {
            DataStoreFailure("Failed to attach thread in shutdown signal".to_string())
        })?;

        let method_result = env
            .call_method(
                &self.java_ref,
                "shutdown",
                "()Ljava/util/concurrent/CompletableFuture;",
                &[],
            )
            .map_err(|e| DataStoreFailure(format!("shutdown() error: {e:?}")))?;

        let future_obj = method_result
            .l()
            .map_err(|e| DataStoreFailure(format!("Future cast failed: {e:?}")))?;

        let _ = env
            .call_method(&future_obj, "get", "()Ljava/lang/Object;", &[])
            .map_err(|e| DataStoreFailure(format!("Future.get() failed: {e:?}")))?
            .l()
            .map_err(|e| DataStoreFailure(format!("Future.get() unwrap failed: {e:?}")))?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponseActual, StatsigErr> {
        let mut env = self
            .java_vm
            .attach_current_thread()
            .map_err(|_| DataStoreFailure("Failed to attach thread in get signal".to_string()))?;

        let key_str = env
            .new_string(key)
            .map_err(|e| DataStoreFailure(format!("Failed to create key from string: {e:?}")))?;

        let method_result = env
            .call_method(
                &self.java_ref,
                "get",
                "(Ljava/lang/String;)Ljava/util/concurrent/CompletableFuture;",
                &[JValue::Object(&key_str)],
            )
            .map_err(|e| DataStoreFailure(format!("get() error: {e:?}")))?;

        let future_obj = method_result
            .l()
            .map_err(|e| DataStoreFailure(format!("Future cast failed: {e:?}")))?;

        let result_obj = env
            .call_method(&future_obj, "get", "()Ljava/lang/Object;", &[])
            .map_err(|e| DataStoreFailure(format!("Future.get() failed: {e:?}")))?
            .l()
            .map_err(|e| DataStoreFailure(format!("Future.get() unwrap failed: {e:?}")))?;

        let result_string = {
            let result_field = env.get_field(&result_obj, "result", "Ljava/lang/String;");
            match result_field {
                Ok(v) => {
                    let jstr: JString = v.l().map_err(|e| DataStoreFailure(e.to_string()))?.into();
                    let rust_str = env
                        .get_string(&jstr)
                        .map_err(|e| DataStoreFailure(e.to_string()))?
                        .into();
                    Some(rust_str)
                }
                _ => None,
            }
        };

        let time_val = match env.get_field(&result_obj, "time", "Ljava/lang/Long;") {
            Ok(v) => Some(v.j().unwrap_or(0) as u64),
            _ => None,
        };

        Ok(DataStoreResponseActual {
            result: result_string,
            time: time_val,
        })
    }

    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
        let mut env = self
            .java_vm
            .attach_current_thread()
            .map_err(|_| DataStoreFailure("Failed to attach thread in set".to_string()))?;

        let key_str = env
            .new_string(key)
            .map_err(|e| DataStoreFailure(format!("key conversion failed: {e:?}")))?;
        let value_str = env
            .new_string(value)
            .map_err(|e| DataStoreFailure(format!("value conversion failed: {e:?}")))?;

        let time_obj = match time {
            Some(t) => {
                let long_class = env
                    .find_class("java/lang/Long")
                    .map_err(|e| DataStoreFailure(format!("Failed to find Long class: {e:?}")))?;
                env.call_static_method(
                    long_class,
                    "valueOf",
                    "(J)Ljava/lang/Long;",
                    &[JValue::Long(t as i64)],
                )
                .map_err(|e| DataStoreFailure(format!("Long.valueOf failed: {e:?}")))?
                .l()
                .map_err(|e| DataStoreFailure(format!("valueOf unwrap failed: {e:?}")))?
            }
            None => JObject::null(),
        };

        let method_result = env.call_method(
            &self.java_ref,
            "set",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/Long;)Ljava/util/concurrent/CompletableFuture;",
            &[JValue::Object(&key_str), JValue::Object(&value_str), JValue::Object(&time_obj)],
        ).map_err(|e| DataStoreFailure(format!("set() error: {e:?}")))?;

        let future_obj = method_result
            .l()
            .map_err(|e| DataStoreFailure(format!("Future cast failed: {e:?}")))?;

        let _ = env
            .call_method(&future_obj, "get", "()Ljava/lang/Object;", &[])
            .map_err(|e| DataStoreFailure(format!("Future.get() failed: {e:?}")))?
            .l()
            .map_err(|e| DataStoreFailure(format!("Future.get() unwrap failed: {e:?}")))?;

        Ok(())
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        let mut env = match self.java_vm.attach_current_thread() {
            Ok(env) => env,
            Err(_) => return false,
        };

        let path_str = match env.new_string(path.to_string()) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let method_result = env.call_method(
            &self.java_ref,
            "supportPollingUpdatesFor",
            "(Ljava/lang/String;)Ljava/util/concurrent/CompletableFuture;",
            &[JValue::Object(&path_str)],
        );

        let future_obj = match method_result.and_then(|v| v.l()) {
            Ok(f) => f,
            Err(_) => return false,
        };

        let result_obj = match env
            .call_method(&future_obj, "get", "()Ljava/lang/Object;", &[])
            .and_then(|v| v.l())
        {
            Ok(r) => r,
            Err(_) => return false,
        };

        match env.call_method(&result_obj, "booleanValue", "()Z", &[]) {
            Ok(val) => val.z().unwrap_or(false),
            Err(_) => false,
        }
    }
}
