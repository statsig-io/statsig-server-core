use serde_json::json;
use statsig_rust::{
    dyn_value,
    user::{StatsigUserInternal, StatsigUserLoggable, UserLoggableData},
    DynamicValue, Statsig, StatsigOptions, StatsigUser,
};

use std::{collections::HashMap, sync::Arc};

lazy_static::lazy_static! {
    static ref FULL_USER: StatsigUser = StatsigUser {
        user_id: Some(dyn_value!("a_user")),
        email: Some(dyn_value!("daniel@statsig.com")),
        ip: Some(dyn_value!("127.0.0.1")),
        user_agent: Some(dyn_value!("statsig-rust/0.1.0")),
        country: Some(dyn_value!("US")),
        locale: Some(dyn_value!("en-US")),
        app_version: Some(dyn_value!("1.0.0")),
        custom_ids: Some(HashMap::from([
            ("companyID".into(), dyn_value!("statsig")),
            ("groupID".to_string(), dyn_value!("sdk_team"),
        )])),
        custom: Some(HashMap::from([(
            "test_custom_field".to_string(),
            dyn_value!("test_custom_field_value"),
        )])),
        private_attributes: Some(HashMap::from([(
            "test_private_attribute".to_string(),
            dyn_value!("test_private_attribute_value"),
        )])),
    };
}

#[test]
fn test_get_full_user_key_matches_for_multiple_simple_users() {
    let user_data = StatsigUser::with_user_id("user1".into());
    let user1 = StatsigUserInternal::new(&user_data, None);
    let user2 = StatsigUserInternal::new(&user_data, None);

    assert_eq!(user1.get_full_user_key(), user2.get_full_user_key());
}

#[test]
fn test_get_full_user_key_matches_for_multiple_full_users() {
    let user_data1 = FULL_USER.clone();
    let user_data2 = FULL_USER.clone();
    let user1 = StatsigUserInternal::new(&user_data1, None);
    let user2 = StatsigUserInternal::new(&user_data2, None);

    assert_eq!(user1.get_full_user_key(), user2.get_full_user_key());
}

#[test]
fn test_get_full_user_key_mismatch_for_multiple_users() {
    let user_data1 = StatsigUser::with_user_id("user1".into());
    let user_data2 = StatsigUser::with_user_id("user2".into());
    let user1 = StatsigUserInternal::new(&user_data1, None);
    let user2 = StatsigUserInternal::new(&user_data2, None);

    assert_ne!(user1.get_full_user_key(), user2.get_full_user_key());
}

#[test]
fn test_get_full_user_key_mismatch_for_almost_identical_users() {
    let user_data1 = FULL_USER.clone();
    let mut user_data2 = FULL_USER.clone();
    user_data2.custom_ids = Some(HashMap::from([(
        "companyID".to_string(),
        dyn_value!("statsig_2"),
    )]));
    let user1 = StatsigUserInternal::new(&user_data1, None);
    let user2 = StatsigUserInternal::new(&user_data2, None);

    assert_ne!(user1.get_full_user_key(), user2.get_full_user_key());
}

#[test]
fn test_get_full_user_key_matches_regardless_of_sorting() {
    let user_data = FULL_USER.clone();
    let mut user_data2 = FULL_USER.clone();

    let mut custom_ids = HashMap::new();
    custom_ids.insert("groupID".to_string(), dyn_value!("sdk_team"));
    custom_ids.insert("companyID".to_string(), dyn_value!("statsig"));

    user_data2.custom_ids = Some(custom_ids);
    let user1 = StatsigUserInternal::new(&user_data, None);
    let user2 = StatsigUserInternal::new(&user_data2, None);

    assert_eq!(user1.get_full_user_key(), user2.get_full_user_key());
}

#[test]
fn test_get_full_user_key_matches_with_statsig_env() {
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            environment: Some("dev".to_string()),
            ..StatsigOptions::default()
        })),
    );

    let user1 = StatsigUserInternal::new(&FULL_USER, Some(&statsig));
    let user2 = StatsigUserInternal::new(&FULL_USER, Some(&statsig));

    assert_eq!(user1.get_full_user_key(), user2.get_full_user_key());
}

#[test]
fn test_get_full_user_key_mismatch_without_statsig_env() {
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            environment: Some("dev".to_string()),
            ..StatsigOptions::default()
        })),
    );

    let user1 = StatsigUserInternal::new(&FULL_USER, Some(&statsig));
    let user2 = StatsigUserInternal::new(&FULL_USER, None);

    assert_ne!(user1.get_full_user_key(), user2.get_full_user_key());
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

    let private_attrs = loggable.data.value.get("private_attributes");
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

    let deserialized_user_id = deserialized.data.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    let deserialized_statsig_env = deserialized.data.value.get("statsigEnvironment").cloned();
    assert_eq!(deserialized_statsig_env, Some(json!({"tier": "dev"})));

    let deserialized_custom = deserialized.data.value.get("custom").cloned();
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

    let deserialized_user_id = deserialized.data.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    assert_eq!(deserialized.data.value.as_object().unwrap().keys().len(), 1);
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

    let deserialized_user_id = deserialized.data.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    let deserialized_custom = deserialized.data.value.get("custom").cloned();
    assert_eq!(
        deserialized_custom,
        Some(json!({"test_custom_field": "test_custom_field_value"}))
    );

    assert_eq!(deserialized.data.value.as_object().unwrap().keys().len(), 2);
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

    let deserialized_user_id = deserialized.data.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    let deserialized_statsig_env = deserialized.data.value.get("statsigEnvironment").cloned();
    assert_eq!(deserialized_statsig_env, Some(json!({"tier": "dev"})));

    let deserialized_custom = deserialized.data.value.get("custom").cloned();
    assert_eq!(
        deserialized_custom,
        Some(json!({
                "test_local_custom_field": "test_local_custom_field_value",
                "test_custom_field": "test_custom_field_value"
        }))
    );
}

#[test]
fn test_serialization_with_env() {
    let user = create_test_user(None);
    let statsig = create_statsig_instance(None, Some("dev".to_string()));
    let user_internal = StatsigUserInternal::new(&user, Some(&statsig));
    let deserialized = serialize_and_deserialize(&user_internal);

    let deserialized_user_id = deserialized.data.value.get("userID").cloned();
    assert_eq!(deserialized_user_id, Some(json!("test_user_id")));

    let deserialized_statsig_env = deserialized.data.value.get("statsigEnvironment").cloned();
    assert_eq!(deserialized_statsig_env, Some(json!({"tier": "dev"})));
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

    let keys = deserialized.data.value.as_object().unwrap();
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

// ---------------------------------------------------------------------------------------------- [Helpers]

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
    let value = serde_json::from_str(&serialized).unwrap();

    StatsigUserLoggable {
        data: Arc::new(UserLoggableData {
            key: "".to_string(),
            value,
        }),
    }
}
