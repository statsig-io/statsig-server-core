//! Minimal reproduction: `check_gate` throughput goes DOWN as threads go up.
//!
//! One shared `Statsig` instance (the normal server deployment) is hit by N OS
//! threads concurrently, each calling `check_gate` in a tight loop. We print the
//! aggregate throughput (evaluations/sec) at increasing thread counts. On a
//! machine with several cores, aggregate throughput should rise with more
//! threads -- but it falls, which is the bug.
//!
//! Offline: specs come from a bundled DCS file and events go to a no-op adapter,
//! so this measures only in-process evaluation, no network.
//!
//! Run with:
//!   cargo test --release --test check_gate_concurrency_bench -- --ignored --nocapture

mod utils;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

const DCS_PATH: &str = "tests/data/check_gate_perf_dcs.json";
const GATE_NAME: &str = "public_dev_only"; // cheap single-rule gate
const CALLS_PER_THREAD: usize = 50_000;
const THREAD_COUNTS: &[usize] = &[1, 2, 4, 8, 16];

#[tokio::test(flavor = "multi_thread")]
#[ignore = "benchmark; run with --ignored --nocapture"]
async fn check_gate_throughput_vs_threads() {
    let statsig = Arc::new(Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(DCS_PATH))),
            event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
            environment: Some("development".to_string()),
            ..StatsigOptions::new()
        })),
    ));
    statsig.initialize().await.unwrap();

    // Warm up so steady-state numbers aren't polluted by first-call costs.
    let warm = StatsigUser::with_user_id("warmup");
    for _ in 0..2_000 {
        let _ = statsig.check_gate(&warm, GATE_NAME);
    }

    let cores = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);
    println!("\ncheck_gate throughput vs threads (gate=\"{GATE_NAME}\", {CALLS_PER_THREAD} calls/thread, {cores} cores)");
    println!(
        "{:>8} | {:>16} | {:>10}",
        "threads", "evals/sec", "vs 1-thread"
    );

    let mut base = 0.0;
    for (i, &threads) in THREAD_COUNTS.iter().enumerate() {
        if threads > cores {
            continue;
        }
        // +1 so the main thread participates: the timer starts only after all
        // workers are spawned and released, excluding thread-creation overhead.
        let barrier = Arc::new(Barrier::new(threads + 1));
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let statsig = statsig.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    // One distinct user per thread (kept constant within the
                    // thread so we measure evaluation, not user allocation).
                    let user = StatsigUser::with_user_id(format!("user_{t}"));
                    barrier.wait();
                    for _ in 0..CALLS_PER_THREAD {
                        let _ = statsig.check_gate(&user, GATE_NAME);
                    }
                })
            })
            .collect();
        // Record start before joining the barrier: workers are blocked until the
        // main thread arrives, so no check_gate work can precede the timestamp.
        let start = Instant::now();
        barrier.wait();
        for h in handles {
            h.join().unwrap();
        }
        let throughput = (threads * CALLS_PER_THREAD) as f64 / start.elapsed().as_secs_f64();
        if i == 0 {
            base = throughput;
        }
        println!(
            "{:>8} | {:>16.0} | {:>9.2}x",
            threads,
            throughput,
            throughput / base
        );
    }

    statsig.shutdown().await.unwrap();
}
