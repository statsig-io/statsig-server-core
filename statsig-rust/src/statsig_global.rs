use crate::{log_d, log_e};
use arc_swap::ArcSwap;
use parking_lot::Mutex;
use std::sync::{Arc, OnceLock};
use tokio::runtime::{Handle, Runtime};

const TAG: &str = "StatsigGlobal";

static ONCE: OnceLock<ArcSwap<StatsigGlobal>> = OnceLock::new();

pub struct StatsigGlobal {
    pub tokio_runtime: Mutex<Option<Arc<Runtime>>>,
    pub pid: u32,
}

impl StatsigGlobal {
    pub fn get() -> Arc<StatsigGlobal> {
        let ptr = ONCE.get_or_init(|| ArcSwap::from_pointee(StatsigGlobal::new()));

        if ptr.load().pid != current_pid() {
            let stale = ptr.swap(Arc::new(StatsigGlobal::new()));
            leak_stale_runtime(stale);
        }

        ptr.load().clone()
    }

    /// Quiesces the runtime while its worker threads still exist. This is the
    /// safe pre-fork path: dropping the runtime joins those threads before the
    /// process memory is copied.
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
            leak_stale_runtime(stale);
            return;
        }

        ptr.store(Arc::new(StatsigGlobal::new()));
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

/// A runtime inherited by a forked child refers to worker threads and an I/O
/// driver that do not exist in that process. Dropping it can panic or hang, so
/// the child intentionally leaks only that stale runtime while replacing the
/// surrounding global state.
fn leak_stale_runtime(stale: Arc<StatsigGlobal>) {
    if let Ok(inner) = Arc::try_unwrap(stale) {
        if let Some(runtime) = inner.tokio_runtime.into_inner() {
            std::mem::forget(runtime);
        }
    }
}

#[inline]
pub(crate) fn current_pid() -> u32 {
    #[cfg(not(target_family = "wasm"))]
    return std::process::id();

    #[cfg(target_family = "wasm")]
    return 0;
}
