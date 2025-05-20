use std::{sync::Arc, time::Duration};

use statsig_rust::{
    log_e, InstanceRegistry, SpecsAdapter, SpecsUpdateListener, StatsigErr, StatsigRuntime,
};

use crate::specs_update_listener_c::SpecsUpdateListenerC;
use async_trait::async_trait;

const TAG: &str = "FunctionBasedSpecsAdapterC";

pub struct FunctionBasedSpecsAdapterC {
    pub setup_internal_fn: extern "C" fn(listener_ref: u64),
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
            0
        });

        (self.setup_internal_fn)(ref_id);
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
    setup_internal_fn: extern "C" fn(listener_ref: u64),
    start_fn: extern "C" fn(),
    shutdown_fn: extern "C" fn(),
    schedule_background_sync_fn: extern "C" fn(),
) -> u64 {
    let adapter = FunctionBasedSpecsAdapterC {
        setup_internal_fn,
        start_fn,
        shutdown_fn,
        schedule_background_sync_fn,
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
