use sigstat::{SpecStore, SpecsSource, SpecsUpdate};
use std::fs;
fn create_test_spec_store() -> SpecStore {
    SpecStore::default()
}

#[test]
fn test_set_values_and_get_config_num_value() {
    let spec_store = create_test_spec_store();

    // Load JSON data from file
    let json_data: serde_json::Value = serde_json::from_str(
        &fs::read_to_string("tests/data/dcs_with_sdk_configs.json").expect("Unable to read file"),
    )
    .expect("Unable to parse JSON");

    let specs_update = SpecsUpdate {
        data: json_data.to_string(),
        source: SpecsSource::Network,
        received_at: 2000,
    };

    match spec_store.set_values(specs_update) {
        Ok(()) => println!("set_values succeeded"),
        Err(e) => println!("set_values failed: {e:?}"),
    }

    assert_eq!(
        spec_store
            .get_sdk_config_value("event_queue_size")
            .and_then(|v| v.float_value),
        Some(1800.0)
    );
    assert_eq!(
        spec_store
            .get_sdk_config_value("event_logging_interval_seconds")
            .and_then(|v| v.float_value),
        Some(1.0)
    );
    assert_eq!(
        spec_store
            .get_sdk_config_value("special_case_sampling_rate")
            .and_then(|v| v.float_value),
        Some(101.0)
    );
    assert_eq!(
        spec_store
            .get_sdk_config_value("non_existent_key")
            .and_then(|v| v.float_value),
        None
    );
}

#[test]
fn test_set_values_and_get_config_str_value() {
    let spec_store = create_test_spec_store();

    let json_data: serde_json::Value = serde_json::from_str(
        &fs::read_to_string("tests/data/dcs_with_sdk_configs.json").expect("Unable to read file"),
    )
    .expect("Unable to parse JSON");

    let specs_update = SpecsUpdate {
        data: json_data.to_string(),
        source: SpecsSource::Network,
        received_at: 2000,
    };

    spec_store.set_values(specs_update).unwrap();

    assert_eq!(
        spec_store
            .get_sdk_config_value("sampling_mode")
            .and_then(|v| v.string_value),
        Some("shadow".to_string())
    );
    assert_eq!(
        spec_store
            .get_sdk_config_value("non_existent_key")
            .and_then(|v| v.string_value),
        None
    );
}
