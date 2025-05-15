use more_asserts::assert_gt;
use serde_json::json;
use statsig_rust::{
    dyn_value,
    user::{statsig_user_internal::FullUserKey, StatsigUserInternal, StatsigUserLoggable},
    DynamicValue, Statsig, StatsigOptions, StatsigUser, StatsigUserBuilder,
};

use std::{collections::HashMap, sync::Arc, time::Instant};

lazy_static::lazy_static! {
    static ref FULL_USER: StatsigUser = StatsigUserBuilder::new_with_user_id("a_user".to_string())
        .email(Some("daniel@statsig.com".into()))
        .ip(Some("127.0.0.1".into()))
        .user_agent(Some("statsig-rust/0.1.0".into()))
        .country(Some("US".into()))
        .locale(Some("en-US".into()))
        .app_version(Some("1.0.0".into()))
        .custom_ids(Some(HashMap::from([("companyID", "statsig")])))
        .custom(Some(HashMap::from([(
            "test_custom_field".to_string(),
            dyn_value!("test_custom_field_value"),
        )])))
        .private_attributes(Some(HashMap::from([(
            "test_private_attribute".to_string(),
            dyn_value!("test_private_attribute_value"),
        )])))
        .build();
}

#[test]
fn test_get_full_user_key_matches_for_multiple_simple_users() {
    let user_data = StatsigUser::with_user_id("user1");
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
    let user_data1 = StatsigUser::with_user_id("user1");
    let user_data2 = StatsigUser::with_user_id("user2");
    let user1 = StatsigUserInternal::new(&user_data1, None);
    let user2 = StatsigUserInternal::new(&user_data2, None);

    assert_ne!(user1.get_full_user_key(), user2.get_full_user_key());
}

#[test]
fn test_get_full_user_key_mismatch_for_almost_identical_users() {
    let user1 = FULL_USER.clone();
    let mut user2 = FULL_USER.clone();
    user2.set_custom_ids(HashMap::from([("companyID", "statsig_2")]));

    let user1 = StatsigUserInternal::new(&user1, None);
    let user2 = StatsigUserInternal::new(&user2, None);

    assert_ne!(user1.get_full_user_key(), user2.get_full_user_key());
}

