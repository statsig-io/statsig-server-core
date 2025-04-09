use std::{collections::HashMap, env, sync::Arc};

use statsig_rust::{
    dyn_value, evaluation::evaluation_types::AnyConfigEvaluation, hashing::djb2,
    output_logger::LogLevel, Statsig, StatsigHttpIdListsAdapter, StatsigOptions, StatsigUser,
};

fn get_sdk_key() -> String {
    let key = env::var("test_api_key").expect("test_api_key environment variable not set");
    assert!(key.starts_with("secret-9IWf"));
    key
}

#[tokio::test]
async fn test_check_gate() {
    let user = StatsigUser {
        email: Some(dyn_value!("daniel@statsig.com")),
        ..StatsigUser::with_user_id("a-user".to_string())
    };

    let statsig = Statsig::new(&get_sdk_key(), None);
    statsig.initialize().await.unwrap();

    let gate_result = statsig.check_gate(&user, "test_50_50");

    assert!(gate_result);
}

#[tokio::test]
async fn test_check_gate_id_list() {
    let user = StatsigUser {
        custom_ids: Some(HashMap::from([(
            "companyID".to_string(),
            dyn_value!("marcos_1"),
        )])),
        ..StatsigUser::with_user_id("marcos_1".to_string())
    };

    let mut opts = StatsigOptions::new();

    let adapter = Arc::new(StatsigHttpIdListsAdapter::new(&get_sdk_key(), &opts));
    opts.id_lists_adapter = Some(adapter);

    let statsig = Statsig::new(&get_sdk_key(), Some(Arc::new(opts)));
    statsig.initialize().await.unwrap();

    let gate_result = statsig.check_gate(&user, "test_id_list");

    assert!(gate_result);
}

#[tokio::test]
async fn test_get_experiment() {
    let user = StatsigUser {
        email: Some(dyn_value!("daniel@statsig.com")),
        ..StatsigUser::with_user_id("a-user".to_string())
    };

    let statsig = Statsig::new(&get_sdk_key(), None);
    statsig.initialize().await.unwrap();

    let experiment = statsig.get_experiment(&user, "running_exp_in_unlayered_with_holdout");
    let _ = statsig.shutdown().await;

    assert_ne!(experiment.value.len(), 0);
}

#[tokio::test]
async fn test_gcir() {
    let user = StatsigUser {
        email: Some(dyn_value!("daniel@statsig.com")),
        ..StatsigUser::with_user_id("a-user".to_string())
    };
    let opts = StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        ..StatsigOptions::new()
    };

    let statsig = Statsig::new(&get_sdk_key(), Some(Arc::new(opts)));
    statsig.initialize().await.unwrap();

    let response = statsig.get_client_init_response(&user);
    let _ = statsig.shutdown().await;

    let gates = response.feature_gates;
    assert_eq!(gates.len(), 69);

    let configs = response.dynamic_configs.len();
    assert_eq!(configs, 62);

    let a_config_opt = response.dynamic_configs.get(&djb2("big_number"));
    let a_config = match a_config_opt {
        Some(v) => match v {
            AnyConfigEvaluation::DynamicConfig(config) => &config.value,
            AnyConfigEvaluation::Experiment(exp) => &exp.value,
        },
        None => panic!("Should have values"),
    };

    assert!(!a_config.is_empty());
}

// Todo: rewrite this test such that it isn't reaching into internal implementation details
// #[tokio::test]
// async fn do_not_double_start_background_tasks() {
//     let statsig_rt = StatsigRuntime::get_runtime();
//     let adapter = Arc::new(MockAdapter::new());
//     let logger = Arc::new(EventLogger::new(
//         "secret-key",
//         adapter.clone(),
//         &StatsigOptions::new(),
//         &statsig_rt,
//     ));
//     let specs_adapter = Arc::new(MockSpecsAdapter::new());
//     let ops_stats = OPS_STATS.get_for_instance("secret-key");
//     let background_tasks_started = Arc::new(AtomicBool::new(false));

//     let success = Statsig::start_background_tasks(
//         logger.clone(),
//         statsig_rt.clone(),
//         None,
//         specs_adapter.clone(),
//         ops_stats.clone(),
//         background_tasks_started.clone(),
//     )
//     .await;

//     assert!(success);

//     Statsig::start_background_tasks(
//         logger.clone(),
//         statsig_rt.clone(),
//         None,
//         specs_adapter.clone(),
//         ops_stats.clone(),
//         background_tasks_started.clone(),
//     )
//     .await;

//     assert!(specs_adapter.get_schedule_call_count() == 1);
// }
