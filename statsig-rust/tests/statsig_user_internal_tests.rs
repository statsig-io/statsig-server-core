use statsig_rust::{
    evaluation::dynamic_string::DynamicString, user::StatsigUserInternal, DynamicValue, StatsigUser,
};

#[test]
fn test_get_user_value() {
    let mut user = StatsigUser::with_user_id("user1");
    user.set_email("user1@example.com");
    let user_internal = StatsigUserInternal::new(&user, None);
    let field = DynamicString::from("email".to_string());
    let user_value = user_internal.get_user_value(&Some(field));
    assert_eq!(user_value, Some(&DynamicValue::from("user1@example.com")));
}

// todo: test get_unit_id
// todo: test get_value_from_environment
// todo: test to_loggable
