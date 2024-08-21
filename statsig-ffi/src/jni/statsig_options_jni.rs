use jni::JNIEnv;
use jni::sys::{ jlong, jint };
use jni::objects::{JClass, JString};
use statsig::statsig_options::{ StatsigOptionsBuilder };
use crate::instance_manager::{ OPTIONS_INSTANCES };
use crate::jni::jni_utils::jstring_to_string;

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOptionsCreate(
    mut env: JNIEnv,
    _class: JClass,
    specs_url: JString,
    log_event_url: JString,
    specs_sync_interval_ms: jint,
    event_logging_flush_interval_ms: jint,
    event_logging_max_queue_size: jint,
    environment: JString,
) -> jlong {
    let specs_url = jstring_to_string(&mut env, specs_url);
    let log_event_url = jstring_to_string(&mut env, log_event_url);
    let environment = jstring_to_string(&mut env, environment);

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
        .environment(environment);

    let options = builder.build();
    let id = OPTIONS_INSTANCES.add(options);
    println!("Created StatsigOptions {}", id);
    id
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigOptionsRelease(
    _env: JNIEnv,
    _class: JClass,
    id: jlong,
) {
    OPTIONS_INSTANCES.release(id);
}
