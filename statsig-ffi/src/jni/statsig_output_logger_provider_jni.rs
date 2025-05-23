use jni::objects::{GlobalRef, JObject, JValue};
use jni::{JNIEnv, JavaVM};
use statsig_rust::{log_e, output_logger::OutputLogProvider};
use std::sync::Arc;

const TAG: &str = "StatsigOutputLoggerProviderJNI";

pub struct OutputLoggerProviderJNI {
    pub(crate) java_vm: JavaVM,
    pub(crate) java_ref: GlobalRef,
}

pub fn convert_to_output_logger_provider_rust(
    env: &JNIEnv,
    provider_java: JObject,
) -> Option<Arc<dyn OutputLogProvider>> {
    if provider_java.is_null() {
        return None;
    }

    let global_ref = match env.new_global_ref(provider_java).ok() {
        Some(g) => g,
        None => {
            log_e!(
                TAG,
                "Failed to create GlobalRef for Statsig OutputLoggerProvider"
            );
            return None;
        }
    };

    let java_vm = match env.get_java_vm().ok() {
        Some(vm) => vm,
        None => {
            log_e!(
                TAG,
                "Failed to get Java VM for Statsig OutputLoggerProvider"
            );
            return None;
        }
    };

    let provider_jni = OutputLoggerProviderJNI {
        java_vm,
        java_ref: global_ref,
    };

    let strong_provider: Arc<dyn OutputLogProvider> = Arc::new(provider_jni);
    Some(strong_provider)
}

impl OutputLogProvider for OutputLoggerProviderJNI {
    fn initialize(&self) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = env.call_method(self.java_ref.as_obj(), "init", "()V", &[]) {
            log_e!(TAG, "Failed to call init(): {:?}", e);
        }
    }

    fn debug(&self, tag: &str, msg: String) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = call_java_log_fn(&mut env, &self.java_ref, "debug", tag, &msg) {
            log_e!(TAG, "Failed to call debug(): {:?}", e);
        }
    }

    fn info(&self, tag: &str, msg: String) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = call_java_log_fn(&mut env, &self.java_ref, "info", tag, &msg) {
            log_e!(TAG, "Failed to call info(): {:?}", e);
        }
    }

    fn warn(&self, tag: &str, msg: String) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = call_java_log_fn(&mut env, &self.java_ref, "warn", tag, &msg) {
            log_e!(TAG, "Failed to call warn(): {:?}", e);
        }
    }

    fn error(&self, tag: &str, msg: String) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = call_java_log_fn(&mut env, &self.java_ref, "error", tag, &msg) {
            log_e!(TAG, "Failed to call error(): {:?}", e);
        }
    }

    fn shutdown(&self) {
        let Ok(mut env) = self.java_vm.attach_current_thread() else {
            log_e!(TAG, "Failed to attach current thread to JVM");
            return;
        };

        if let Err(e) = env.call_method(self.java_ref.as_obj(), "shutdown", "()V", &[]) {
            log_e!(TAG, "Failed to call shutdown(): {:?}", e);
        }
    }
}

fn call_java_log_fn(
    env: &mut JNIEnv,
    java_ref: &JObject,
    method_name: &str,
    tag: &str,
    msg: &str,
) -> jni::errors::Result<()> {
    let j_tag = env.new_string(tag)?;
    let j_msg = env.new_string(msg)?;

    env.call_method(
        java_ref,
        method_name,
        "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValue::Object(&j_tag), JValue::Object(&j_msg)],
    )?;

    Ok(())
}
