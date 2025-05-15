use statsig_rust::{dyn_value, evaluation::dynamic_string::DynamicString, StatsigUserBuilder};
use std::collections::HashMap;

macro_rules! map {
    ($($key:expr => $value:expr),+ $(,)?) => {{
        let mut m = ::std::collections::HashMap::new();
        $(
            m.insert($key, $value);
        )+
        m
    }};
}

#[test]
fn test_user_creation_with_user_id_strings() {
    let my_string: String = "user1".to_string();
    let my_str: &str = "user1";

    let from_string_ref = StatsigUserBuilder::new_with_user_id(&my_string).build();
    assert_eq!(from_string_ref.data.user_id, Some(dyn_value!("user1")));

    let from_string = StatsigUserBuilder::new_with_user_id(my_string).build();
    assert_eq!(from_string.data.user_id, Some(dyn_value!("user1")));

    let from_str = StatsigUserBuilder::new_with_user_id(my_str).build();
    assert_eq!(from_str.data.user_id, Some(dyn_value!("user1")));
}

#[test]
fn test_user_creation_with_user_id_numbers() {
    let my_i64: i64 = 1234567890;
    let my_f64: f64 = 1234567890.1;

    let from_i64 = StatsigUserBuilder::new_with_user_id(my_i64).build();
    assert_eq!(
        from_i64
            .data
            .user_id
            .as_ref()
            .and_then(|u| u.string_value.clone()),
        Some(DynamicString::from("1234567890".to_string()))
    );

    let from_f64 = StatsigUserBuilder::new_with_user_id(my_f64).build();
    assert_eq!(
        from_f64
            .data
            .user_id
            .as_ref()
            .and_then(|d| d.string_value.clone()),
        Some(DynamicString::from("1234567890.1".to_string()))
    );
}

#[test]
fn test_user_creation_with_custom_ids_strings() {
    let my_string: String = "test".to_string();

    let my_string_ref_map: HashMap<&String, &String> = map!(&my_string => &my_string);
    let with_string_ref = StatsigUserBuilder::new_with_custom_ids(my_string_ref_map).build();
    assert_eq!(
        with_string_ref
            .data
            .custom_ids
            .as_ref()
            .unwrap()
            .get("test"),
        Some(&dyn_value!("test"))
    );

    let my_string_map: HashMap<String, String> = map!(my_string.clone() => my_string);
    let with_string = StatsigUserBuilder::new_with_custom_ids(my_string_map).build();
    assert_eq!(
        with_string.data.custom_ids.as_ref().unwrap().get("test"),
        Some(&dyn_value!("test"))
    );

    let my_str_map: HashMap<&str, &str> = map!("test" => "test");
    let with_str = StatsigUserBuilder::new_with_custom_ids(my_str_map).build();
    assert_eq!(
        with_str.data.custom_ids.as_ref().unwrap().get("test"),
        Some(&dyn_value!("test"))
    );
}

#[test]
fn test_user_creation_with_custom_ids_numbers() {
    let my_i64_map: HashMap<String, i64> = map!("test".to_string() => 1234567890);
    let with_i64 = StatsigUserBuilder::new_with_custom_ids(my_i64_map).build();
    assert_eq!(
        with_i64.data.custom_ids.as_ref().unwrap().get("test"),
        Some(&dyn_value!(1234567890))
    );

    let my_f64_map: HashMap<String, f64> = map!("test".to_string() => 1234567890.1);
    let with_f64 = StatsigUserBuilder::new_with_custom_ids(my_f64_map).build();
    assert_eq!(
        with_f64.data.custom_ids.as_ref().unwrap().get("test"),
        Some(&dyn_value!(1234567890.1))
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

    assert_eq!(user.data.email, Some(dyn_value!("test@test.com")));
    assert_eq!(user.data.ip, Some(dyn_value!("127.0.0.1")));
    assert_eq!(user.data.user_agent, Some(dyn_value!("test")));
    assert_eq!(user.data.country, Some(dyn_value!("US")));
    assert_eq!(user.data.locale, Some(dyn_value!("en_US")));
    assert_eq!(user.data.app_version, Some(dyn_value!("1.0.0")));
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
    assert_eq!(user.data.email, Some(dyn_value!("test@test.com")));

    assert_eq!(user.data.ip, Some(dyn_value!("0.0.0.0")));
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
        user.data.custom,
        Some(HashMap::from([(
            "test_custom_again".to_string(),
            dyn_value!("a"),
        )]))
    );

    // todo: Should we set the value to None
    // assert_eq!(user.private_attributes, None);
    assert_eq!(
        user.data.private_attributes,
        Some(HashMap::from([
            ("test_private".to_string(), dyn_value!(2),)
        ]))
    );
}
