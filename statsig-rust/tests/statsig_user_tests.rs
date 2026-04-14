use statsig_rust::{dyn_value, StatsigUser, StatsigUserDataMap};
use std::collections::HashMap;

#[test]
fn test_creation_with_user_id() {
    let user = StatsigUser::with_user_id("user1".to_string());
    assert_eq!(user.data.user_id, Some(dyn_value!("user1")));
}

#[test]
fn test_creation_with_custom_ids() {
    let user = StatsigUser::with_custom_ids(HashMap::from([(
        "companyID".to_string(),
        "statsig".to_string(),
    )]));
    assert_eq!(
        user.data.custom_ids,
        Some(StatsigUserDataMap::from([(
            "companyID".to_string(),
            dyn_value!("statsig")
        )]))
    );
}

#[test]
#[cfg(not(feature = "ordered_user_data_maps"))]
fn test_statsig_user_data_accepts_hashmap_fields_by_default() {
    let custom_ids = HashMap::from([("companyID".to_string(), dyn_value!("statsig"))]);
    let custom = HashMap::from([("plan".to_string(), dyn_value!("enterprise"))]);
    let private_attributes = HashMap::from([("secret".to_string(), dyn_value!("value"))]);
    let statsig_environment = HashMap::from([("tier".to_string(), dyn_value!("production"))]);

    let user = StatsigUser::new(statsig_rust::StatsigUserData {
        user_id: Some(dyn_value!("user1")),
        custom_ids: Some(custom_ids.clone()),
        email: None,
        ip: None,
        user_agent: None,
        country: None,
        locale: None,
        app_version: None,
        statsig_environment: Some(statsig_environment.clone()),
        private_attributes: Some(private_attributes.clone()),
        custom: Some(custom.clone()),
    });

    assert_eq!(user.data.custom_ids, Some(custom_ids));
    assert_eq!(user.data.custom, Some(custom));
    assert_eq!(user.data.private_attributes, Some(private_attributes));
    assert_eq!(user.data.statsig_environment, Some(statsig_environment));
}

#[test]
fn test_setting_string_fields() {
    let mut user = StatsigUser::with_user_id("".to_string());
    user.set_email("test@test.com");
    user.set_ip("127.0.0.1");
    user.set_user_agent("test");
    user.set_country("US");
    user.set_locale("en-US");
    user.set_app_version("1.0.0");

    assert_eq!(user.data.email, Some(dyn_value!("test@test.com")));
    assert_eq!(user.data.ip, Some(dyn_value!("127.0.0.1")));
    assert_eq!(user.data.user_agent, Some(dyn_value!("test")));
    assert_eq!(user.data.country, Some(dyn_value!("US")));
    assert_eq!(user.data.locale, Some(dyn_value!("en-US")));
}

#[test]
fn test_changing_string_fields() {
    let mut user = StatsigUser::with_user_id("".to_string());
    user.set_email("test@test.com");
    user.set_email(None::<String>);
    user.set_ip("127.0.0.1");
    user.set_ip("0.0.0.0");

    assert_eq!(user.data.email, None);
    assert_eq!(user.data.ip, Some(dyn_value!("0.0.0.0")));
}

#[test]
fn test_setting_attr_map_fields() {
    let custom = StatsigUserDataMap::from([("test_custom".to_string(), dyn_value!(1))]);
    let priv_attr = StatsigUserDataMap::from([("test_private".to_string(), dyn_value!(2))]);

    let mut user = StatsigUser::with_user_id("".to_string());

    user.set_custom(HashMap::from([("test_custom".to_string(), dyn_value!(1))]));
    assert_eq!(user.get_custom(), Some(&custom));

    user.set_private_attributes(HashMap::from([("test_private".to_string(), dyn_value!(2))]));
    assert_eq!(user.get_private_attributes(), Some(&priv_attr));
}

#[test]
fn test_setting_statsig_environment() {
    let mut user = StatsigUser::with_user_id("".to_string());
    user.set_statsig_environment(Some(HashMap::from([(
        "test_environment".to_string(),
        "test_value".to_string(),
    )])));
    assert_eq!(
        user.get_statsig_environment(),
        Some(HashMap::from([("test_environment", "test_value")]))
    );

    user.set_statsig_environment(None::<HashMap<String, String>>);
    assert_eq!(user.get_statsig_environment(), None);
}
