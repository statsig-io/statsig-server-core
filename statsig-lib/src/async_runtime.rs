use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::runtime::{Builder, Handle, Runtime};

pub struct AsyncRuntime {
    runtime: Mutex<Option<Runtime>>,
    pub runtime_handle: Handle,
}

impl AsyncRuntime {
    pub fn get_runtime() -> Arc<AsyncRuntime> {
        let (opt_runtime, runtime_handle) = create_runtime_if_required();

        return Arc::new(AsyncRuntime {
            runtime: Mutex::new(opt_runtime),
            runtime_handle,
        });
    }

    pub fn get_handle(&self) -> Handle {
        self.runtime_handle.clone()
    }

    pub fn shutdown(&self) {
        if let Ok(mut lock) = self.runtime.lock() {
            if let Some(runtime) = lock.take() {
                runtime.shutdown_timeout(Duration::from_secs(1))
            }
        }
    }
}

fn create_runtime_if_required() -> (Option<Runtime>, Handle) {
    if let Ok(handle) = Handle::try_current() {
        return (None, handle);
    }

    let rt = Builder::new_multi_thread()
        .worker_threads(3)
        .thread_name("statsig")
        .enable_all()
        .build()
        .expect("Failed to find or create a tokio Runtime");

    let handle = rt.handle().clone();
    (Some(rt), handle)
}
