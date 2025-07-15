use arc_swap::ArcSwap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::runtime::Runtime;

static ONCE: OnceLock<ArcSwap<StatsigGlobal>> = OnceLock::new();

pub struct StatsigGlobal {
    pub tokio_runtime: Mutex<Option<Arc<Runtime>>>,
    pub pid: u32,
}

impl StatsigGlobal {
    pub fn get() -> Arc<StatsigGlobal> {
        let ptr = ONCE.get_or_init(|| ArcSwap::from_pointee(StatsigGlobal::new()));

        if ptr.load().pid != std::process::id() {
            ptr.store(Arc::new(StatsigGlobal::new()));
        }

        ptr.load().clone()
    }

    pub fn reset() {
        println!("Resetting StatsigGlobal");
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
            pid: std::process::id(),
        }
    }
}
