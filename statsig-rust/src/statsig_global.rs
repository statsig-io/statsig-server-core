use arc_swap::ArcSwap;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tokio::runtime::Runtime;

static ONCE: OnceLock<ArcSwap<StatsigGlobal>> = OnceLock::new();

pub struct StatsigGlobal {
    pub tokio_runtime: Mutex<Option<Weak<Runtime>>>,
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

#[cfg(test)]
mod tests {
    use crate::StatsigRuntime;

    use super::*;

    fn get_strong_count() -> usize {
        let global = StatsigGlobal::get();
        let sc = global
            .tokio_runtime
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .strong_count();
        sc
    }

    #[test]
    fn test_get() {
        let global = StatsigGlobal::get();
        let original_pid = global.pid;

        let original_rt = StatsigRuntime::get_runtime();
        assert_eq!(original_pid, std::process::id());

        original_rt.spawn("test", |_| async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        });

        assert_eq!(get_strong_count(), 1,);

        let another_rt = StatsigRuntime::get_runtime();
        assert_eq!(get_strong_count(), 2);

        let pid = unsafe { libc::fork() };
        if pid == 0 {
            let child_global = StatsigGlobal::get();
            assert_ne!(child_global.pid, original_pid);

            let child_rt = StatsigRuntime::get_runtime();
            child_rt.spawn("test", |_| async {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            });

            assert_eq!(
                get_strong_count(),
                1,
                "Should have reset back to 1 because of the fork"
            );

            std::process::exit(0);
        }

        let global = StatsigGlobal::get();
        assert_eq!(global.pid, original_pid);

        unsafe {
            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);
            assert_eq!(libc::WEXITSTATUS(status), 0);
        };

        another_rt.shutdown();
        drop(another_rt);
        assert_eq!(get_strong_count(), 1);
    }
}