#[test]
fn test_get_full_user_key_matches_regardless_of_sorting() {
    let mut user1 = FULL_USER.clone();
    let mut user2 = FULL_USER.clone();

    user1.set_custom_ids(HashMap::from([
        ("companyID", "statsig"),
        ("groupID", "sdk_team"),
    ]));
    user2.set_custom_ids(HashMap::from([
        ("groupID", "sdk_team"),
        ("companyID", "statsig"),
    ]));

    let user1 = StatsigUserInternal::new(&user1, None);
    let user2 = StatsigUserInternal::new(&user2, None);

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
fn test_full_user_key() {
    let start = Instant::now();

    let mut previous: FullUserKey = (0, 0, 0, 0, 0, 0, 0, vec![], vec![], vec![], vec![]);
    for i in 0..10000 {
        let user = StatsigUserBuilder::new_with_user_id(format!("a_user_{}", i))
            .app_version(Some("1.0.0".to_string()))
            .country(Some("US".to_string()))
            .email(Some("daniel@statsig.com".to_string()))
            .ip(Some("127.0.0.1".to_string()))
            .locale(Some("en-US".to_string()))
            .user_agent(Some("statsig-rust/0.1.0".to_string()))
            .custom_ids(Some(HashMap::from([(
                "companyID".to_string(),
                "statsig".to_string(),
            )])))
            .custom(Some(HashMap::from([(
                "test_custom_field".to_string(),
                dyn_value!("test_custom_field_value"),
            )])))
            .private_attributes(Some(HashMap::from([(
                "test_private_attribute".to_string(),
                dyn_value!("test_private_attribute_value"),
            )])))
            .build();

        let user_internal = StatsigUserInternal::new(&user, None);
        let new_key = user_internal.get_full_user_key();
        assert_ne!(previous, new_key);
        assert_gt!(new_key.0, 0);
        previous = new_key;
    }
    let end = Instant::now();
    println!("Time taken: {:?}", end.duration_since(start));
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

    let user = StatsigUserBuilder::new_with_user_id("test_user".to_string())
        .private_attributes(Some(private_attrs))
        .build();

    let user_internal = StatsigUserInternal::new(&user, None);
    let loggable = user_internal.to_loggable();

    let serialized = json!(loggable);

    let private_attrs = &serialized.get("privateAttributes");
    assert!(private_attrs.is_none());

    let private_attrs = &serialized.get("private_attributes");
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

    let deserialized_user_id = &deserialized.data.user_id;
    assert_eq!(deserialized_user_id, &Some(dyn_value!("test_user_id")));

    let deserialized_statsig_env = deserialized
        .environment
        .as_ref()
        .and_then(|x| x.get("tier").cloned());
    assert_eq!(deserialized_statsig_env, Some(dyn_value!("dev")));

    let deserialized_custom = &deserialized
        .data
        .custom
        .as_ref()
        .unwrap()
        .get("test_custom_field");
    assert_eq!(
        deserialized_custom,
        &Some(&dyn_value!("test_custom_field_value"))
    );
}

#[test]
fn test_serialization_with_no_custom_fields() {
    let user = create_test_user(None);
    let user_internal = StatsigUserInternal::new(&user, None);
    let deserialized = serialize_and_deserialize(&user_internal);

    let deserialized_user_id = &deserialized.data.user_id;
    assert_eq!(deserialized_user_id, &Some(dyn_value!("test_user_id")));

    assert!(deserialized.data.custom.as_ref().is_none());
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

    let deserialized_user_id = &deserialized.data.user_id;
    assert_eq!(deserialized_user_id, &Some(dyn_value!("test_user_id")));

    let deserialized_custom = &deserialized
        .data
        .custom
        .as_ref()
        .unwrap()
        .get("test_custom_field");
    assert_eq!(
        deserialized_custom,
        &Some(&dyn_value!("test_custom_field_value"))
    );

    assert_eq!(deserialized.data.custom.as_ref().unwrap().len(), 1);
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

    let deserialized_user_id = &deserialized.data.user_id;
    assert_eq!(deserialized_user_id, &Some(dyn_value!("test_user_id")));

    let deserialized_statsig_env = deserialized
        .environment
        .as_ref()
        .and_then(|x| x.get("tier").cloned());
    assert_eq!(deserialized_statsig_env, Some(dyn_value!("dev")));

    let deserialized_custom = &deserialized
        .data
        .custom
        .as_ref()
        .unwrap()
        .get("test_local_custom_field");
    assert_eq!(
        deserialized_custom,
        &Some(&dyn_value!("test_local_custom_field_value"))
    );
}

#[test]
fn test_serialization_with_env() {
    let user = create_test_user(None);
    let statsig = create_statsig_instance(None, Some("dev".to_string()));
    let user_internal = StatsigUserInternal::new(&user, Some(&statsig));
    let deserialized = serialize_and_deserialize(&user_internal);

    let deserialized_user_id = &deserialized.data.user_id;
    assert_eq!(deserialized_user_id, &Some(dyn_value!("test_user_id")));

    let deserialized_statsig_env = deserialized
        .environment
        .as_ref()
        .and_then(|x| x.get("tier").cloned());
    assert_eq!(deserialized_statsig_env, Some(dyn_value!("dev")));
}

#[test]
fn test_serialization_without_env() {
    let user = create_test_user(None);
    let statsig = create_statsig_instance(None, None);
    let user_internal = StatsigUserInternal::new(&user, Some(&statsig));
    let deserialized = serialize_and_deserialize(&user_internal);

    let deserialized_user_id = &deserialized.data.user_id;
    assert_eq!(deserialized_user_id, &Some(dyn_value!("test_user_id")));

    assert!(deserialized.environment.is_none());
}

#[test]
fn test_serialization_has_correct_keys() {
    let user = StatsigUserBuilder::new_with_user_id("test_user_id".to_string())
        .custom(Some(HashMap::from([(
            "test_custom_field".to_string(),
            DynamicValue::from("test_custom_field_value"),
        )])))
        .private_attributes(Some(HashMap::from([(
            "test_private_attribute".to_string(),
            DynamicValue::from("test_private_attribute_value"),
        )])))
        .email(Some("test_email".into()))
        .ip(Some("test_ip".into()))
        .user_agent(Some("test_user_agent".into()))
        .country(Some("test_country".into()))
        .locale(Some("test_locale".into()))
        .app_version(Some("test_app_version".into()))
        .custom_ids(Some(HashMap::from([(
            "test_custom_id".to_string(),
            "test_custom_id_value".to_string(),
        )])))
        .build();

    let user_internal = StatsigUserInternal::new(&user, None);
    let deserialized = serialize_and_deserialize(&user_internal);

    let keys = deserialized.data.as_ref();
    assert!(keys.user_id.is_some());
    assert!(keys.email.is_some());
    assert!(keys.ip.is_some());
    assert!(keys.user_agent.is_some());
    assert!(keys.country.is_some());
    assert!(keys.locale.is_some());
    assert!(keys.app_version.is_some());
    assert!(keys.custom.is_some());
    assert!(keys.custom_ids.is_some());

    assert!(keys.private_attributes.is_none());
}

// ---------------------------------------------------------------------------------------------- [Helpers]

fn create_test_user(custom_fields: Option<HashMap<String, DynamicValue>>) -> StatsigUser {
    StatsigUserBuilder::new_with_user_id("test_user_id".to_string())
        .custom(custom_fields)
        .build()
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
