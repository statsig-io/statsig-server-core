use serde_json::json;
use statsig_rust::{StatsigUserBuilder, StatsigUserData};
use std::collections::{HashMap, HashSet};

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

#[test]
fn test_id_list_memberships_are_skipped_in_both_serde_directions() {
    let memberships = HashMap::from([(
        "lookup".to_string(),
        HashSet::from(["employees".to_string()]),
    )]);
    let user = StatsigUserBuilder::new_with_user_id("a_user_id")
        .id_list_memberships(memberships)
        .build();

    let serialized = serde_json::to_value(user.data.as_ref()).unwrap();
    assert!(serialized.get("idListMemberships").is_none());
    assert!(serialized.get("id_list_memberships").is_none());

    let deserialized: StatsigUserData = serde_json::from_value(json!({
        "userID": "a_user_id",
        "idListMemberships": {"lookup": ["employees"]},
        "id_list_memberships": {"lookup": ["employees"]}
    }))
    .unwrap();
    assert!(deserialized.id_list_memberships.is_none());
}
