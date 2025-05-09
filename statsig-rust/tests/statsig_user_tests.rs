use statsig_rust::{dyn_value, StatsigUser};
use std::collections::HashMap;

#[test]
fn test_creation_with_user_id() {
    let user = StatsigUser::with_user_id("user1".to_string());
    assert_eq!(user.user_id, Some(dyn_value!("user1")));
}

#[test]
fn test_creation_with_custom_ids() {
    let user = StatsigUser::with_custom_ids(HashMap::from([(
        "companyID".to_string(),
        "statsig".to_string(),
    )]));
    assert_eq!(
        user.custom_ids,
        Some(HashMap::from([(
            "companyID".to_string(),
            dyn_value!("statsig")
        )]))
    );
}

#[test]
fn test_setting_string_fields() {
    let mut user = StatsigUser::with_user_id("".to_string());
    user.email = Some(dyn_value!("test@test.com"));
    user.ip = Some(dyn_value!("127.0.0.1"));
    user.user_agent = Some(dyn_value!("test"));
    user.country = Some(dyn_value!("US"));
    user.locale = Some(dyn_value!("en-US"));
    user.app_version = Some(dyn_value!("1.0.0"));

    assert_eq!(user.email, Some(dyn_value!("test@test.com")));
    assert_eq!(user.ip, Some(dyn_value!("127.0.0.1")));
    assert_eq!(user.user_agent, Some(dyn_value!("test")));
    assert_eq!(user.country, Some(dyn_value!("US")));
    assert_eq!(user.locale, Some(dyn_value!("en-US")));
}

#[test]
fn test_changing_string_fields() {
    let mut user = StatsigUser::with_user_id("".to_string());
    user.email = Some(dyn_value!("test@test.com"));
    user.email = None;
    user.ip = Some(dyn_value!("127.0.0.1"));
    user.ip = Some(dyn_value!("0.0.0.0"));

    assert_eq!(user.email, None);
    assert_eq!(user.ip, Some(dyn_value!("0.0.0.0")));
}

#[test]
fn test_setting_attr_map_fields() {
    let custom = HashMap::from([("test_custom".to_string(), dyn_value!(1))]);
    let priv_attr = HashMap::from([("test_private".to_string(), dyn_value!(2))]);

    let mut user = StatsigUser::with_user_id("".to_string());

    user.custom = Some(custom.clone());
    assert_eq!(user.custom, Some(custom));

    user.private_attributes = Some(priv_attr.clone());
    assert_eq!(user.private_attributes, Some(priv_attr));
}
