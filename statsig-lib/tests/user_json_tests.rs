use serde_json::json;
use sigstat::StatsigUser;
use std::collections::HashMap;

#[tokio::test]
async fn test_field_names() {
    let user = StatsigUser {
        custom_ids: Some(HashMap::from([("podId".into(), "my_pod".into())])),
        ..StatsigUser::with_user_id("a_user_id".into())
    };

    let result = json!(user).as_object().unwrap().clone();

    assert!(result.contains_key("customIDs"));
    assert!(result.contains_key("userID"));
    assert!(!result.contains_key("email"));
}
