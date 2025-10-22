use std::{ffi::c_char, sync::Arc, time::Duration};

use statsig_rust::{
    log_e, InstanceRegistry, SpecsAdapter, SpecsUpdateListener, StatsigErr, StatsigRuntime,
};

use crate::{ffi_utils::c_char_to_string, specs_update_listener_c::SpecsUpdateListenerC};
use async_trait::async_trait;

const TAG: &str = "FunctionBasedSpecsAdapterC";

const THREAD_ERROR_MESSAGE: &str =
    "Attempted to run FunctionBasedSpecsAdapterC on a thread other than the one it was registered on";

pub struct FunctionBasedSpecsAdapterC {
    pub setup_internal_fn: extern "C" fn(listener_ref: u64),
    pub start_fn: extern "C" fn(),
    pub shutdown_fn: extern "C" fn(),
    pub schedule_background_sync_fn: extern "C" fn(),

    bindings_language: String,
    registering_thread_id: std::thread::ThreadId,
}

impl FunctionBasedSpecsAdapterC {
    fn verify_thread_requirements(&self) -> Result<(), StatsigErr> {
        if self.bindings_language != "php" {
            return Ok(());
        };

        let current_thread_id = std::thread::current().id();
        if self.registering_thread_id != current_thread_id {
            log_e!(TAG, "{THREAD_ERROR_MESSAGE}",);
            return Err(StatsigErr::ThreadFailure(THREAD_ERROR_MESSAGE.to_string()));
        }

        Ok(())
    }
}

#[async_trait]
impl SpecsAdapter for FunctionBasedSpecsAdapterC {
    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        if self.verify_thread_requirements().is_err() {
            return;
        }

        let update_listener = SpecsUpdateListenerC { inner: listener };

        let ref_id = InstanceRegistry::register(update_listener).unwrap_or_else(|| {
            log_e!(TAG, "Failed to create SpecsUpdateListenerC");
            0
        });

        (self.setup_internal_fn)(ref_id);
    }

    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.verify_thread_requirements()?;

        (self.start_fn)();
        Ok(())
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.verify_thread_requirements()?;

        (self.schedule_background_sync_fn)();
        Ok(())
    }

    async fn shutdown(
        &self,
        _timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        self.verify_thread_requirements()?;

        (self.shutdown_fn)();
        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(FunctionBasedSpecsAdapter).to_string()
    }
}

#[no_mangle]
pub extern "C" fn function_based_specs_adapter_create(
    bindings_language: *const c_char,
    setup_internal_fn: extern "C" fn(listener_ref: u64),
    start_fn: extern "C" fn(),
    shutdown_fn: extern "C" fn(),
    schedule_background_sync_fn: extern "C" fn(),
) -> u64 {
    let bindings_language = match c_char_to_string(bindings_language) {
        Some(name) => name,
        None => {
            log_e!(TAG, "Failed to convert bindings_language to string");
            return 0;
        }
    };

    let adapter = FunctionBasedSpecsAdapterC {
        bindings_language,
        setup_internal_fn,
        start_fn,
        shutdown_fn,
        schedule_background_sync_fn,
        registering_thread_id: std::thread::current().id(),
    };

    InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create FunctionBasedSpecsAdapterC");
        0
    })
}

#[no_mangle]
pub extern "C" fn function_based_specs_adapter_release(adapter_ref: u64) {
    InstanceRegistry::remove(&adapter_ref);
}
