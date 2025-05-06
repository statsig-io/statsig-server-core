mod utils;
use std::{collections::HashMap, env, sync::Arc, thread::sleep, time::Duration};
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use utils::mock_specs_adapter::MockSpecsAdapter;

use statsig_rust::{
    dyn_value, evaluation::evaluation_types::AnyConfigEvaluation, hashing::djb2,
    output_logger::LogLevel, DynamicValue, SpecsSource, Statsig, StatsigHttpIdListsAdapter,
    StatsigOptions, StatsigUser,
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

#[tokio::test]
async fn test_user_agent_and_country_lookup() {
    // Default behavior
    let user = StatsigUser {
        email: Some(dyn_value!("daniel@statsig.com")),
        user_agent: Some(DynamicValue::from("Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1")),
        ..StatsigUser::with_user_id("a-user".to_string())
    };
    let opts = StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        ..StatsigOptions::new()
    };

    let statsig = Statsig::new(&get_sdk_key(), Some(Arc::new(opts)));
    statsig.initialize().await.unwrap();
    // Avg it takes 2 seconds
    sleep(Duration::from_secs(2));
    assert!(statsig.check_gate(&user, "test_ua"));

    // Wait for ua and ip to initialize
    let user = StatsigUser {
            email: Some(dyn_value!("daniel@statsig.com")),
            user_agent: Some(DynamicValue::from("Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1")),
            ..StatsigUser::with_user_id("a-user".to_string())
        };
    let opts = StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        wait_for_country_lookup_init: Some(true),
        wait_for_user_agent_init: Some(true),
        ..StatsigOptions::new()
    };

    let statsig_2 = Statsig::new(&get_sdk_key(), Some(Arc::new(opts)));
    statsig_2.initialize().await.unwrap();
    assert!(statsig.check_gate(&user, "test_ua"));
}

#[tokio::test]
async fn test_user_agent_disabled() {
    // Properly disable
    let user = StatsigUser {
        email: Some(dyn_value!("daniel@statsig.com")),
        user_agent: Some(DynamicValue::from("Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1")),
        ..StatsigUser::with_user_id("a-user".to_string())
    };
    let opts_3 = StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        wait_for_country_lookup_init: Some(true),
        wait_for_user_agent_init: Some(true),
        disable_user_agent_parsing: Some(true),
        disable_country_lookup: Some(true),
        ..StatsigOptions::new()
    };

    let statsig_3 = Statsig::new(&get_sdk_key(), Some(Arc::new(opts_3)));
    statsig_3.initialize().await.unwrap();
    assert!(!statsig_3.check_gate(&user, "test_ua"));
}

#[tokio::test]
async fn test_initialize_with_details() {
    let statsig = Statsig::new(&get_sdk_key(), None);
    let details = statsig.initialize_with_details().await.unwrap();
    assert!(details.init_success);
    assert!(details.is_config_spec_ready);
    assert!(details.source == SpecsSource::Network);
    assert!(details.failure_details.is_none());
    assert!(details.is_id_list_ready.is_none());
}

#[tokio::test]
async fn test_initialize_with_details_failure() {
    let statsig = Statsig::new("invalid-sdk-key", None);
    let details = statsig.initialize_with_details().await.unwrap();
    assert!(details.init_success);
    assert!(!details.is_config_spec_ready);
    assert!(details.source == SpecsSource::NoValues);
    assert!(details.failure_details.is_some());
    assert!(details.is_id_list_ready.is_none());
}

#[tokio::test]
async fn test_initialize_with_details_with_id_lists() {
    let id_list_opt = StatsigOptions {
        enable_id_lists: Some(true),
        ..StatsigOptions::new()
    };
    let statsig = Statsig::new(&get_sdk_key(), Some(Arc::new(id_list_opt)));
    let details = statsig.initialize_with_details().await.unwrap();
    assert!(details.init_success);
    assert!(details.is_config_spec_ready);
    assert!(details.source == SpecsSource::Network);
    assert!(details.failure_details.is_none());
    assert!(details.is_id_list_ready.is_some());
}

#[tokio::test]
async fn test_identify() {
    let mock_event_logger = Arc::new(MockEventLoggingAdapter::new());
    let opts = StatsigOptions {
        event_logging_adapter: Some(mock_event_logger.clone()),
        specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
            "tests/data/eval_proj_dcs.json",
        ))),
        ..StatsigOptions::default()
    };

    let statsig = Statsig::new(&get_sdk_key(), Some(Arc::new(opts)));
    statsig.initialize().await.unwrap();

    let user = StatsigUser::with_user_id("test-user-for-identify".to_string());
    statsig.identify(&user);

    sleep(Duration::from_millis(1));
    statsig.flush_events().await;

    let first_event = mock_event_logger.force_get_first_event();
    assert_eq!(first_event.get("eventName").unwrap(), "statsig::identify");
    assert!(first_event.get("value").unwrap().is_null());
    assert!(first_event.get("metadata").unwrap().is_null());

    let user_obj = first_event.get("user").unwrap();
    assert_eq!(user_obj.get("userID").unwrap(), "test-user-for-identify");
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
