use std::{collections::HashMap, sync::Arc};

use assert_json_diff::assert_json_eq;
use serde_json::{json, Value};
use statsig_rust::{
    dyn_value,
    user::{user_data::UserData, StatsigUserLoggable},
    StatsigUser, StatsigUserDataMap,
};

#[test]
fn test_simple_serialization() {
    let loggable = StatsigUserLoggable {
        data: Arc::new(UserData {
            user_id: Some(dyn_value!("a_user")),
            ..Default::default()
        }),
        environment: None,
        global_custom: None,
    };

    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_eq!(serialized, "{\"userID\":\"a_user\"}");
}

#[test]
fn test_empty_user_serialization() {
    let loggable = StatsigUserLoggable::default();
    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_eq!(serialized, "{}");

    let loggable = StatsigUserLoggable::null();
    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_eq!(serialized, "{}");
}

#[test]
fn test_private_attributes_serialization() {
    let loggable = StatsigUserLoggable {
        data: Arc::new(UserData {
            private_attributes: Some(StatsigUserDataMap::from([(
                "private_attribute_key".to_string(),
                dyn_value!("a_private_attribute_value"),
            )])),
            ..Default::default()
        }),
        environment: None,
        global_custom: None,
    };

    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_eq!(serialized, "{}");
}

#[test]
fn test_full_user_serialization() {
    let loggable = StatsigUserLoggable {
        data: Arc::new(UserData {
            user_id: Some(dyn_value!("a_user")),
            custom_ids: Some(StatsigUserDataMap::from([(
                "custom_id".to_string(),
                dyn_value!("a_value"),
            )])),
            email: Some(dyn_value!("a_email")),
            ip: Some(dyn_value!("a_ip")),
            user_agent: Some(dyn_value!("a_user_agent")),
            country: Some(dyn_value!("a_country")),
            locale: Some(dyn_value!("a_locale")),
            app_version: Some(dyn_value!("a_app_version")),
            custom: Some(StatsigUserDataMap::from([(
                "custom_key".to_string(),
                dyn_value!("a_custom_value"),
            )])),
            private_attributes: Some(StatsigUserDataMap::from([(
                "private_attribute_key".to_string(),
                dyn_value!("a_private_attribute_value"),
            )])),
            statsig_environment: Some(StatsigUserDataMap::from([(
                "statsig_environment_key".to_string(),
                dyn_value!("a_statsig_environment_value"),
            )])),
        }),
        environment: None,
        global_custom: None,
    };

    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_json_eq!(
        serde_json::from_str::<Value>(&serialized).unwrap(),
        json!({
            "userID": "a_user",
            "customIDs": {
                "custom_id": "a_value"
            },
            "email": "a_email",
            "ip": "a_ip",
            "userAgent": "a_user_agent",
            "country": "a_country",
            "locale": "a_locale",
            "appVersion": "a_app_version",
            "custom": {
                "custom_key": "a_custom_value"
            }
        })
    );
}

#[test]
fn test_environment_serialization() {
    let loggable = StatsigUserLoggable {
        data: Arc::new(UserData::default()),
        environment: Some(StatsigUserDataMap::from([(
            "tier".to_string(),
            dyn_value!("a_environment_value"),
        )])),
        global_custom: None,
    };

    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_json_eq!(
        serde_json::from_str::<Value>(&serialized).unwrap(),
        json!({
            "statsigEnvironment": {
                "tier": "a_environment_value"
            },
        })
    );
}

#[test]
fn test_user_custom_overrides_global_custom() {
    let loggable = StatsigUserLoggable {
        data: Arc::new(UserData {
            custom: Some(StatsigUserDataMap::from([(
                "custom_key".to_string(),
                dyn_value!("from_local_custom"),
            )])),
            private_attributes: Some(StatsigUserDataMap::from([(
                "custom_key".to_string(),
                dyn_value!("from_private_custom"),
            )])),
            ..Default::default()
        }),
        environment: None,
        global_custom: Some(HashMap::from([(
            "custom_key".to_string(),
            dyn_value!("from_global_custom"),
        )])),
    };

    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_json_eq!(
        serde_json::from_str::<Value>(&serialized).unwrap(),
        json!({
            "custom": {
                "custom_key": "from_local_custom"
            }
        })
    );
}

#[test]
#[cfg(feature = "ordered_user_data_maps")]
fn test_custom_serialization_preserves_user_data_order() {
    let loggable = StatsigUserLoggable {
        data: Arc::new(UserData {
            custom: Some(StatsigUserDataMap::from([
                ("first".to_string(), dyn_value!("1")),
                ("second".to_string(), dyn_value!("2")),
                ("third".to_string(), dyn_value!("3")),
            ])),
            ..Default::default()
        }),
        environment: None,
        global_custom: None,
    };

    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_eq!(
        serialized,
        "{\"custom\":{\"first\":\"1\",\"second\":\"2\",\"third\":\"3\"}}"
    );
}

#[test]
#[cfg(feature = "ordered_user_data_maps")]
fn test_custom_serialization_merges_without_duplicate_keys() {
    let loggable = StatsigUserLoggable {
        data: Arc::new(UserData {
            custom: Some(StatsigUserDataMap::from([
                ("shared".to_string(), dyn_value!("from_user")),
                ("user_only".to_string(), dyn_value!("from_user_only")),
            ])),
            ..Default::default()
        }),
        environment: None,
        global_custom: Some(HashMap::from([(
            "shared".to_string(),
            dyn_value!("from_global"),
        )])),
    };

    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_eq!(
        serialized,
        "{\"custom\":{\"shared\":\"from_user\",\"user_only\":\"from_user_only\"}}"
    );
}

#[test]
fn test_global_custom_serialization() {
    let mut user = StatsigUser::with_user_id("a_user");
    user.set_custom(HashMap::from([("user_custom_key", "user_custom_value")]));

    let loggable = StatsigUserLoggable {
        data: user.data,
        environment: None,
        global_custom: Some(HashMap::from([(
            "global_custom_key".to_string(),
            dyn_value!("global_custom_value"),
        )])),
    };

    let serialized = serde_json::to_string(&loggable).unwrap();
    assert_json_eq!(
        serde_json::from_str::<Value>(&serialized).unwrap(),
        json!({
            "userID": "a_user",
            "custom": {
                "user_custom_key": "user_custom_value",
                "global_custom_key": "global_custom_value"
            }
        })
    );
}
