use crate::jni_utils::put_all_into_java_map;
use jni::objects::{GlobalRef, JObject, JValue};
use jni::{JNIEnv, JavaVM};
use statsig_rust::{log_e, ok_or_return_with, ObservabilityClient, OpsStatsEventObserver};
use std::collections::HashMap;
use std::sync::{Arc, Weak};

const TAG: &str = "StatsigObservabilityClientJNI";

type ObservabilityClientRefs = (
    Option<Arc<dyn ObservabilityClient>>,
    Option<Weak<dyn ObservabilityClient>>,
);

pub struct ObservabilityClientJNI {
    pub(crate) java_vm: JavaVM,
    pub(crate) java_ref: GlobalRef,
}

pub fn convert_to_ob_rust(env: &JNIEnv, ob_java: JObject) -> ObservabilityClientRefs {
    if ob_java.is_null() {
        return (None, None);
    }

    let global_ref = match env.new_global_ref(ob_java).ok() {
        Some(g) => g,
        None => {
            log_e!(
                TAG,
                "Failed to create GlobalRef for Statsig ObservabilityClient"
            );
            return (None, None);
        }
    };

    let java_vm = match env.get_java_vm().ok() {
        Some(vm) => vm,
        None => {
            log_e!(TAG, "Failed to get Java VM for Statsig ObservabilityClient");
            return (None, None);
        }
    };

    let ob_jni = ObservabilityClientJNI {
        java_vm,
        java_ref: global_ref,
    };

    let strong_ob: Arc<dyn ObservabilityClient> = Arc::new(ob_jni);
    let weak_ob = Arc::downgrade(&strong_ob);

    (Some(strong_ob), Some(weak_ob))
}

impl ObservabilityClient for ObservabilityClientJNI {
    fn init(&self) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = env.call_method(self.java_ref.as_obj(), "init", "()V", &[]) {
            log_e!(TAG, "Failed to call init(): {:?}", e);
        }
    }

    fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = call_java_metric_fn(
            &mut env,
            &self.java_ref,
            "increment",
            &metric_name,
            value,
            &tags,
        ) {
            log_e!(TAG, "Failed to call increment(): {:?}", e);
        }
    }

    fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = call_java_metric_fn(
            &mut env,
            &self.java_ref,
            "gauge",
            &metric_name,
            value,
            &tags,
        ) {
            log_e!(TAG, "Failed to call gauge(): {:?}", e);
        }
    }

    fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) =
            call_java_metric_fn(&mut env, &self.java_ref, "dist", &metric_name, value, &tags)
        {
            log_e!(TAG, "Failed to call dist(): {:?}", e);
        }
    }

    fn should_enable_high_cardinality_for_this_tag(&self, tag: String) -> Option<bool> {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return None;
        };
        let tag_j_string = ok_or_return_with!(env.new_string(tag), |_| None);
        let result = env.call_method(
            &self.java_ref,
            "shouldEnableHighCardinalityForThisTag",
            "(Ljava/lang/String;DLjava/util/Map;)V",
            &[JValue::Object(&tag_j_string)],
        );

        let bool_val = ok_or_return_with!(result.and_then(|r| r.z()), |_| None);
        Some(bool_val)
    }

    fn error(&self, tag: String, error: String) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        let Ok(j_tag) = env.new_string(tag) else {
            log_e!(TAG, "Failed to convert tag to Java string");
            return;
        };

        let Ok(j_error) = env.new_string(error) else {
            log_e!(TAG, "Failed to convert error message to Java string");
            return;
        };

        if let Err(e) = env.call_method(
            self.java_ref.as_obj(),
            "error",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&j_tag), JValue::Object(&j_error)],
        ) {
            log_e!(TAG, "Failed to call error() method on Java client: {:?}", e);
        }
    }

    fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
        self
    }
}

fn call_java_metric_fn(
    env: &mut JNIEnv,
    java_ref: &JObject,
    method_name: &str,
    metric_name: &str,
    value: f64,
    tags: &Option<HashMap<String, String>>,
) -> jni::errors::Result<()> {
    let j_metric_name = env.new_string(metric_name)?;

    let j_tags = match tags {
        Some(rust_map) => {
            let jmap = env.new_object("java/util/HashMap", "()V", &[])?;
            put_all_into_java_map(env, &jmap, rust_map)?;
            jmap
        }
        None => JObject::null(),
    };

    env.call_method(
        java_ref,
        method_name,
        "(Ljava/lang/String;DLjava/util/Map;)V",
        &[
            JValue::Object(&j_metric_name),
            JValue::Double(value),
            JValue::Object(&j_tags),
        ],
    )?;

    Ok(())
}
