use statsig_rust::{
    global_configs::GlobalConfigs, networking::ResponseData, SpecStore, SpecsSource, SpecsUpdate,
    StatsigRuntime,
};
use std::fs;

fn create_test_spec_store(sdk_key: &str) -> SpecStore {
    SpecStore::new(
        sdk_key,
        sdk_key.to_string(),
        StatsigRuntime::get_runtime(),
        None,
    )
}

#[test]
fn test_set_values_and_get_config_num_value() {
    let sdk_key = "secret-key-set-global-configs-test";
    let spec_store = create_test_spec_store(sdk_key);
    let global_config_instance = GlobalConfigs::get_instance(sdk_key);
    // Load JSON data from file
    let data =
        fs::read_to_string("tests/data/dcs_with_sdk_configs.json").expect("Unable to read file");

    let specs_update = SpecsUpdate {
        data: ResponseData::from_bytes(data.into_bytes()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    };

    match spec_store.set_values(specs_update) {
        Ok(()) => println!("set_values succeeded"),
        Err(e) => println!("set_values failed: {e:?}"),
    }

    assert_eq!(
        global_config_instance.use_sdk_config_value("event_queue_size", |v| v.unwrap().float_value),
        Some(1800.0)
    );

    assert_eq!(
        global_config_instance
            .use_sdk_config_value("event_logging_interval_seconds", |v| v.unwrap().float_value),
        Some(1.0)
    );

    assert_eq!(
        global_config_instance
            .use_sdk_config_value("special_case_sampling_rate", |v| v.unwrap().float_value),
        Some(101.0)
    );

    assert_eq!(
        global_config_instance.use_sdk_config_value("non_existent_key", |v| v.cloned()),
        None
    );
}

#[test]
fn test_set_values_and_get_config_str_value() {
    let sdk_key = "secret-key-get-global-configs-test";
    let spec_store: SpecStore = create_test_spec_store(sdk_key);
    let global_config_instance = GlobalConfigs::get_instance(sdk_key);

    let data =
        fs::read_to_string("tests/data/dcs_with_sdk_configs.json").expect("Unable to read file");

    let specs_update = SpecsUpdate {
        data: ResponseData::from_bytes(data.into_bytes()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    };

    spec_store.set_values(specs_update).unwrap();

    assert_eq!(
        global_config_instance.use_sdk_config_value("sampling_mode", |v| v
            .unwrap()
            .string_value
            .clone()
            .unwrap()
            .value),
        "shadow".to_string()
    );
    assert_eq!(
        global_config_instance.use_sdk_config_value("non_existent_key", |v| v.cloned()),
        None
    );
}

#[test]
fn test_get_default_diagnostics_sampling_rate() {
    let global_config_instance = GlobalConfigs::get_instance("");
    let init_rate =
        global_config_instance.use_diagnostics_sampling_rate("initialize", |x| x.cloned());
    let config_sync_rate =
        global_config_instance.use_diagnostics_sampling_rate("config_sync", |x| x.cloned());
    let dcs_rate = global_config_instance.use_diagnostics_sampling_rate("dcs", |x| x.cloned());
    let get_id_list_rate =
        global_config_instance.use_diagnostics_sampling_rate("get_id_list", |x| x.cloned());

    assert_eq!(init_rate, Some(10000.0));
    assert_eq!(config_sync_rate, Some(1000.0));
    assert_eq!(dcs_rate, Some(1000.0));
    assert_eq!(get_id_list_rate, Some(100.0));
}

#[test]
fn test_set_and_get_sampling_rate() {
    let sdk_key = "secret-key-sampling-global-configs-test";
    let spec_store: SpecStore = create_test_spec_store(sdk_key);
    let data =
        fs::read_to_string("tests/data/dcs_with_sdk_configs.json").expect("Unable to read file");

    let specs_update = SpecsUpdate {
        data: ResponseData::from_bytes(data.into_bytes()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    };

    match spec_store.set_values(specs_update) {
        Ok(()) => println!("set_values succeeded"),
        Err(e) => println!("set_values failed: {e:?}"),
    }

    let global_config_instance = GlobalConfigs::get_instance(sdk_key);
    let init_rate =
        global_config_instance.use_diagnostics_sampling_rate("initialize", |x| x.cloned());
    let config_sync_rate =
        global_config_instance.use_diagnostics_sampling_rate("config_sync", |x| x.cloned());
    let dcs_rate = global_config_instance.use_diagnostics_sampling_rate("dcs", |x| x.cloned());
    let get_id_list_rate =
        global_config_instance.use_diagnostics_sampling_rate("get_id_list", |x| x.cloned());

    assert_eq!(init_rate, Some(9999.0));
    assert_eq!(config_sync_rate, Some(1000.0));
    assert_eq!(dcs_rate, Some(999.0));
    assert_eq!(get_id_list_rate, Some(99.0));
}

#[test]
fn test_set_and_get_sdk_flag() {
    let sdk_key = "secret-key-sampling-global-configs-test";
    let spec_store: SpecStore = create_test_spec_store(sdk_key);
    let data =
        fs::read_to_string("tests/data/dcs_with_sdk_configs.json").expect("Unable to read file");
    let specs_update = SpecsUpdate {
        data: ResponseData::from_bytes(data.into_bytes()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    };

    match spec_store.set_values(specs_update) {
        Ok(()) => println!("set_values succeeded"),
        Err(e) => println!("set_values failed: {e:?}"),
    }

    let global_config_instance = GlobalConfigs::get_instance(sdk_key);
    let disable_console_capture =
        global_config_instance.get_sdk_flag_value("disable_console_capture");

    assert!(disable_console_capture);
}

#[test]
fn test_set_and_get_sdk_flag_when_dcs_has_no_sdk_flags() {
    let sdk_key = "secret-key-sampling-global-configs-test";
    let spec_store: SpecStore = create_test_spec_store(sdk_key);
    let data = fs::read_to_string("tests/data/eval_proj_dcs.json").expect("Unable to read file");

    let specs_update = SpecsUpdate {
        data: ResponseData::from_bytes(data.into_bytes()),
        source: SpecsSource::Network,
        received_at: 2000,
        source_api: None,
    };

    match spec_store.set_values(specs_update) {
        Ok(()) => println!("set_values succeeded"),
        Err(e) => println!("set_values failed: {e:?}"),
    }

    let global_config_instance = GlobalConfigs::get_instance(sdk_key);
    let disable_console_capture =
        global_config_instance.get_sdk_flag_value("disable_console_capture");

    assert!(!disable_console_capture);
}
