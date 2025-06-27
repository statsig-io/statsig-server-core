use rand::Rng;
use serde_json::{json, Value};
use statsig::{Statsig, StatsigEvent, StatsigUser};
use std::collections::HashMap;
use std::time::Instant;
use std::{env, fs};

const SDK_TYPE: &str = "rust-server";
const CORE_ITER: usize = 100_000;
const GCIR_ITER: usize = 1000;

fn make_random_user() -> StatsigUser {
    let mut rng = rand::thread_rng();
    StatsigUser::with_user_id(format!("user_{}", rng.gen::<u32>()))
}

pub fn run_bench<F>(iterations: usize, func: F) -> f64
where
    F: Fn() -> (),
{
    let mut durations = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        func();
        let duration = start.elapsed();
        durations.push(duration.as_secs_f64() * 1000.0); // Convert to milliseconds
    }

    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p99_index = (iterations as f64 * 0.99) as usize;
    durations[p99_index]
}

fn log_benchmark(name: &str, version: &str, p99: f64, global_user: &StatsigUser) {
    println!("{:<50} {:.4}ms", name, p99);

    let ci = env::var("CI").unwrap_or_default();
    if ci != "1" && ci != "true" {
        return;
    }

    let event = StatsigEvent {
        event_name: "sdk_benchmark".to_string(),
        value: Some(json!(p99)),
        metadata: Some(HashMap::from([
            ("benchmarkName".to_string(), json!(name)),
            ("sdkType".to_string(), json!(SDK_TYPE)),
            ("sdkVersion".to_string(), json!(version)),
        ])),
    };

    if let Some(err) = Statsig::log_event(global_user, event) {
        println!("Error logging event: {:?}", err);
    }
}

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[tokio::main]
pub async fn main() {
    let statsig_version = built_info::DEPENDENCIES
        .iter()
        .find(|(name, _)| *name == "statsig")
        .map(|(_, version)| *version)
        .unwrap_or("unknown");

    let metadata_file = env::var("BENCH_METADATA_FILE").expect("BENCH_METADATA_FILE must be set");
    let metadata = HashMap::from([
        ("sdk_type".to_string(), SDK_TYPE.to_string()),
        ("sdk_version".to_string(), statsig_version.to_string()),
    ]);
    let metadata_json = serde_json::to_string(&metadata).unwrap();
    fs::write(metadata_file, metadata_json).unwrap();

    let key = env::var("PERF_SDK_KEY").expect("PERF_SDK_KEY must be set");
    if let Some(err) = Statsig::initialize(&key).await {
        panic!("Failed to initialize Statsig: {:?}", err);
    }

    let global_user = StatsigUser::with_user_id("global_user".to_string());
    let mut results = HashMap::new();

    println!("Statsig Rust Legacy (v{})", statsig_version);
    println!("--------------------------------");

    // Check Gate
    let p99 = run_bench(CORE_ITER, || {
        let user = make_random_user();
        let _ = Statsig::check_gate(&user, "test_advanced");
    });
    results.insert("check_gate", p99);

    // Check Gate Global User
    let p99 = run_bench(CORE_ITER, || {
        let _ = Statsig::check_gate(&global_user, "test_advanced");
    });
    results.insert("check_gate_global_user", p99);

    // Get Feature Gate
    let p99 = run_bench(CORE_ITER, || {
        let user = make_random_user();
        let _ = Statsig::get_feature_gate(&user, "test_advanced");
    });
    results.insert("get_feature_gate", p99);

    // Get Feature Gate Global User
    let p99 = run_bench(CORE_ITER, || {
        let _ = Statsig::get_feature_gate(&global_user, "test_advanced");
    });
    results.insert("get_feature_gate_global_user", p99);

    // Get Experiment
    let p99 = run_bench(CORE_ITER, || {
        let user = make_random_user();
        let _ = Statsig::get_experiment::<Value>(&user, "an_experiment");
    });
    results.insert("get_experiment", p99);

    // Get Experiment Global User
    let p99 = run_bench(CORE_ITER, || {
        let _ = Statsig::get_experiment::<Value>(&global_user, "an_experiment");
    });
    results.insert("get_experiment_global_user", p99);

    // Get Client Initialize Response
    let p99 = run_bench(GCIR_ITER, || {
        let user = make_random_user();
        let res = Statsig::get_client_initialize_response(&user).unwrap();
        serde_json::to_string(&res).unwrap();
    });
    results.insert("get_client_initialize_response", p99);

    // Get Client Initialize Response Global User
    let p99 = run_bench(GCIR_ITER, || {
        let res = Statsig::get_client_initialize_response(&global_user).unwrap();
        serde_json::to_string(&res).unwrap();
    });
    results.insert("get_client_initialize_response_global_user", p99);

    // Log results
    for (name, p99) in results {
        log_benchmark(name, statsig_version, p99, &global_user);
    }

    if let Some(e) = Statsig::shutdown().await {
        println!("Error shutting down Statsig: {:?}", e);
    }
    println!("\n\n");
}
