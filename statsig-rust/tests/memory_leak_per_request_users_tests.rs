mod utils;

use crate::utils::mock_scrapi::StubData;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::sync::Arc;
use std::time::Duration;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

/// Returns resident set size in bytes by reading /proc/self/statm.
/// Linux only; the test is `#[cfg(target_os = "linux")]` below.
#[cfg(target_os = "linux")]
fn get_rss_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let resident_pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("parse resident pages");
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    resident_pages * page_size
}

#[cfg(target_os = "linux")]
fn humanize(bytes: i64) -> String {
    let abs = bytes.unsigned_abs() as f64;
    if abs < 1024.0 {
        format!("{bytes} B")
    } else if abs < 1024.0 * 1024.0 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if abs < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

async fn setup() -> (Arc<Statsig>, MockScrapi) {
    let mock_scrapi = MockScrapi::new().await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String("{}".to_string()),
            status: 200,
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            environment: Some("development".to_string()),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );
    statsig.initialize().await.unwrap();
    (Arc::new(statsig), mock_scrapi)
}

/// Repro: server-side workload where each request carries a distinct userID.
///
/// The existing tests in other bindings (Go's TestMemoryLeak, C++'s
/// memory_safety_test) either reuse a single fixed user or construct per-iter
/// users but discard them without using them in evaluations. This test
/// simulates the realistic server pattern: every iteration builds a fresh
/// StatsigUser with a unique user_id and passes it to check_gate /
/// get_dynamic_config / get_experiment / get_layer / get_client_init_response.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn per_request_users_memory_leak() {
    let (statsig, _scrapi) = setup().await;

    // Warmup
    for i in 0..10 {
        let u = StatsigUser::with_user_id(format!("warmup_user_{i}"));
        let _ = statsig.check_gate(&u, "test_public");
        let _ = statsig.get_dynamic_config(&u, "test_empty_array");
        let _ = statsig.get_experiment(&u, "exp_with_obj_and_array");
        let _ = statsig.get_layer(&u, "layer_with_many_params");
        let _ = statsig.get_client_init_response(&u);
    }

    tokio::time::sleep(Duration::from_secs(1)).await;

    let initial_rss = get_rss_bytes() as i64;
    println!("Initial RSS: {}", humanize(initial_rss));

    const ITERATIONS: usize = 10_000;
    for i in 0..ITERATIONS {
        let u = StatsigUser::with_user_id(format!("user_{i}"));
        let _ = statsig.check_gate(&u, "test_public");
        let _ = statsig.get_dynamic_config(&u, "test_empty_array");
        let _ = statsig.get_experiment(&u, "exp_with_obj_and_array");
        let _ = statsig.get_layer(&u, "layer_with_many_params");
        let _ = statsig.get_client_init_response(&u);
        // `u` drops here; Rust deterministically releases its owned data.
    }

    tokio::time::sleep(Duration::from_secs(1)).await;

    let final_rss = get_rss_bytes() as i64;
    println!("Final RSS:   {}", humanize(final_rss));

    let delta = final_rss - initial_rss;
    let pct = (delta as f64) / (initial_rss as f64) * 100.0;
    println!("Delta:       {} ({:.2}%)", humanize(delta), pct);

    assert!(
        pct < 50.0,
        "Memory leak with per-request users: {} ({:.2}%)",
        humanize(delta),
        pct
    );
}
