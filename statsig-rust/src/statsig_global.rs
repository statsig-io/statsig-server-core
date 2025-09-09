use crate::log_d;
use arc_swap::ArcSwap;
use parking_lot::Mutex;
use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;

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
            ptr.store(Arc::new(StatsigGlobal::new()));
        }

        ptr.load().clone()
    }

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

        ptr.store(Arc::new(StatsigGlobal::new()));
    }

    fn new() -> Self {
        Self {
            tokio_runtime: Mutex::new(None),
            pid: current_pid(),
        }
    }
}

#[inline]
fn current_pid() -> u32 {
    #[cfg(not(target_family = "wasm"))]
    return std::process::id();

    #[cfg(target_family = "wasm")]
    return 0;
}
