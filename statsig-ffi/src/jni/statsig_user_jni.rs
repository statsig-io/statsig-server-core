use jni::JNIEnv;
use jni::sys::{jboolean, jclass, jlong, JNI_FALSE, JNI_TRUE, jstring};
use jni::objects::{JClass, JString};
use statsig::statsig_user::StatsigUserBuilder;
use crate::ffi_utils::{parse_json_to_map};
use crate::instance_manager::{ USER_INSTANCES};
use crate::jni::jni_utils::jstring_to_string;

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigUserCreate(
    mut env: JNIEnv,
    _class: JClass,
    user_id: JString,
    custom_ids_json: JString,
    email: JString,
    ip: JString,
    user_agent: JString,
    country: JString,
    locale: JString,
    app_version: JString,
    custom_json: JString,
    private_attributes_json: JString,
) -> jlong {
    let user_id = jstring_to_string(&mut env, user_id);
    let custom_ids = parse_json_to_map(jstring_to_string(&mut env, custom_ids_json));
    let email = jstring_to_string(&mut env, email);
    let ip = jstring_to_string(&mut env, ip);
    let user_agent = jstring_to_string(&mut env, user_agent);
    let country = jstring_to_string(&mut env, country);
    let locale = jstring_to_string(&mut env, locale);
    let app_version = jstring_to_string(&mut env, app_version);
    let custom = parse_json_to_map(jstring_to_string(&mut env, custom_json));
    let private_attributes = parse_json_to_map(jstring_to_string(&mut env, private_attributes_json));

    let mut builder = match custom_ids {
        Some(custom_ids) => StatsigUserBuilder::new_with_custom_ids(custom_ids).user_id(user_id),
        None => {
            StatsigUserBuilder::new_with_user_id(user_id.unwrap_or_default())
                .custom_ids(None)
        }
    };

    builder = builder
        .email(email)
        .ip(ip)
        .user_agent(user_agent)
        .country(country)
        .locale(locale)
        .app_version(app_version)
        .custom(custom)
        .private_attributes(private_attributes);

    let user = builder.build();
    let id = USER_INSTANCES.add(user);
    println!("Created StatsigUser {}", id);
    id
}

#[no_mangle]
pub extern "system" fn Java_com_statsig_StatsigJNI_statsigUserRelease(
    _env: JNIEnv,
    _class: JClass,
    id: jlong,
) {
    USER_INSTANCES.release(id);
}

