use statsig_rust::{dyn_value, StatsigUserBuilder};
use std::collections::HashMap;

#[test]
fn test_user_creation_with_user_id() {
    let user = StatsigUserBuilder::new_with_user_id("user1".to_string()).build();

    assert_eq!(user.user_id, Some(dyn_value!("user1")));
}

#[test]
fn test_user_creation_with_custom_ids() {
    let user = StatsigUserBuilder::new_with_custom_ids(HashMap::from([(
        "test".to_string(),
        "test".to_string(),
    )]))
    .build();

    assert_eq!(
        user.custom_ids.unwrap_or_default().get("test"),
        Some(&dyn_value!("test"))
    );
}

#[test]
fn test_setting_string_fields() {
    let user = StatsigUserBuilder::new_with_user_id("".to_string())
        .email(Some("test@test.com".to_string()))
        .ip(Some("127.0.0.1".to_string()))
        .user_agent(Some("test".to_string()))
        .country(Some("US".to_string()))
        .locale(Some("en_US".to_string()))
        .app_version(Some("1.0.0".to_string()))
        .build();

    assert_eq!(user.email, Some(dyn_value!("test@test.com")));
    assert_eq!(user.ip, Some(dyn_value!("127.0.0.1")));
    assert_eq!(user.user_agent, Some(dyn_value!("test")));
    assert_eq!(user.country, Some(dyn_value!("US")));
    assert_eq!(user.locale, Some(dyn_value!("en_US")));
    assert_eq!(user.app_version, Some(dyn_value!("1.0.0")));
}

#[test]
fn test_changing_string_fields() {
    let user = StatsigUserBuilder::new_with_user_id("".to_string())
        .email(Some("test@test.com".to_string()))
        .email(None)
        .ip(Some("127.0.0.1".to_string()))
        .ip(Some("0.0.0.0".to_string()))
        .build();

    // todo: Should we set the value to None
    // assert_eq!(user.email, None);
    assert_eq!(user.email, Some(dyn_value!("test@test.com")));

    assert_eq!(user.ip, Some(dyn_value!("0.0.0.0")));
}

#[test]
fn test_setting_attr_map_fields() {
    let user = StatsigUserBuilder::new_with_user_id("".to_string())
        .custom(Some(HashMap::from([(
            "test_custom".to_string(),
            dyn_value!(1),
        )])))
        .custom(Some(HashMap::from([(
            "test_custom_again".to_string(),
            dyn_value!("a"),
        )])))
        .private_attributes(Some(HashMap::from([(
            "test_private".to_string(),
            dyn_value!(2),
        )])))
        .private_attributes(None)
        .build();

    assert_eq!(
        user.custom,
        Some(HashMap::from([(
            "test_custom_again".to_string(),
            dyn_value!("a"),
        )]))
    );

    // todo: Should we set the value to None
    // assert_eq!(user.private_attributes, None);
    assert_eq!(
        user.private_attributes,
        Some(HashMap::from([
            ("test_private".to_string(), dyn_value!(2),)
        ]))
    );
}
