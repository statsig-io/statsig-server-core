use crate::{log_d, log_e};
use arc_swap::ArcSwap;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, OnceLock};
use tokio::runtime::{Handle, Runtime};

const TAG: &str = "StatsigGlobal";

static ONCE: OnceLock<ArcSwap<StatsigGlobal>> = OnceLock::new();

/// Statsig-owned runtime generation; Tokio exposes no runtime identity.
/// Unlike a PID, this changes when the parent resets its runtime before a fork.
static RUNTIME_EPOCH: AtomicU64 = AtomicU64::new(0);

#[inline]
pub(crate) fn current_runtime_epoch() -> u64 {
    RUNTIME_EPOCH.load(AtomicOrdering::SeqCst)
}

#[inline]
fn bump_runtime_epoch() {
    RUNTIME_EPOCH.fetch_add(1, AtomicOrdering::SeqCst);
}

pub struct StatsigGlobal {
    pub tokio_runtime: Mutex<Option<Arc<Runtime>>>,
    pub pid: u32,
}

impl StatsigGlobal {
    pub fn get() -> Arc<StatsigGlobal> {
        let ptr = ONCE.get_or_init(|| ArcSwap::from_pointee(StatsigGlobal::new()));

        if ptr.load().pid != current_pid() {
            let stale = ptr.swap(Arc::new(StatsigGlobal::new()));
            bump_runtime_epoch();
            leak_stale_runtime(stale);
        }

        ptr.load().clone()
    }

    /// Quiesces the runtime before a fork while its worker threads still exist.
    /// Normal drop joins them before the heap is copied, so the child cannot
    /// inherit locks whose owning threads disappeared. The runtime is rebuilt
    /// lazily on the next SDK call.
    pub fn reset() {
        log_d!(TAG, "Resetting StatsigGlobal");
        let mut did_init = false;

        let ptr = ONCE.get_or_init(|| {
            did_init = true;
            ArcSwap::from_pointee(StatsigGlobal::new())
        });

        if did_init {
            return;
        }

        if ptr.load().pid != current_pid() {
            let stale = ptr.swap(Arc::new(StatsigGlobal::new()));
            bump_runtime_epoch();
            leak_stale_runtime(stale);
            return;
        }

        ptr.store(Arc::new(StatsigGlobal::new()));
        bump_runtime_epoch();
    }

    /// Stops the shared tokio runtime, joining its worker threads.
    ///
    /// Per-instance shutdown deliberately leaves the shared runtime running so
    /// later instances can reuse it. Callers that unload this library while the
    /// process keeps running need those threads gone first, and this is the
    /// only thing that stops them.
    ///
    /// Returns false without resetting when called from a runtime thread.
    /// Dropping a runtime from inside itself panics, and binding teardown can
    /// reach this from one, so the guard lives here rather than in callers.
    pub fn shutdown_shared_runtime() -> bool {
        if Handle::try_current().is_ok() {
            log_e!(
                TAG,
                "Refusing to shut down the shared runtime from inside a runtime thread"
            );
            return false;
        }

        Self::reset();
        true
    }

    fn new() -> Self {
        Self {
            tokio_runtime: Mutex::new(None),
            pid: current_pid(),
        }
    }
}

/// Leaks only the child's stale post-fork runtime. Its worker threads and I/O
/// driver no longer exist, so dropping it can panic or hang while signaling or
/// joining them. This is a one-time cost per fork; pre-fork `reset()` drops the
/// live runtime normally.
fn leak_stale_runtime(stale: Arc<StatsigGlobal>) {
    if let Ok(inner) = Arc::try_unwrap(stale) {
        if let Some(runtime) = inner.tokio_runtime.into_inner() {
            std::mem::forget(runtime);
        }
    }
    // If another reference exists, leave it alone rather than force a drop.
}

#[inline]
pub(crate) fn current_pid() -> u32 {
    #[cfg(not(target_family = "wasm"))]
    return std::process::id();

    #[cfg(target_family = "wasm")]
    return 0;
}
