mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::log_event_payload::LogEventRequest;
use statsig_rust::{
    EventLoggingAdapter, Statsig, StatsigErr, StatsigOptions, StatsigRuntime, StatsigUser,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Counting allocator
//
// RSS is *not* a reliable signal for whether the SDK leaks. On glibc/macOS the
// allocator keeps freed pages around as a high-water-mark reserve and only
// lazily (if ever) returns them to the OS, so RSS climbs and plateaus under
// churn even when nothing is retained. To measure whether memory is actually
// being *retained*, we track net live heap bytes (total allocated - total
// freed). Live heap only grows if something is genuinely held onto, so it is
// the metric this test asserts on. RSS is printed for visibility only.
// ---------------------------------------------------------------------------
struct CountingAlloc;

static ALLOCATED: AtomicU64 = AtomicU64::new(0);
static DEALLOCATED: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            ALLOCATED.fetch_add(new_size as u64, Ordering::Relaxed);
            DEALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// Net live heap bytes = total allocated - total freed.
fn live_heap_bytes() -> i64 {
    ALLOCATED.load(Ordering::Relaxed) as i64 - DEALLOCATED.load(Ordering::Relaxed) as i64
}

#[cfg(target_os = "linux")]
fn get_rss_bytes() -> u64 {
    let statm = match std::fs::read_to_string("/proc/self/statm") {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let resident_pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    resident_pages * page_size
}

#[cfg(target_os = "macos")]
fn get_rss_bytes() -> u64 {
    use libc::{c_int, c_void};

    #[repr(C)]
    #[derive(Default)]
    struct MachTaskBasicInfo {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        user_time: [u32; 2],
        system_time: [u32; 2],
        policy: c_int,
        suspend_count: c_int,
    }

    const MACH_TASK_BASIC_INFO: c_int = 20;
    const MACH_TASK_BASIC_INFO_COUNT: u32 =
        (std::mem::size_of::<MachTaskBasicInfo>() / std::mem::size_of::<u32>()) as u32;

    extern "C" {
        fn mach_task_self() -> u32;
        fn task_info(
            target_task: u32,
            flavor: c_int,
            task_info_out: *mut c_void,
            task_info_count: *mut u32,
        ) -> c_int;
    }

    let mut info = MachTaskBasicInfo::default();
    let mut count = MACH_TASK_BASIC_INFO_COUNT;
    let rc = unsafe {
        task_info(
            mach_task_self(),
            MACH_TASK_BASIC_INFO,
            &mut info as *mut _ as *mut c_void,
            &mut count,
        )
    };
    if rc != 0 {
        return 0;
    }
    info.resident_size
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_rss_bytes() -> u64 {
    0
}

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

/// Logging adapter that discards every event.
///
/// This is deliberate. A real (or wiremock-based) adapter retains a copy of
/// each flushed request for as long as the in-process HTTP harness keeps it
/// around, which makes the *test harness* — not the SDK — grow without bound
/// as the number of flushes increases. Using a no-op adapter isolates the
/// SDK's own memory behavior, which is what this test is measuring.
struct NoOpLoggingAdapter;

#[async_trait::async_trait]
impl EventLoggingAdapter for NoOpLoggingAdapter {
    async fn start(&self, _rt: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        Ok(())
    }
    async fn log_events(&self, _request: LogEventRequest) -> Result<bool, StatsigErr> {
        Ok(true)
    }
    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }
    fn should_schedule_background_flush(&self) -> bool {
        true
    }
}

async fn setup() -> Arc<Statsig> {
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            event_logging_adapter: Some(Arc::new(NoOpLoggingAdapter)),
            environment: Some("development".to_string()),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );
    statsig.initialize().await.unwrap();
    Arc::new(statsig)
}

/// Exercises the realistic server pattern where every request carries a
/// distinct user ID, and asserts that the SDK's memory usage is *stable*.
///
/// Each iteration builds a fresh `StatsigUser` with a unique user_id and runs
/// the full evaluation surface (check_gate / get_dynamic_config /
/// get_experiment / get_layer / get_client_init_response).
///
/// The SDK has several bounded caches that fill during a warmup ramp
/// (interned strings/values, the memoizing SHA-256 cache, the exposure dedupe
/// set). Once those are primed, net live heap must stop growing. We verify
/// this by running two equal-length phases after warmup and asserting that the
/// second phase adds essentially no net live heap — i.e. growth has plateaued
/// rather than continuing linearly (which is what a per-iteration leak would
/// produce).
#[tokio::test(flavor = "multi_thread")]
async fn per_request_users_memory_stability() {
    // Per-phase iteration count. Each iteration runs all five evaluation APIs
    // with a unique user. A phase must be long enough for the exposure dedupe
    // set to fill to its bound and reset at least once (it caps at ~100k keys),
    // so that each phase's peak live heap reflects the same "full caches" state.
    // Tunable via env for manual long runs.
    let phase_iters: usize = std::env::var("MEM_PHASE_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30_000);
    let warmup_iters: usize = std::env::var("MEM_WARMUP_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_000);

    let statsig = setup().await;

    let run_iter = |i: usize| {
        let u = StatsigUser::with_user_id(format!("user_{i}"));
        let _ = statsig.check_gate(&u, "test_public");
        let _ = statsig.get_dynamic_config(&u, "test_empty_array");
        let _ = statsig.get_experiment(&u, "exp_with_obj_and_array");
        let _ = statsig.get_layer(&u, "layer_with_many_params");
        let _ = statsig.get_client_init_response(&u);
        // `u` drops here; Rust deterministically releases its owned data.
    };

    // Runs `phase_iters` iterations starting at `start`, sampling live heap
    // periodically and returning the peak live heap observed during the phase.
    //
    // We compare *peaks* rather than end-points because the exposure dedupe set
    // is a sawtooth bounded at SAMPLING_MAX_KEYS (~100k entries, ~3 MB): an
    // end-point can land anywhere in that swing, but the peak always reflects
    // the same "full" state. The sawtooth amplitude is therefore constant
    // across phases and cancels out, leaving only genuine baseline growth.
    let run_phase = |label: &str, start: usize| -> i64 {
        let mut peak = live_heap_bytes();
        for i in 0..phase_iters {
            run_iter(start + i);
            if i % 1_000 == 0 {
                peak = peak.max(live_heap_bytes());
            }
        }
        peak = peak.max(live_heap_bytes());
        println!(
            "{label:<14} peak live heap {:>10} | RSS {:>10}",
            humanize(peak),
            humanize(get_rss_bytes() as i64),
        );
        peak
    };

    // ---- Warmup: prime all bounded caches so the steady state is reached. ----
    for i in 0..warmup_iters {
        run_iter(i);
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!(
        "post-warmup    live heap {:>10} | RSS {:>10}",
        humanize(live_heap_bytes()),
        humanize(get_rss_bytes() as i64),
    );

    // Two identical phases. With caches primed, the steady-state peak must not
    // grow from one phase to the next.
    let phase1_peak = run_phase("phase-1", warmup_iters);
    let phase2_peak = run_phase("phase-2", warmup_iters + phase_iters);

    let peak_growth = phase2_peak - phase1_peak;
    println!(
        "Peak live-heap growth, phase-1 -> phase-2 ({phase_iters} iters each): {}",
        humanize(peak_growth)
    );

    // A real per-request leak would push phase-2's peak above phase-1's by
    // roughly (leak_per_iter * phase_iters), scaling without bound as iterations
    // increase. A stable SDK keeps the peak flat across phases. The tolerance
    // covers small, bounded jitter (cache top-off, allocator bookkeeping) while
    // still catching genuine unbounded growth.
    const TOLERANCE_BYTES: i64 = 4 * 1024 * 1024;
    assert!(
        peak_growth < TOLERANCE_BYTES,
        "Live heap peak kept growing across identical phases: phase-1 peak {}, phase-2 peak {}, \
         growth {} (tolerance {}). This indicates a per-request memory leak.",
        humanize(phase1_peak),
        humanize(phase2_peak),
        humanize(peak_growth),
        humanize(TOLERANCE_BYTES),
    );
}
