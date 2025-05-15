use statsig_rust::{dyn_value, StatsigUser};
use std::collections::HashMap;

#[test]
fn test_user_id_accessor() {
    let mut user = StatsigUser::with_user_id("user1".to_string());
    assert_eq!(user.get_user_id(), Some("user1"));

    user.set_user_id("user2");
    assert_eq!(user.get_user_id(), Some("user2"));

    user.set_user_id(123);
    assert_eq!(user.get_user_id(), Some("123"));
}

#[test]
fn test_custom_ids_accessor() {
    let mut user = StatsigUser::with_user_id("");
    assert_eq!(user.get_custom_ids(), None);

    user.set_custom_ids(HashMap::from([("employeeID", "num1")]));
    assert_eq!(
        user.get_custom_ids(),
        Some(HashMap::from([("employeeID", "num1")]))
    );

    user.set_custom_ids(HashMap::from([("employeeID", 1)]));
    assert_eq!(
        user.get_custom_ids(),
        Some(HashMap::from([("employeeID", "1")]))
    );
}

#[test]
fn test_email_accessor() {}

macro_rules! test_string_field_accessor {
    ($test_name:ident, $getter_name:ident, $setter_name:ident) => {
        #[test]
        fn $test_name() {
            let mut user = StatsigUser::with_user_id("");
            assert_eq!(user.$getter_name(), None);

            // Option<String>
            user.$setter_name(Some("some_string".to_string()));
            assert_eq!(user.$getter_name(), Some("some_string"));

            // String
            user.$setter_name("a_string".to_string());
            assert_eq!(user.$getter_name(), Some("a_string"));

            // &String
            let my_email = "a_string_ref".to_string();
            user.$setter_name(&my_email);
            assert_eq!(user.$getter_name(), Some("a_string_ref"));

            // &str
            user.$setter_name("str_literal");
            assert_eq!(user.$getter_name(), Some("str_literal"));

            // None
            user.$setter_name(None::<String>);
            assert_eq!(user.$getter_name(), None);
        }
    };
}

test_string_field_accessor!(test_email_string_accessor, get_email, set_email);
test_string_field_accessor!(test_ip_string_accessor, get_ip, set_ip);
test_string_field_accessor!(test_country_string_accessor, get_country, set_country);
test_string_field_accessor!(test_locale_string_accessor, get_locale, set_locale);
test_string_field_accessor!(
    test_app_version_string_accessor,
    get_app_version,
    set_app_version
);
test_string_field_accessor!(test_user_agent_accessor, get_user_agent, set_user_agent);

macro_rules! test_map_field_accessor {
    ($test_name:ident, $getter_name:ident, $setter_name:ident) => {
        #[test]
        fn $test_name() {
            let mut user = StatsigUser::with_user_id("");
            assert_eq!(user.$getter_name(), None);

            // HashMap<String, bool>
            user.$setter_name(HashMap::from([("isAdmin", true)]));
            assert_eq!(
                user.$getter_name(),
                Some(&HashMap::from([("isAdmin".to_string(), dyn_value!(true))]))
            );

            // Option<HashMap<String, bool>>
            user.$setter_name(Some(HashMap::from([("isAdmin", true)])));
            assert_eq!(
                user.$getter_name(),
                Some(&HashMap::from([("isAdmin".to_string(), dyn_value!(true))]))
            );

            // HashMap<String, DynamicValue>
            let multi_custom = HashMap::from([
                ("isAdmin".to_string(), dyn_value!(true)),
                ("powerLevel".to_string(), dyn_value!(10)),
            ]);
            user.$setter_name(Some(multi_custom.clone()));
            assert_eq!(user.$getter_name(), Some(&multi_custom));

            // None
            user.$setter_name(None::<HashMap<String, bool>>);
            assert_eq!(user.$getter_name(), None);
        }
    };
}

test_map_field_accessor!(test_custom_map_accessor, get_custom, set_custom);
test_map_field_accessor!(
    test_private_attributes_map_accessor,
    get_private_attributes,
    set_private_attributes
);
