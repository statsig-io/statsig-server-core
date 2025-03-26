use std::{ffi::c_char, sync::Arc, time::Duration};

use statsig_rust::{
    log_e, InstanceRegistry, SpecsAdapter, SpecsUpdateListener, StatsigErr, StatsigRuntime,
};

use crate::{
    ffi_utils::{c_char_to_string, string_to_c_char},
    specs_update_listener_c::SpecsUpdateListenerC,
};
use async_trait::async_trait;

const TAG: &str = "FunctionBasedSpecsAdapterC";

pub struct FunctionBasedSpecsAdapterC {
    pub setup_internal_fn: extern "C" fn(listener_ref: *const c_char),
    pub start_fn: extern "C" fn(),
    pub shutdown_fn: extern "C" fn(),
    pub schedule_background_sync_fn: extern "C" fn(),
}

#[async_trait]
impl SpecsAdapter for FunctionBasedSpecsAdapterC {
    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>) {
        let update_listener = SpecsUpdateListenerC { inner: listener };

        let ref_id = InstanceRegistry::register(update_listener).unwrap_or_else(|| {
            log_e!(TAG, "Failed to create SpecsUpdateListenerC");
            "".to_string()
        });

        let listener_ref = string_to_c_char(ref_id);

        (self.setup_internal_fn)(listener_ref);
    }

    async fn start(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        (self.start_fn)();
        Ok(())
    }

    async fn schedule_background_sync(
        self: Arc<Self>,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        (self.schedule_background_sync_fn)();
        Ok(())
    }

    async fn shutdown(
        &self,
        _timeout: Duration,
        _statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr> {
        (self.shutdown_fn)();
        Ok(())
    }

    fn get_type_name(&self) -> String {
        stringify!(FunctionBasedSpecsAdapter).to_string()
    }
}

#[no_mangle]
pub extern "C" fn function_based_specs_adapter_create(
    setup_internal_fn: extern "C" fn(listener_ref: *const c_char),
    start_fn: extern "C" fn(),
    shutdown_fn: extern "C" fn(),
    schedule_background_sync_fn: extern "C" fn(),
) -> *const c_char {
    let adapter = FunctionBasedSpecsAdapterC {
        setup_internal_fn,
        start_fn,
        shutdown_fn,
        schedule_background_sync_fn,
    };

    let ref_id = InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create FunctionBasedSpecsAdapterC");
        "".to_string()
    });

    string_to_c_char(ref_id)
}

#[no_mangle]
pub extern "C" fn function_based_specs_adapter_release(adapter_ref: *const c_char) {
    if let Some(id) = c_char_to_string(adapter_ref) {
        InstanceRegistry::remove(&id);
    }
}
