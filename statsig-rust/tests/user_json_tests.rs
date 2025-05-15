use serde_json::json;
use statsig_rust::StatsigUserBuilder;
use std::collections::HashMap;

#[tokio::test]
async fn test_field_names() {
    let user = StatsigUserBuilder::new_with_custom_ids(HashMap::from([(
        "podId".to_string(),
        "my_pod".to_string(),
    )]))
    .user_id(Some("a_user_id".to_string()))
    .build();

    let result = json!(user.data.as_ref()).as_object().unwrap().clone();

    assert!(result.contains_key("customIDs"));
    assert!(result.contains_key("userID"));
    assert!(!result.contains_key("email"));
}
