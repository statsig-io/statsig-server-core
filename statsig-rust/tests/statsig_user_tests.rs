use statsig_rust::{dyn_value, StatsigUser};
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
        Some(HashMap::from([(
            "companyID".to_string(),
            dyn_value!("statsig")
        )]))
    );
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
    let custom = HashMap::from([("test_custom".to_string(), dyn_value!(1))]);
    let priv_attr = HashMap::from([("test_private".to_string(), dyn_value!(2))]);

    let mut user = StatsigUser::with_user_id("".to_string());

    user.set_custom(custom.clone());
    assert_eq!(user.get_custom(), Some(&custom));

    user.set_private_attributes(priv_attr.clone());
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
