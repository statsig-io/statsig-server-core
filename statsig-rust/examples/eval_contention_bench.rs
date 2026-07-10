// Pure-Rust eval contention bench (S2SDK-140 follow-up).
//
// Java-harness profiling of the heavy-gate workload showed ~50% of CPU time
// in futex/sched_yield - lock churn on the eval hot path - but the native
// frames had no symbols. This bench removes the JVM/JNI entirely (which also
// removes InstanceRegistry from the path) so we can (a) see if the thread-
// scaling collapse reproduces in pure Rust and (b) profile with symbols.
//
// Isolation knobs:
//   BENCH_THREADS      comma list of thread counts to sweep   (default 1,4,16,64)
//   BENCH_GATE         gate to evaluate                        (default test_many_rules)
//   BENCH_MANY_GATES   >0: cycle syn_gate_NNNN names to spread the
//                      non_exposed_checks DashMap across shards (default 0)
//   BENCH_DURATION_SEC seconds per measurement                 (default 10)
//   BENCH_DCS_FILE     specs payload                           (default ../grammarly-load-repro/dcs_payload_big.json)
//   BENCH_EXPOSURES    "1": log exposures instead of the non-exposure path
//   BENCH_REUSE_USER   evals per fresh user                    (default 40)
//
// Run: cargo run --release --example eval_contention_bench
//
// Comparing `same gate` vs `BENCH_MANY_GATES=3000` isolates single-key
// contention on non_exposed_checks; profiling with symbols (samply/sample)
// attributes the rest (spec_store RwLock vs eval compute).

use statsig_rust::{
    hashing::djb2, FeatureGateEvaluationOptions, Statsig, StatsigLocalFileSpecsAdapter,
    StatsigOptions, StatsigUser, StatsigUserBuilder,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn env(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn make_user(idx: u64) -> StatsigUser {
    StatsigUserBuilder::new_with_user_id(format!("user_{idx}"))
        .email(Some(format!("u{}@example.com", idx % 1000)))
        .custom_ids(Some(HashMap::from([(
            "companyID".to_string(),
            format!("c{}", idx % 500),
        )])))
        .country(Some("US".to_string()))
        .locale(Some("en_US".to_string()))
        .app_version(Some(format!("5.2.{}", idx % 90)))
        .custom_from_str_map(Some(HashMap::from([
            ("plan".to_string(), "premium".to_string()),
            ("cohort".to_string(), format!("c{}", idx % 20)),
            ("platform".to_string(), "mac".to_string()),
            ("abGroup".to_string(), format!("g{}", idx % 8)),
        ])))
        .build()
}

#[tokio::main]
async fn main() {
    let dcs_file = env(
        "BENCH_DCS_FILE",
        "../../grammarly-load-repro/dcs_payload_big.json",
    );
    let gate = env("BENCH_GATE", "test_many_rules");
    let many_gates: u64 = env("BENCH_MANY_GATES", "0").parse().unwrap();
    let duration_sec: u64 = env("BENCH_DURATION_SEC", "10").parse().unwrap();
    let reuse_user: u64 = env("BENCH_REUSE_USER", "40").parse().unwrap();
    let log_exposures = env("BENCH_EXPOSURES", "0") == "1";
    let thread_counts: Vec<usize> = env("BENCH_THREADS", "1,4,16,64")
        .split(',')
        .map(|s| s.trim().parse().unwrap())
        .collect();

    let sdk_key = "secret-bench-key";

    // StatsigLocalFileSpecsAdapter reads {dir}/{djb2(key)}_specs.json.
    // Per-process dir so concurrent bench runs don't clobber each other's specs.
    let dir = std::env::temp_dir().join(format!("statsig_eval_bench_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let specs_path = dir.join(format!("{}_specs.json", djb2(sdk_key)));
    std::fs::copy(&dcs_file, &specs_path)
        .unwrap_or_else(|e| panic!("copy {dcs_file} -> {specs_path:?}: {e}"));

    let adapter = Arc::new(StatsigLocalFileSpecsAdapter::new(
        sdk_key,
        dir.to_str().unwrap(),
        None,
        false,
        true, // disable_network
    ));

    let options = StatsigOptions {
        specs_adapter: Some(adapter.clone()),
        disable_network: Some(true),
        ..Default::default()
    };

    let statsig = Arc::new(Statsig::new(sdk_key, Some(Arc::new(options))));
    statsig.initialize().await.expect("initialize");
    adapter.resync_from_file().expect("resync specs from file");

    let probe = make_user(0);
    println!(
        "probe check_gate({gate}) = {} (exposures={log_exposures}, manyGates={many_gates})",
        statsig.check_gate(&probe, &gate)
    );

    for &threads in &thread_counts {
        let stop = Arc::new(AtomicBool::new(false));
        let total_ops = Arc::new(AtomicU64::new(0));
        let mut handles = Vec::new();

        for t in 0..threads {
            let statsig = statsig.clone();
            let stop = stop.clone();
            let total_ops = total_ops.clone();
            let gate = gate.clone();
            handles.push(std::thread::spawn(move || {
                let opts = FeatureGateEvaluationOptions {
                    disable_exposure_logging: !log_exposures,
                };
                let mut ops: u64 = 0;
                let mut user = make_user(t as u64);
                while !stop.load(Ordering::Relaxed) {
                    ops += 1;
                    if ops % reuse_user == 0 {
                        user = make_user((t as u64) << 32 | ops);
                    }
                    let gate_name = if many_gates > 0 {
                        format!("syn_gate_{:04}", ops % many_gates)
                    } else {
                        gate.clone()
                    };
                    statsig.check_gate_with_options(&user, &gate_name, opts.clone());
                }
                total_ops.fetch_add(ops, Ordering::Relaxed);
            }));
        }

        let start = Instant::now();
        std::thread::sleep(Duration::from_secs(duration_sec));
        stop.store(true, Ordering::Relaxed);
        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed().as_secs_f64();
        let ops = total_ops.load(Ordering::Relaxed);
        println!(
            "threads={threads:>3}  total={ops:>12}  qps={:>12.0}  per-thread qps={:>10.0}",
            ops as f64 / elapsed,
            ops as f64 / elapsed / threads as f64
        );
    }

    statsig.shutdown().await.ok();
}
