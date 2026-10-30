//! JNI bridge for the experimental forward-proxy user enrichment API
//! (`statsig_rust::experimental`). Backs `com.statsig.experimental.StatsigFPUserBuilder`
//! and `com.statsig.experimental.ForwardProxyUserEnrichmentProvider`.

use crate::jni::jni_utils::{
    convert_java_proxy_config_to_rust, jstring_to_string, string_to_jstring,
};
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use statsig_rust::experimental::{
    ForwardProxyEnrichmentOptions, ForwardProxyUserEnrichmentProvider, StatsigFPUserBuilder,
};
use statsig_rust::{log_e, InstanceRegistry, StatsigRuntime, StatsigUserData};

const TAG: &str = "UserEnrichmentJNI";

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigFpEnrichmentProviderCreate(
    mut env: JNIEnv,
    _class: JClass,
    sdk_key: JString,
    url: JString,
    timeout_ms: jlong,
    retries: jlong,
    proxy_config: JObject,
) -> jlong {
    let sdk_key = match jstring_to_string(&mut env, sdk_key) {
        Some(value) => value,
        None => return 0,
    };
    let url = match jstring_to_string(&mut env, url) {
        Some(value) => value,
        None => return 0,
    };

    let options = ForwardProxyEnrichmentOptions {
        url,
        timeout_ms: u64::try_from(timeout_ms).ok(),
        retries: u32::try_from(retries).ok(),
        proxy_config: convert_java_proxy_config_to_rust(&mut env, &proxy_config),
    };

    match ForwardProxyUserEnrichmentProvider::new(&sdk_key, options) {
        Ok(provider) => InstanceRegistry::register(provider).unwrap_or(0) as jlong,
        Err(error) => {
            log_e!(TAG, "Failed to create enrichment provider: {}", error);
            0
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigFpEnrichmentProviderRelease(
    _env: JNIEnv,
    _class: JClass,
    provider_ref: jlong,
) {
    InstanceRegistry::remove(&(provider_ref as u64));
}

fn make_builder(
    env: &mut JNIEnv,
    provider_ref: jlong,
    json_data: JString,
) -> Result<StatsigFPUserBuilder, String> {
    let provider =
        InstanceRegistry::get::<ForwardProxyUserEnrichmentProvider>(&(provider_ref as u64))
            .ok_or("invalid enrichment provider ref")?;
    let json_data = jstring_to_string(env, json_data).ok_or("invalid user data string")?;
    let user_data: StatsigUserData = serde_json::from_str(&json_data).map_err(|e| e.to_string())?;
    Ok(StatsigFPUserBuilder::from_user_data(provider, user_data))
}

async fn build_payload(builder: StatsigFPUserBuilder) -> String {
    match builder.build().await {
        Ok(user) => match InstanceRegistry::register(user) {
            Some(user_ref) => format!(r#"{{"userRef":{user_ref}}}"#),
            None => error_payload("failed to register enriched StatsigUser"),
        },
        Err(error) => error_payload(&error.to_string()),
    }
}

fn error_payload(message: &str) -> String {
    serde_json::json!({"error": message}).to_string()
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigFpUserBuilderBuildBlocking(
    mut env: JNIEnv,
    _class: JClass,
    provider_ref: jlong,
    json_data: JString,
) -> jstring {
    let payload = match make_builder(&mut env, provider_ref, json_data) {
        Ok(builder) => {
            match StatsigRuntime::get_runtime().get_handle() {
                Ok(handle) => {
                    // SDK callbacks may run on Tokio workers. Let Tokio replace
                    // that worker while this synchronous Java call waits.
                    if tokio::runtime::Handle::try_current().is_ok() {
                        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread {
                            tokio::task::block_in_place(|| handle.block_on(build_payload(builder)))
                        } else {
                            error_payload("Blocking enrichment is unavailable on a current-thread runtime; use build()")
                        }
                    } else {
                        handle.block_on(build_payload(builder))
                    }
                }
                Err(error) => error_payload(&error.to_string()),
            }
        }
        Err(error) => error_payload(&error),
    };
    string_to_jstring(&mut env, payload)
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigFpUserBuilderBuildAsync(
    mut env: JNIEnv,
    _class: JClass,
    provider_ref: jlong,
    json_data: JString,
    callback: JObject,
) -> jboolean {
    let Ok(builder) = make_builder(&mut env, provider_ref, json_data) else {
        return JNI_FALSE;
    };
    let Ok(vm) = env.get_java_vm() else {
        return JNI_FALSE;
    };
    let Ok(callback) = env.new_global_ref(callback) else {
        return JNI_FALSE;
    };
    let Ok(handle) = StatsigRuntime::get_runtime().get_handle() else {
        return JNI_FALSE;
    };
    handle.spawn(async move {
        let payload = build_payload(builder).await;
        let delivered = (|| {
            let mut env = vm.attach_current_thread().ok()?;
            let result = JObject::from(env.new_string(&payload).ok()?);
            env.call_method(
                callback.as_obj(),
                "accept",
                "(Ljava/lang/Object;)V",
                &[JValue::Object(&result)],
            )
            .ok()?;
            Some(())
        })();
        if delivered.is_none() {
            // No Java wrapper took ownership of a successful native result.
            if let Ok(result) = serde_json::from_str::<serde_json::Value>(&payload) {
                if let Some(user_ref) = result.get("userRef").and_then(|value| value.as_u64()) {
                    InstanceRegistry::remove(&user_ref);
                }
            }
        }
    });
    JNI_TRUE
}
