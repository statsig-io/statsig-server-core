mod utils;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use serde::Serialize;
use serde_json::Value;
use statsig_rust::{
    statsig_types::{DynamicConfig, Experiment, Layer},
    Statsig, StatsigOptions, StatsigUser, StatsigUserBuilder,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

const DEFAULT_ITERATIONS: usize = 10_000;
const SAMPLE_SIZE: usize = 32;
const BENCH_GATE_NAME: &str = "test_50_50";
const BENCH_CONFIG_NAME: &str = "operating_system_config";
const BENCH_EXPERIMENT_NAME: &str = "experiment_with_many_params";
const BENCH_LAYER_NAME: &str = "layer_with_many_params";
const EXPECTED_GATE_SAMPLE: [&str; SAMPLE_SIZE] = [
    "true", "true", "true", "false", "true", "true", "true", "true", "false", "false", "true",
    "false", "false", "true", "false", "false", "false", "false", "false", "false", "false",
    "false", "false", "true", "false", "true", "false", "false", "true", "false", "false", "false",
];
const EXPECTED_CONFIG_SAMPLE: [&str; SAMPLE_SIZE] = [
    "1", "13", "1", "13", "1", "13", "1", "13", "1", "13", "1", "13", "1", "13", "1", "13", "1",
    "13", "1", "13", "1", "13", "1", "13", "1", "13", "1", "13", "1", "13", "1", "13",
];
const EXPECTED_EXPERIMENT_SAMPLE: [&str; SAMPLE_SIZE] = [
    "test_2", "layer", "layer", "layer", "layer", "control", "layer", "control", "layer", "layer",
    "layer", "layer", "test_2", "test_2", "layer", "test_1", "test_2", "test_1", "test_2", "layer",
    "layer", "control", "layer", "test_2", "layer", "layer", "control", "test_1", "layer", "layer",
    "control", "layer",
];

#[derive(Debug, Serialize)]
struct BenchSummary {
    iterations: usize,
    operations: Vec<OperationSummary>,
}

#[derive(Debug, Serialize)]
struct OperationSummary {
    name: &'static str,
    total_ms: f64,
    avg_us: f64,
    counts: BTreeMap<String, usize>,
    sample: Vec<String>,
}

fn benchmark_iterations() -> usize {
    std::env::var("STATSIG_BENCH_ITERATIONS")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_ITERATIONS)
}

fn build_user(index: usize) -> StatsigUser {
    let email = if index % 2 == 0 {
        format!("bench-{index}@statsig.com")
    } else {
        format!("bench-{index}@example.com")
    };

    StatsigUserBuilder::new_with_user_id(format!("bench-user-{index}"))
        .email(Some(email))
        .build()
}

async fn setup() -> Statsig {
    let options = StatsigOptions {
        specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
            "tests/data/eval_proj_dcs.json",
        ))),
        event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
        ..StatsigOptions::default()
    };

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();
    statsig
}

fn operation_summary<F>(name: &'static str, users: &[StatsigUser], mut eval: F) -> OperationSummary
where
    F: FnMut(&StatsigUser) -> String,
{
    let start = Instant::now();
    let mut counts = BTreeMap::new();
    let mut sample = Vec::with_capacity(SAMPLE_SIZE.min(users.len()));

    for (index, user) in users.iter().enumerate() {
        let result = eval(user);
        *counts.entry(result.clone()).or_insert(0) += 1;

        if index < SAMPLE_SIZE {
            sample.push(result);
        }
    }

    let elapsed = start.elapsed();
    let total_ms = elapsed.as_secs_f64() * 1000.0;
    let avg_us = elapsed.as_secs_f64() * 1_000_000.0 / users.len() as f64;

    OperationSummary {
        name,
        total_ms,
        avg_us,
        counts,
        sample,
    }
}

fn config_signature(config: DynamicConfig) -> String {
    config
        .value
        .get("num")
        .and_then(Value::as_i64)
        .unwrap_or_default()
        .to_string()
}

fn experiment_signature(experiment: Experiment) -> String {
    experiment
        .value
        .get("a_string")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn layer_signature(layer: Layer) -> String {
    layer
        .__value
        .get("a_string")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn counts(entries: &[(&str, usize)]) -> BTreeMap<String, usize> {
    entries
        .iter()
        .map(|(key, value)| (key.to_string(), *value))
        .collect()
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "run with `cargo test -p statsig-rust --test core_eval_benchmark --release -- --ignored --nocapture`"]
async fn benchmark_uncached_core_evaluations() {
    let iterations = benchmark_iterations();
    let statsig = setup().await;
    let users: Vec<StatsigUser> = (0..iterations).map(build_user).collect();

    let summary = BenchSummary {
        iterations,
        operations: vec![
            operation_summary("check_gate", &users, |user| {
                statsig.check_gate(user, BENCH_GATE_NAME).to_string()
            }),
            operation_summary("get_config", &users, |user| {
                config_signature(statsig.get_dynamic_config(user, BENCH_CONFIG_NAME))
            }),
            operation_summary("get_experiment", &users, |user| {
                experiment_signature(statsig.get_experiment(user, BENCH_EXPERIMENT_NAME))
            }),
            operation_summary("get_layer", &users, |user| {
                layer_signature(statsig.get_layer(user, BENCH_LAYER_NAME))
            }),
        ],
    };

    println!("{}", serde_json::to_string_pretty(&summary).unwrap());

    assert_eq!(
        summary.operations[0].counts,
        counts(&[("false", 4981), ("true", 5019)])
    );
    assert_eq!(
        summary.operations[0].sample,
        EXPECTED_GATE_SAMPLE.map(str::to_string).to_vec()
    );

    assert_eq!(
        summary.operations[1].counts,
        counts(&[("1", 5000), ("13", 5000)])
    );
    assert_eq!(
        summary.operations[1].sample,
        EXPECTED_CONFIG_SAMPLE.map(str::to_string).to_vec()
    );

    let expected_experiment_counts = counts(&[
        ("control", 1615),
        ("layer", 5073),
        ("test_1", 1689),
        ("test_2", 1623),
    ]);
    assert_eq!(summary.operations[2].counts, expected_experiment_counts);
    assert_eq!(
        summary.operations[2].sample,
        EXPECTED_EXPERIMENT_SAMPLE.map(str::to_string).to_vec()
    );

    assert_eq!(summary.operations[3].counts, expected_experiment_counts);
    assert_eq!(
        summary.operations[3].sample,
        EXPECTED_EXPERIMENT_SAMPLE.map(str::to_string).to_vec()
    );

    statsig.shutdown().await.unwrap();
}
