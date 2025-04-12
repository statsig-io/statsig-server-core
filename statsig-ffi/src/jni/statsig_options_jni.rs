use crate::jni::jni_utils::{jboolean_to_bool, jstring_to_string, string_to_jstring};
use crate::jni::statsig_observability_client_jni::convert_to_ob_rust;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;
use statsig_rust::{log_d, log_e, log_w, statsig_options::StatsigOptionsBuilder};
use statsig_rust::{InstanceRegistry, ObservabilityClient, StatsigOptions};
use std::sync::Arc;

const TAG: &str = "StatsigOptionsJNI";

pub struct StatsigOptionsJNI {
    pub inner: Arc<StatsigOptions>,
    pub _strong_obs_client: Option<Arc<dyn ObservabilityClient>>,
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOptionsCreate(
    mut env: JNIEnv,
    _class: JClass,
    specs_url: JString,
    log_event_url: JString,
    id_lists_url: JString,
    id_lists_sync_interval_ms: jlong,
    specs_sync_interval_ms: jlong,
    event_logging_flush_interval_ms: jlong,
    event_logging_max_queue_size: jlong,
    init_timeout_ms: jlong,
    environment: JString,
    output_logger_level: jint,
    service_name: JString,
    observability_client: JObject,
    enable_id_lists: jboolean,
    enable_country_lookup: jboolean,
    disable_all_logging: jboolean,
    enable_user_agent_parsing: jboolean,
    disable_network: jboolean,
) -> jstring {
    let specs_url = jstring_to_string(&mut env, specs_url);
    let log_event_url = jstring_to_string(&mut env, log_event_url);
    let id_lists_url = jstring_to_string(&mut env, id_lists_url);
    let environment = jstring_to_string(&mut env, environment);
    let enable_id_lists = jboolean_to_bool(enable_id_lists);
    let enable_user_agent_parsing = jboolean_to_bool(enable_user_agent_parsing);
    let enable_country_lookup = jboolean_to_bool(enable_country_lookup);
    let disable_all_logging = jboolean_to_bool(disable_all_logging);
    let disable_network = jboolean_to_bool(disable_network);

    let service_name = jstring_to_string(&mut env, service_name);

    let specs_sync_interval_ms = if specs_sync_interval_ms > 0 {
        Some(specs_sync_interval_ms as u32)
    } else {
        None
    };

    let id_lists_sync_interval_ms = if id_lists_sync_interval_ms > 0 {
        Some(id_lists_sync_interval_ms as u32)
    } else {
        None
    };

    if event_logging_flush_interval_ms > 0 {
        log_w!(TAG, "Warning: `event_logging_flush_interval_ms` is deprecated in favor of smart log event feature. It will not be used and is safe to remove.");
    };

    let event_logging_max_queue_size = if event_logging_max_queue_size > 0 {
        Some(event_logging_max_queue_size as u32)
    } else {
        None
    };

    let init_timeout_ms_option = if init_timeout_ms > 0 {
        Some(init_timeout_ms as u64)
    } else {
        None
    };

    let (strong_ob, weak_ob) = convert_to_ob_rust(&env, observability_client);

    let mut builder = StatsigOptionsBuilder::new();

    builder = builder
        .specs_url(specs_url)
        .log_event_url(log_event_url)
        .specs_sync_interval_ms(specs_sync_interval_ms)
        .event_logging_max_queue_size(event_logging_max_queue_size)
        .environment(environment)
        .id_lists_url(id_lists_url)
        .id_lists_sync_interval_ms(id_lists_sync_interval_ms)
        .observability_client(weak_ob)
        .enable_id_lists(enable_id_lists)
        .disable_all_logging(disable_all_logging)
        .output_log_level(Some(output_logger_level as u32))
        .service_name(service_name)
        .enable_user_agent_parsing(enable_user_agent_parsing)
        .enable_country_lookup(enable_country_lookup)
        .disable_network(disable_network)
        .init_timeout_ms(init_timeout_ms_option);

    let options = builder.build();

    let options_jni = StatsigOptionsJNI {
        inner: Arc::new(options),
        _strong_obs_client: strong_ob,
    };

    let id = InstanceRegistry::register(options_jni);
    match id {
        Some(id) => {
            log_d!(TAG, "Created StatsigOptions with ID {}", id);
            string_to_jstring(&mut env, id.to_string())
        }
        None => {
            log_e!(TAG, "Failed to create StatsigOptions");
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOptionsRelease(
    mut env: JNIEnv,
    _class: JClass,
    options_ref: JString,
) {
    if let Some(id) = jstring_to_string(&mut env, options_ref) {
        InstanceRegistry::remove(&id);
    }
}
