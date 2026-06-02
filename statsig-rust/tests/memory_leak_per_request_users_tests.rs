//! Regression test for unbounded growth in `ExposureSampling::should_dedupe_exposure`.
//!
//! Each iteration uses a distinct `user_id`, which previously inserted into an
//! unbounded `HashSet` and grew RSS with unique-user cardinality.
//!
//! With the bounded LRU fix, RSS growth should stay under the threshold below over
//! 10k iterations on a minimal user. The test sets a small `exposure_dedupe_max_keys`
//! (1_000) to force insert-time eviction.
//!
//! Run manually on Linux:
//! `cargo test --release --test memory_leak_per_request_users_tests -- --ignored`

mod utils;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{
    ClientInitResponseOptions, Statsig, StatsigOptions, StatsigUser,
};
use std::sync::Arc;

const ITERATIONS: usize = 10_000;
const DEDUPE_CAP: usize = 1_000;
const MAX_RSS_GROWTH_BYTES: i64 = 50 * 1024 * 1024;

const GATE_NAME: &str = "test_public";
const CONFIG_NAME: &str = "test_empty_array";
const EXPERIMENT_NAME: &str = "test_experiment_no_targeting";
const LAYER_NAME: &str = "test_layer";

#[cfg(target_os = "linux")]
fn read_rss_bytes() -> Option<i64> {
    let contents = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: i64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb * 1024);
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn read_rss_bytes() -> Option<i64> {
    None
}

#[tokio::test]
#[ignore]
async fn test_memory_leak_per_request_users_bounded() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data(
        "tests/data/eval_proj_dcs.json",
    ));
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

    {
        let warmup_user = StatsigUser::with_user_id("warmup");
        let _ = statsig.check_gate(&warmup_user, GATE_NAME);
    }

    let initial_rss =
        read_rss_bytes().expect("RSS reading is only supported on Linux");

    for i in 0..ITERATIONS {
        let user = StatsigUser::with_user_id(format!("user-{i}"));

        let _ = statsig.check_gate(&user, GATE_NAME);
        let _ = statsig.get_dynamic_config(&user, CONFIG_NAME);
        let _ = statsig.get_experiment(&user, EXPERIMENT_NAME);
        let _ = statsig.get_layer(&user, LAYER_NAME);
        let _ = statsig.get_client_init_response_with_options(
            &user,
            &ClientInitResponseOptions::default(),
        );
    }

    let final_rss = read_rss_bytes().expect("RSS read after loop");
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
        "RSS grew by {delta} bytes (> {MAX_RSS_GROWTH_BYTES} bytes); exposure dedupe cache may be unbounded again"
    );
}
