use crate::jni::jni_utils::{jboolean_to_bool, jstring_to_string, string_to_jstring};
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;
use sigstat::{instance_store::INST_STORE, log_d, log_e, statsig_options::StatsigOptionsBuilder};

const TAG: &str = "StatsigOptionsJNI";

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOptionsCreate(
    mut env: JNIEnv,
    _class: JClass,
    specs_url: JString,
    log_event_url: JString,
    id_lists_url: JString,
    specs_sync_interval_ms: jlong,
    event_logging_flush_interval_ms: jlong,
    event_logging_max_queue_size: jlong,
    environment: JString,
    enable_id_lists: jboolean,
    disable_all_logging: jboolean,
    output_logger_level: jint,
    service_name: JString,
    enable_user_agent_parsing: jboolean,
    enable_country_lookup: jboolean,
) -> jstring {
    let specs_url = jstring_to_string(&mut env, specs_url);
    let log_event_url = jstring_to_string(&mut env, log_event_url);
    let id_lists_url = jstring_to_string(&mut env, id_lists_url);
    let environment = jstring_to_string(&mut env, environment);
    let enable_id_lists = jboolean_to_bool(enable_id_lists);
    let enable_user_agent_parsing = jboolean_to_bool(enable_user_agent_parsing);
    let enable_country_lookup = jboolean_to_bool(enable_country_lookup);
    let disable_all_logging = jboolean_to_bool(disable_all_logging);
    let service_name = jstring_to_string(&mut env, service_name);

    let specs_sync_interval_ms = if specs_sync_interval_ms > 0 {
        Some(specs_sync_interval_ms as u32)
    } else {
        None
    };

    let event_logging_flush_interval_ms = if event_logging_flush_interval_ms > 0 {
        Some(event_logging_flush_interval_ms as u32)
    } else {
        None
    };

    let event_logging_max_queue_size = if event_logging_max_queue_size > 0 {
        Some(event_logging_max_queue_size as u32)
    } else {
        None
    };

    let mut builder = StatsigOptionsBuilder::new();

    builder = builder
        .specs_url(specs_url)
        .log_event_url(log_event_url)
        .specs_sync_interval_ms(specs_sync_interval_ms)
        .event_logging_flush_interval_ms(event_logging_flush_interval_ms)
        .event_logging_max_queue_size(event_logging_max_queue_size)
        .environment(environment)
        .id_lists_url(id_lists_url)
        .enable_id_lists(enable_id_lists)
        .disable_all_logging(disable_all_logging)
        .output_log_level(Some(output_logger_level as u32))
        .service_name(service_name)
        .enable_user_agent_parsing(enable_user_agent_parsing)
        .enable_country_lookup(enable_country_lookup);

    let options = builder.build();
    let id = INST_STORE.add(options);
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
        INST_STORE.remove(&id);
    }
}
