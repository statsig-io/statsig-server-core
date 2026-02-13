use once_cell::sync::Lazy;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::{collections::HashMap, sync::Arc, time::Instant};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

const SCAPI_URL: &str = "http://scrapi:8000";
const SDK_TYPE: &str = "statsig-server-core-rust";
const ITER_ULTRA_LITE: u32 = 100;
const ITER_LITE: u32 = 1000;
const ITER_HEAVY: u32 = 10_000;
const SDK_VERSION: Lazy<String> = Lazy::new(|| {
    built_info::DEPENDENCIES
        .iter()
        .find(|(name, _)| *name == "statsig-rust")
        .map(|(_, version)| *version)
        .unwrap_or("unknown")
        .to_string()
});

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkResult {
    #[serde(rename = "benchmarkName")]
    pub benchmark_name: String,
    pub p99: f64,
    pub max: f64,
    pub min: f64,
    pub median: f64,
    pub avg: f64,
    #[serde(rename = "specName")]
    pub spec_name: String,
    #[serde(rename = "sdkType")]
    pub sdk_type: String,
    #[serde(rename = "sdkVersion")]
    pub sdk_version: String,
}

pub struct BenchCore;

impl BenchCore {
    pub async fn run() {
        println!("Statsig Rust Core (v{})", SDK_VERSION.as_str());
        println!("--------------------------------");

        let spec_names = load_spec_names();

        let options = Arc::new(StatsigOptions {
            specs_url: Some(format!("{SCAPI_URL}/v2/download_config_specs")),
            log_event_url: Some(format!("{SCAPI_URL}/v1/log_event")),
            ..Default::default()
        });

        let mut results = Vec::new();

        let opts_clone = options.clone();
        benchmark(
            &mut results,
            "initialize",
            "proto",
            ITER_ULTRA_LITE,
            async move || {
                Statsig::new("secret-RUST_CORE::BC_USE_PROTO", Some(opts_clone.clone()))
                    .initialize()
                    .await
                    .unwrap()
            },
        )
        .await;

        let opts_clone = options.clone();
        benchmark(
            &mut results,
            "initialize",
            "json",
            ITER_ULTRA_LITE,
            async move || {
                Statsig::new("secret-RUST_CORE::BC_USE_JSON", Some(opts_clone.clone()))
                    .initialize()
                    .await
                    .unwrap()
            },
        )
        .await;

        let statsig = Statsig::new_shared("secret-RUST_CORE", Some(options)).unwrap();
        statsig.initialize().await.unwrap();

        let global_user = StatsigUser::with_user_id("global_user");

        for gate_name in spec_names["feature_gates"].iter() {
            benchmark(
                &mut results,
                "check_gate",
                gate_name,
                ITER_HEAVY,
                async || {
                    let user = create_user();
                    Statsig::shared().check_gate(&user, gate_name);
                },
            )
            .await;

            benchmark(
                &mut results,
                "check_gate_global_user",
                gate_name,
                ITER_HEAVY,
                async || {
                    Statsig::shared().check_gate(&global_user, gate_name);
                },
            )
            .await;

            benchmark(
                &mut results,
                "get_feature_gate",
                gate_name,
                ITER_HEAVY,
                async || {
                    let user = create_user();
                    Statsig::shared().get_feature_gate(&user, gate_name);
                },
            )
            .await;

            benchmark(
                &mut results,
                "get_feature_gate_global_user",
                gate_name,
                ITER_HEAVY,
                async || {
                    Statsig::shared().get_feature_gate(&global_user, gate_name);
                },
            )
            .await;
        }

        for config_name in spec_names["dynamic_configs"].iter() {
            benchmark(
                &mut results,
                "get_dynamic_config",
                config_name,
                ITER_HEAVY,
                async || {
                    let user = create_user();
                    Statsig::shared().get_dynamic_config(&user, config_name);
                },
            )
            .await;

            benchmark(
                &mut results,
                "get_dynamic_config_global_user",
                config_name,
                ITER_HEAVY,
                async || {
                    Statsig::shared().get_dynamic_config(&global_user, config_name);
                },
            )
            .await;
        }

        for experiment_name in spec_names["experiments"].iter() {
            benchmark(
                &mut results,
                "get_experiment",
                experiment_name,
                ITER_HEAVY,
                async || {
                    let user = create_user();
                    Statsig::shared().get_experiment(&user, experiment_name);
                },
            )
            .await;

            benchmark(
                &mut results,
                "get_experiment_global_user",
                experiment_name,
                ITER_HEAVY,
                async || {
                    Statsig::shared().get_experiment(&global_user, experiment_name);
                },
            )
            .await;
        }

        for layer_name in spec_names["layers"].iter() {
            benchmark(
                &mut results,
                "get_layer",
                layer_name,
                ITER_HEAVY,
                async || {
                    let user = create_user();
                    Statsig::shared().get_layer(&user, layer_name);
                },
            )
            .await;

            benchmark(
                &mut results,
                "get_layer_global_user",
                layer_name,
                ITER_HEAVY,
                async || {
                    Statsig::shared().get_layer(&global_user, layer_name);
                },
            )
            .await;
        }

        statsig.shutdown().await.unwrap();

        write_results(&results);
    }
}

