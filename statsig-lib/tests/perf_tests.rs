use std::cmp::min;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use serde_json::json;
use statsig::{InitializeResponse, Statsig, StatsigOptions, StatsigUser};
use crate::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::mock_specs_adapter::MockSpecsAdapter;

mod mock_specs_adapter;
mod mock_event_logging_adapter;

async fn setup() -> (StatsigUser, Statsig) {
    let custom_ids: HashMap<String, String> = HashMap::from([
        ("companyID".into(), "an_employee".into())
    ]);

    let user = StatsigUser {
        user_id: Some("user-d".into()),
        country: Some("GB".into()),
        ..StatsigUser::with_custom_ids(custom_ids)
    };

    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(Arc::new(MockSpecsAdapter::with_data("tests/eval_proj_dcs.json")));
    options.event_logging_adapter = Some(Arc::new(MockEventLoggingAdapter::new()));
    options.environment = Some("development".to_string());

    let start = Instant::now();
    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();
    let duration = start.elapsed();
    println!("Init Duration: {}", duration.as_secs_f64() * 1000.0);

    (user, statsig)
}

#[tokio::test]
async fn test_individual_gate_checks() {
    let (user, statsig) = setup().await;

    let gate_name = "test_many_rules";
    let start = Instant::now();

    let mut result = false;
    for _ in 0..1000 {
        result = statsig.check_gate(&user, gate_name);
    }

    let duration = start.elapsed();
    println!("Result {}", result);
    println!("Duration {}", duration.as_secs_f64() * 1000.0);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_all_gate_checks() {
    let (user, statsig) = setup().await;

    let mut times: HashMap<String, (bool, f64)> = HashMap::new();
    let values = statsig.get_current_values().unwrap().values.clone();

    let all_start = Instant::now();
    for (gate_name, _) in values.feature_gates.iter() {
        let start = Instant::now();

        let mut result = false;
        for _ in 0..1000 {
            result = statsig.check_gate(&user, gate_name);
        }

        let duration = start.elapsed();
        times.insert(gate_name.clone(), (result, duration.as_secs_f64() * 1000.0));
    }

    let all_duration = all_start.elapsed().as_secs_f64() * 1000.0;
    let mut times_vec: Vec<(String, (bool, f64))> = times.into_iter().collect();

    // Sort the vector by the f64 value in the tuple
    times_vec.sort_by(|a, b| b.1 .1.partial_cmp(&a.1 .1).unwrap());

    // If you need only the sorted (bool, f64) pairs, you can map the sorted vector:
    let sorted_values: Vec<(String, (bool, f64))> = times_vec;

    println!("Duration {:?}", sorted_values);
    println!("All Duration {:?}", all_duration);
}

#[tokio::test]
async fn test_gcir() {
    let (user, statsig) = setup().await;

    let start = Instant::now();

    let mut result: Option<InitializeResponse> = None;
    for _ in 0..1000 {
        result = Some(statsig.get_client_init_response(&user));
    }

    let duration = start.elapsed();
    let result_str = json!(result).to_string();
    println!("Response: {}", &result_str[0..min(200, result_str.len())]);
    // println!("Response: {}", &result_str);
    println!("Duration {}", duration.as_secs_f64() * 1000.0);
}
