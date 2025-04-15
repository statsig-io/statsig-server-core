use serde_json::json;
use statsig_rust::{
    user::{StatsigUserInternal, StatsigUserLoggable},
    DynamicValue, Statsig, StatsigOptions, StatsigUser,
};

use std::{collections::HashMap, sync::Arc};

fn create_test_user(custom_fields: Option<HashMap<String, DynamicValue>>) -> StatsigUser {
    StatsigUser {
        custom: custom_fields,
        ..StatsigUser::with_user_id("test_user_id".to_string())
    }
}

fn create_statsig_instance(
    global_custom: Option<HashMap<String, DynamicValue>>,
    environment: Option<String>,
) -> Statsig {
    let options = StatsigOptions {
        global_custom_fields: global_custom,
        environment,
        ..Default::default()
    };
    Statsig::new("secret-key", Some(Arc::new(options)))
}

fn serialize_and_deserialize(user_internal: &StatsigUserInternal) -> StatsigUserLoggable {
    let loggable = user_internal.to_loggable();
    let serialized = serde_json::to_string(&loggable).unwrap();
    serde_json::from_str(&serialized).unwrap()
}

#[test]
fn test_loggable_strips_private_attributes() {
    let mut private_attrs = HashMap::new();
    private_attrs.insert("secret".to_string(), DynamicValue::from("sensitive_data"));

    let user = StatsigUser {
        private_attributes: Some(private_attrs),
        ..StatsigUser::with_user_id("test_user".to_string())
    };

    let user_internal = StatsigUserInternal::new(&user, None);
    let loggable = user_internal.to_loggable();

    let private_attrs = loggable.value.get("private_attributes");
    assert!(private_attrs.is_none());
}

#[test]
fn test_serialization_with_global_custom_fields() {
    let user = create_test_user(None);
    let global_custom = HashMap::from([(
        "test_custom_field".to_string(),
        DynamicValue::from("test_custom_field_value"),
    )]);

    let statsig = create_statsig_instance(Some(global_custom), Some("dev".to_string()));
    let user_internal = StatsigUserInternal::new(&user, Some(&statsig));
    let deserialized = serialize_and_deserialize(&user_internal);

    let deserialized_user_id = deserialized.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    let deserialized_statsig_env = deserialized.value.get("statsigEnvironment").cloned();
    assert_eq!(deserialized_statsig_env, Some(json!({"tier": "dev"})));

    let deserialized_custom = deserialized.value.get("custom").cloned();
    assert_eq!(
        deserialized_custom,
        Some(json!({"test_custom_field": "test_custom_field_value"}))
    );
}

#[test]
fn test_serialization_with_no_custom_fields() {
    let user = create_test_user(None);
    let user_internal = StatsigUserInternal::new(&user, None);
    let deserialized = serialize_and_deserialize(&user_internal);

    let deserialized_user_id = deserialized.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    assert_eq!(deserialized.value.as_object().unwrap().keys().len(), 1);
}

#[test]
fn test_serialization_with_local_custom_fields() {
    let custom_fields = HashMap::from([(
        "test_custom_field".to_string(),
        DynamicValue::from("test_custom_field_value"),
    )]);
    let user = create_test_user(Some(custom_fields));

    let user_internal = StatsigUserInternal::new(&user, None);
    let deserialized = serialize_and_deserialize(&user_internal);

    let deserialized_user_id = deserialized.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    let deserialized_custom = deserialized.value.get("custom").cloned();
    assert_eq!(
        deserialized_custom,
        Some(json!({"test_custom_field": "test_custom_field_value"}))
    );

    assert_eq!(deserialized.value.as_object().unwrap().keys().len(), 2);
}

#[test]
fn test_serialization_with_local_custom_fields_and_global_custom_fields() {
    let local_custom = HashMap::from([(
        "test_local_custom_field".to_string(),
        DynamicValue::from("test_local_custom_field_value"),
    )]);
    let user = create_test_user(Some(local_custom));

    let global_custom = HashMap::from([(
        "test_custom_field".to_string(),
        DynamicValue::from("test_custom_field_value"),
    )]);

    let statsig = create_statsig_instance(Some(global_custom), Some("dev".to_string()));
    let user_internal = StatsigUserInternal::new(&user, Some(&statsig));
    let deserialized = serialize_and_deserialize(&user_internal);

    let deserialized_user_id = deserialized.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    let deserialized_statsig_env = deserialized.value.get("statsigEnvironment").cloned();
    assert_eq!(deserialized_statsig_env, Some(json!({"tier": "dev"})));

    let deserialized_custom = deserialized.value.get("custom").cloned();
    assert_eq!(
        deserialized_custom,
        Some(json!({
                "test_local_custom_field": "test_local_custom_field_value",
                "test_custom_field": "test_custom_field_value"
        }))
    );
}

#[test]
fn test_serialization_has_correct_keys() {
    let user = StatsigUser {
        custom: Some(HashMap::from([(
            "test_custom_field".to_string(),
            DynamicValue::from("test_custom_field_value"),
        )])),
        private_attributes: Some(HashMap::from([(
            "test_private_attribute".to_string(),
            DynamicValue::from("test_private_attribute_value"),
        )])),
        email: Some("test_email".into()),
        ip: Some("test_ip".into()),
        user_agent: Some("test_user_agent".into()),
        country: Some("test_country".into()),
        locale: Some("test_locale".into()),
        app_version: Some("test_app_version".into()),
        custom_ids: Some(HashMap::from([(
            "test_custom_id".to_string(),
            DynamicValue::from("test_custom_id_value"),
        )])),
        ..StatsigUser::with_user_id("test_user_id".to_string())
    };
    let user_internal = StatsigUserInternal::new(&user, None);
    let deserialized = serialize_and_deserialize(&user_internal);

    let keys = deserialized.value.as_object().unwrap();
    assert!(keys.contains_key("userID"));
    assert!(keys.contains_key("email"));
    assert!(keys.contains_key("ip"));
    assert!(keys.contains_key("userAgent"));
    assert!(keys.contains_key("country"));
    assert!(keys.contains_key("locale"));
    assert!(keys.contains_key("appVersion"));
    assert!(keys.contains_key("custom"));
    assert!(keys.contains_key("customIDs"));

    assert!(!keys.contains_key("privateAttributes"));
}