fn load_spec_names() -> HashMap<String, Vec<String>> {
    let path = "/shared-volume/spec_names.json";
    for i in 1..11 {
        if std::fs::metadata(path).is_ok() {
            break;
        }
        println!("({i}/10) Waiting for spec_names.json to be created...");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    let json = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn create_user() -> StatsigUser {
    let rnd = rand::rng().random_range(0..1000000);
    let mut user = StatsigUser::with_user_id(format!("user_{}", rnd));
    user.set_email("user@example.com");
    user.set_ip("127.0.0.1");
    user.set_locale("en-US");
    user.set_app_version("1.0.0");
    user.set_country("US");
    user.set_user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36");
    user.set_custom(HashMap::from([("isAdmin".to_string(), false)]));
    user.set_private_attributes(HashMap::from([("isPaid".to_string(), "nah")]));
    user
}

async fn benchmark<F>(
    results: &mut Vec<BenchmarkResult>,
    benchmark_name: &str,
    spec_name: &str,
    iterations: u32,
    func: F,
) where
    F: AsyncFn() -> (),
{
    let mut durations = Vec::with_capacity(iterations as usize);

    for _ in 0..iterations {
        let start = Instant::now();
        func().await;
        let duration = start.elapsed();
        durations.push(duration.as_secs_f64() * 1000.0); // Convert to milliseconds
    }

    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let result = BenchmarkResult {
        benchmark_name: benchmark_name.to_string(),
        spec_name: spec_name.to_string(),
        sdk_type: SDK_TYPE.to_string(),
        sdk_version: SDK_VERSION.as_str().to_string(),
        p99: durations[durations.len() * 99 / 100],
        max: *durations.last().unwrap(),
        min: *durations.first().unwrap(),
        median: durations[durations.len() / 2],
        avg: durations.iter().sum::<f64>() / durations.len() as f64,
    };

    // System.out.printf("%-30s p99(%.4fms) max(%.4fms) %s\n", benchName, result.p99, result.max, specName);

    println!(
        "{:30} p99(ms): {:.4} max(ms): {:.4} {}",
        benchmark_name, result.p99, result.max, spec_name
    );

    results.push(result);
}

fn write_results(results: &Vec<BenchmarkResult>) {
    let sdk_version = SDK_VERSION.as_str().to_string();
    let mut root = HashMap::new();
    root.insert("sdkType", json!(SDK_TYPE));
    root.insert("sdkVersion", json!(sdk_version));
    root.insert("results", json!(results));
    let json = serde_json::to_string_pretty(&root).unwrap();
    let out_path = format!("/shared-volume/{}-{}-results.json", SDK_TYPE, sdk_version);
    std::fs::write(out_path, json).unwrap();
}
