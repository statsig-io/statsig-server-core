use statsig_rust::statsig_types::Layer;

#[test]
fn test_layer_serialization() {
    let raw_value = r#"{
        "__disable_exposure": false,
        "__evaluation": {
            "explicit_parameters": [],
            "group": "override",
            "id_type": "",
            "is_device_based": false,
            "name": "test_layer",
            "rule_id": "override",
            "secondary_exposures": [],
            "undelegated_secondary_exposures": [],
            "value": { "foo": "bar" }
        },
        "__user": { "userID": "a-user" },
        "__value": { "foo": "bar" },
        "__version": null,
        "allocated_experiment_name": null,
        "details": {
            "lcut": 1729873603830,
            "reason": "LocalOverride:Recognized",
            "received_at": 1750806236320
        },
        "group_name": null,
        "id_type": "",
        "name": "test_layer",
        "rule_id": "override"
    }"#;

    let result = serde_json::from_str::<Layer>(raw_value);
    assert_eq!(result.as_ref().err().map(|x| x.to_string()), None);
    assert_eq!(result.ok().map(|x| x.name), Some("test_layer".to_string()));
}
