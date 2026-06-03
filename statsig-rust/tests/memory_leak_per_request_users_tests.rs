//! Regression coverage for unbounded growth in
//! `ExposureSampling::should_dedupe_exposure`.
//!
//! Each iteration uses a distinct `user_id`, which previously inserted into an
//! unbounded `HashSet` and grew RSS with unique-user cardinality. With the
//! bounded LRU fix the dedupe cache is capped at `exposure_dedupe_max_keys`.
//!
//! The authoritative, deterministic guard for the bound lives in the unit tests
//! next to the fix in `statsig-rust/src/event_logging/exposure_sampling.rs`
//! (`tests::test_dedupe_set_is_bounded_by_max_keys`). Those assert directly on
//! the cache length, run on every platform, and fail on the pre-fix commit.
//!
//! This integration test is a Linux-only, real-world RSS smoke check that
//! exercises the full evaluation path end-to-end. It is `#[ignore]`d by default.
//!
//! Run manually on Linux:
//! `cargo test --release --test memory_leak_per_request_users_tests -- --ignored`

mod utils;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{ClientInitResponseOptions, Statsig, StatsigOptions, StatsigUser};
use std::sync::Arc;

const WARMUP_ITERATIONS: usize = 1_000;
const DEDUPE_CAP: u32 = 1_000;

#[cfg(target_os = "linux")]
const ITERATIONS: usize = 10_000;
#[cfg(target_os = "linux")]
const MAX_RSS_GROWTH_BYTES: i64 = 50 * 1024 * 1024;

const GATE_NAME: &str = "test_public";
const CONFIG_NAME: &str = "test_empty_array";
const EXPERIMENT_NAME: &str = "test_experiment_no_targeting";
const LAYER_NAME: &str = "test_layer";

// Reads current RSS in bytes from /proc/self/status (Linux only). Panics on a
// read/parse failure so a broken measurement surfaces as a test failure rather
// than being silently skipped, which could otherwise mask a real regression.
#[cfg(target_os = "linux")]
fn read_rss_bytes() -> i64 {
    let contents =
        std::fs::read_to_string("/proc/self/status").expect("failed to read /proc/self/status");
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: i64 = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .expect("failed to parse VmRSS value");
            return kb * 1024;
        }
    }
    panic!("VmRSS field not found in /proc/self/status");
}

#[tokio::test]
#[ignore]
async fn test_memory_leak_per_request_users_bounded() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/eval_proj_dcs.json"));
    let logging_adapter = Arc::new(MockEventLoggingAdapter::new());

    let statsig = Statsig::new(
        "secret-shhh",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter.clone()),
            event_logging_adapter: Some(logging_adapter.clone()),
            exposure_dedupe_max_keys: Some(DEDUPE_CAP),
            ..StatsigOptions::new()
        })),
    );
    statsig.initialize().await.unwrap();

    // Warm-up: run the full operation set so lazy initialization, evaluation
    // caches, and allocator bucket growth settle before we capture the baseline.
    // Otherwise first-iteration allocations would be counted as "growth".
    for i in 0..WARMUP_ITERATIONS {
        let user = StatsigUser::with_user_id(format!("warmup-{i}"));
        run_evaluations(&statsig, &user);
    }

    #[cfg(not(target_os = "linux"))]
    {
        // RSS is only readable via /proc on Linux. The bound itself is verified
        // cross-platform by the deterministic unit tests in `exposure_sampling`,
        // so here we explicitly log that the RSS smoke check is being skipped.
        println!(
            "Skipping RSS smoke check on non-Linux platform ({}). \
             The dedupe-cache bound is covered cross-platform by \
             exposure_sampling::tests::test_dedupe_set_is_bounded_by_max_keys.",
            std::env::consts::OS
        );
        let _ = statsig.shutdown().await;
    }

    #[cfg(target_os = "linux")]
    {
        let initial_rss = read_rss_bytes();

        for i in 0..ITERATIONS {
            let user = StatsigUser::with_user_id(format!("user-{i}"));
            run_evaluations(&statsig, &user);
        }

        let final_rss = read_rss_bytes();
        let delta = final_rss - initial_rss;

        println!(
            "RSS: initial = {:.2} MB, final = {:.2} MB, delta = {:+.2} MB",
            initial_rss as f64 / 1024.0 / 1024.0,
            final_rss as f64 / 1024.0 / 1024.0,
            delta as f64 / 1024.0 / 1024.0,
        );

        let _ = statsig.shutdown().await;

        assert!(
            delta < MAX_RSS_GROWTH_BYTES,
            "RSS grew by {delta} bytes (> {MAX_RSS_GROWTH_BYTES} bytes); exposure dedupe cache exceeded its bound"
        );
    }
}

fn run_evaluations(statsig: &Statsig, user: &StatsigUser) {
    let _ = statsig.check_gate(user, GATE_NAME);
    let _ = statsig.get_dynamic_config(user, CONFIG_NAME);
    let _ = statsig.get_experiment(user, EXPERIMENT_NAME);
    let _ = statsig.get_layer(user, LAYER_NAME);
    let _ =
        statsig.get_client_init_response_with_options(user, &ClientInitResponseOptions::default());
}
