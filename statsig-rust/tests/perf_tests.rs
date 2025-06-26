mod utils;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use serde::{Deserialize, Serialize};
use serde_json::json;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser, StatsigUserBuilder};
use std::cmp::min;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct PerfEntry {
    value: bool,
    duration: f64,
}

async fn setup() -> (StatsigUser, Statsig, Arc<MockSpecsAdapter>) {
    let custom_ids: HashMap<String, String> =
        HashMap::from([("companyID".into(), "an_employee".into())]);

    let user = StatsigUserBuilder::new_with_user_id("user-d".to_string())
        .country(Some("GB".into()))
        .custom_ids(Some(custom_ids))
        .build();

    let mut options = StatsigOptions::new();
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/eval_proj_dcs.json"));
    options.specs_adapter = Some(specs_adapter.clone());
    options.event_logging_adapter = Some(Arc::new(MockEventLoggingAdapter::new()));
    options.environment = Some("development".to_string());

    let start = Instant::now();
    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();
    let duration = start.elapsed();
    println!("Init Duration: {}", duration.as_secs_f64() * 1000.0);

    (user, statsig, specs_adapter)
}

#[tokio::test]
async fn test_individual_gate_checks() {
    let (_, statsig, _) = setup().await;

    let gate_name = "test_public";
    let start = Instant::now();

    let mut result = false;
    for i in 0..100000 {
        let user = StatsigUser::with_user_id(format!("{}", i));
        result = statsig.check_gate(&user, gate_name);
    }

    let duration = start.elapsed();
    println!("Result {result}");
    println!("Duration {}", duration.as_secs_f64() * 1000.0);

    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_all_gate_checks() {
    let (user, statsig, _) = setup().await;

    let mut times: HashMap<String, PerfEntry> = HashMap::new();
    let values = statsig
        .get_context()
        .spec_store
        .get_current_values()
        .unwrap();

    let all_start = Instant::now();
    for gate_name in values.feature_gates.keys() {
        let start = Instant::now();

        let mut value = false;
        for _ in 0..1000 {
            value = statsig.check_gate(&user, gate_name);
        }

        let duration = start.elapsed();
        times.insert(
            gate_name.to_string(),
            PerfEntry {
                value,
                duration: duration.as_secs_f64() * 1000.0,
            },
        );
    }

    let all_duration = all_start.elapsed().as_secs_f64() * 1000.0;
    let times_json = json!(HashMap::from([("times", times.clone())])).to_string();
    let mut times_vec: Vec<(String, (bool, f64))> = times
        .into_iter()
        .map(|(key, entry)| (key, (entry.value, entry.duration)))
        .collect();

    // Sort the vector by the f64 duration in the tuple
    times_vec.sort_by(|a, b| b.1 .1.partial_cmp(&a.1 .1).unwrap());

    let sorted_values: Vec<(String, (bool, f64))> = times_vec;
    println!("Duration {sorted_values:?}");
    println!("All Duration {all_duration:?}");

    std::fs::write("/tmp/test_all_gate_checks_perf.json", times_json).unwrap();
}

#[tokio::test]
async fn test_gcir() {
    let (user, statsig, _) = setup().await;

    let start = Instant::now();

    let mut durations: Vec<f64> = Vec::new();
    let mut result: Option<String> = None;
    for _ in 0..1000 {
        let start_inner = Instant::now();
        result = Some(statsig.get_client_init_response_as_string(&user));
        let duration_inner = start_inner.elapsed();
        durations.push(duration_inner.as_secs_f64() * 1000.0);
    }

    let duration = start.elapsed();
    let result_str = json!(result).to_string();
    println!("Response: {}", &result_str[0..min(200, result_str.len())]);
    // println!("Response: {}", &result_str);
    println!("Overall {} ms", duration.as_secs_f64() * 1000.0);
    println!(
        "Average {} ms",
        durations.iter().sum::<f64>() / durations.len() as f64
    );
    println!("p99 Duration {} ms", durations.get(990).unwrap());
}

#[tokio::test]
async fn test_specs_sync() {
    let (_, _, specs_adapter) = setup().await;

    let start = Instant::now();
    for _ in 0..100 {
        specs_adapter.resync().await;
    }

    let duration = start.elapsed();
    println!("Duration {}", duration.as_secs_f64() * 1000.0);
}
