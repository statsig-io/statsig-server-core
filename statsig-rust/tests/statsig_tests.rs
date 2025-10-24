mod utils;
use more_asserts::assert_ge;
use std::{collections::HashMap, env, sync::Arc, thread::sleep, time::Duration};
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use utils::mock_specs_adapter::MockSpecsAdapter;

use statsig_rust::{
    evaluation::evaluation_types::AnyConfigEvaluation, hashing::djb2, output_logger::LogLevel,
    SpecsSource, Statsig, StatsigHttpIdListsAdapter, StatsigOptions, StatsigUser,
    StatsigUserBuilder,
};

fn get_sdk_key() -> String {
    let key = env::var("test_api_key").expect("test_api_key environment variable not set");
    assert!(key.starts_with("secret-9IWf"));
    key
}

#[tokio::test]
async fn test_check_gate() {
    let user = StatsigUserBuilder::new_with_user_id("a-user".to_string())
        .email(Some("daniel@statsig.com".to_string()))
        .build();

    let statsig = Statsig::new(&get_sdk_key(), None);
    statsig.initialize().await.unwrap();

    let gate_result = statsig.check_gate(&user, "test_50_50");

    assert!(gate_result);
}

#[tokio::test]
async fn test_check_gate_id_list() {
    let user = StatsigUserBuilder::new_with_user_id("marcos_1".to_string())
        .custom_ids(Some(HashMap::from([(
            "companyID".to_string(),
            "marcos_1".to_string(),
        )])))
        .build();

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
    let user = StatsigUserBuilder::new_with_user_id("a-user".to_string())
        .email(Some("daniel@statsig.com".to_string()))
        .build();

    let statsig = Statsig::new(&get_sdk_key(), None);
    statsig.initialize().await.unwrap();

    let experiment = statsig.get_experiment(&user, "running_exp_in_unlayered_with_holdout");
    let _ = statsig.shutdown().await;

    assert_ne!(experiment.value.len(), 0);
}

#[tokio::test]
async fn test_get_experiment_by_group_name() {
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            ..StatsigOptions::new()
        })),
    );
    statsig.initialize().await.unwrap();

    let experiment_name = "test_experiment_no_targeting";
    let group_name = "Control";
    let experiment = statsig.get_experiment_by_group_name(experiment_name, group_name);

    assert_eq!(experiment.name, experiment_name);
    assert_eq!(experiment.group_name.as_deref(), Some(group_name));
    assert_eq!(experiment.rule_id, "54QJztEPRLXK7ZCvXeY9q4");
    assert_eq!(experiment.id_type, "userID");
    assert_eq!(experiment.value["value"], "control");
}

#[tokio::test]
async fn test_gcir() {
    let user = StatsigUserBuilder::new_with_user_id("a-user".to_string())
        .email(Some("daniel@statsig.com".to_string()))
        .build();
    let opts = StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        ..StatsigOptions::new()
    };

    let statsig = Statsig::new(&get_sdk_key(), Some(Arc::new(opts)));
    statsig.initialize().await.unwrap();

    let response = statsig.get_client_init_response(&user);
    let _ = statsig.shutdown().await;

    let gates = response.feature_gates;
    assert_ge!(gates.len(), 69);

    let configs = response.dynamic_configs.len();
    assert_ge!(configs, 62);

    let a_config_opt = response.dynamic_configs.get(&djb2("big_number"));
    let a_config = match a_config_opt {
        Some(v) => match v {
            AnyConfigEvaluation::DynamicConfig(config) => &config.value,
            AnyConfigEvaluation::Experiment(exp) => &exp.value,
        },
        None => panic!("Should have values"),
    };

    assert!(!a_config.get_json().unwrap().is_empty());
}

#[tokio::test]
async fn test_user_agent_and_country_lookup() {
    // Default behavior
    let user = StatsigUserBuilder::new_with_user_id("a-user".to_string())
        .email(Some("daniel@statsig.com".to_string()))
        .user_agent(Some(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1".into(),
        ))
        .build();
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
    let user = StatsigUserBuilder::new_with_user_id("a-user".to_string())
        .email(Some("daniel@statsig.com".to_string()))
        .user_agent(Some(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1".into(),
        ))
        .build();
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
async fn test_get_running_task_ids() {
    let statsig = Statsig::new(
        &get_sdk_key(),
        Some(Arc::new(StatsigOptions {
            enable_id_lists: Some(true),
            ..StatsigOptions::new()
        })),
    );

    fn get_task_ids_str(task_ids: &mut [(String, String)]) -> String {
        task_ids.sort();
        task_ids
            .iter()
            .map(|a| a.0.clone())
            .collect::<Vec<String>>()
            .join(" | ")
    }

    let mut task_ids_before = statsig.statsig_runtime.get_running_task_ids();
    let task_ids_before_str = get_task_ids_str(&mut task_ids_before);

    assert_eq!(
        task_ids_before_str,
        "EVT_LOG_BG_LOOP | opts_stats_listen_for | opts_stats_listen_for" // the event logger flush job and two subscription to OpsStats
    );

    statsig.initialize().await.unwrap();

    let mut task_ids_after = statsig.statsig_runtime.get_running_task_ids();
    let task_ids_after_str = get_task_ids_str(&mut task_ids_after);
    assert_eq!(
        task_ids_after_str,
        // before tasks + id list and specs bg sync jobs
        "EVT_LOG_BG_LOOP | http_id_list_bg_sync | http_specs_bg_sync | opts_stats_listen_for | opts_stats_listen_for"
    );
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
