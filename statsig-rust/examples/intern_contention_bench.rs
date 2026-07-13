// Micro-bench: is InternedString get+drop the eval-path serializer?
//
// Every evaluation does InternedString::from_str_ref(spec_name) (evaluator.rs)
// and later drops it (release_string). Both operations touch global interned
// storage. This bench isolates exactly that pair, same hot key on all threads,
// mirroring a hot gate at high QPS.
//
//   BENCH_THREADS   comma list (default 1,4,16,64)
//   BENCH_DURATION_SEC (default 5)
//
// Run: cargo run --release --example intern_contention_bench

use statsig_rust::interned_string::InternedString;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    let thread_counts: Vec<usize> = std::env::var("BENCH_THREADS")
        .unwrap_or_else(|_| "1,4,16,64".into())
        .split(',')
        .map(|s| s.trim().parse().unwrap())
        .collect();
    let duration_sec: u64 = std::env::var("BENCH_DURATION_SEC")
        .unwrap_or_else(|_| "5".into())
        .parse()
        .unwrap();

    for &threads in &thread_counts {
        let stop = Arc::new(AtomicBool::new(false));
        let total = Arc::new(AtomicU64::new(0));
        let mut handles = Vec::new();
        for _ in 0..threads {
            let stop = stop.clone();
            let total = total.clone();
            handles.push(std::thread::spawn(move || {
                let mut ops = 0u64;
                while !stop.load(Ordering::Relaxed) {
                    // The exact per-eval pair: intern the (hot) spec name,
                    // then drop it (which calls release_string).
                    let s = InternedString::from_str_ref("test_many_rules");
                    std::hint::black_box(&s);
                    drop(s);
                    ops += 1;
                }
                total.fetch_add(ops, Ordering::Relaxed);
            }));
        }
        let start = Instant::now();
        std::thread::sleep(Duration::from_secs(duration_sec));
        stop.store(true, Ordering::Relaxed);
        for h in handles {
            h.join().unwrap();
        }
        let el = start.elapsed().as_secs_f64();
        let ops = total.load(Ordering::Relaxed);
        println!(
            "threads={threads:>3}  intern+drop/sec={:>12.0}  per-thread={:>11.0}",
            ops as f64 / el,
            ops as f64 / el / threads as f64
        );
    }
}
