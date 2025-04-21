use std::env;

use statsig_rust::Statsig;

fn get_sdk_key() -> String {
    let key = env::var("test_api_key").expect("test_api_key environment variable not set");
    assert!(key.starts_with("secret-9IWf"));
    key
}

#[tokio::test]
async fn test_get_feature_gate_list() {
    let statsig = Statsig::new(&get_sdk_key(), None);
    statsig.initialize().await.unwrap();

    let gate_list = statsig.get_feature_gate_list();

    assert!(!gate_list.is_empty());
    assert!(gate_list.contains(&"test_50_50".to_string()));
}

#[tokio::test]
async fn test_get_dynamic_config_list() {
    let statsig = Statsig::new(&get_sdk_key(), None);
    statsig.initialize().await.unwrap();

    let config_list = statsig.get_dynamic_config_list();

    assert!(!config_list.is_empty());
}

#[tokio::test]
async fn test_get_experiment_list() {
    let statsig = Statsig::new(&get_sdk_key(), None);
    statsig.initialize().await.unwrap();

    let experiment_list = statsig.get_experiment_list();

    assert!(!experiment_list.is_empty());
    assert!(experiment_list.contains(&"running_exp_in_unlayered_with_holdout".to_string()));
}

#[tokio::test]
async fn test_get_parameter_store_list() {
    let statsig = Statsig::new(&get_sdk_key(), None);
    statsig.initialize().await.unwrap();

    let parameter_store_list = statsig.get_parameter_store_list();

    assert!(
        !parameter_store_list.is_empty(),
        "Parameter store list should not be empty"
    );
}
