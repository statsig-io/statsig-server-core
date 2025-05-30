use crate::ffi_utils::{string_to_c_char, SafeOptBool};
use async_trait::async_trait;
use statsig_rust::{
    log_e, log_event_payload::LogEventRequest, EventLoggingAdapter, InstanceRegistry, StatsigErr,
    StatsigRuntime,
};
use std::{ffi::c_char, sync::Arc};

const TAG: &str = "FunctionBasedEventLoggingAdapterC";

pub struct FunctionBasedEventLoggingAdapterC {
    pub start_fn: extern "C" fn(),
    pub log_events_fn: extern "C" fn(request_json: *const c_char) -> SafeOptBool,
    pub shutdown_fn: extern "C" fn(),
}

#[async_trait]
impl EventLoggingAdapter for FunctionBasedEventLoggingAdapterC {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        (self.start_fn)();
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        let request_json = match serde_json::to_string(&request) {
            Ok(json) => json,
            Err(e) => {
                return Err(StatsigErr::JsonParseError(
                    "Failed to serialize log event request".to_string(),
                    e.to_string(),
                ));
            }
        };

        let data = string_to_c_char(request_json);
        let result = (self.log_events_fn)(data);

        if result == 1 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        (self.shutdown_fn)();
        Ok(())
    }

    fn should_schedule_background_flush(&self) -> bool {
        false
    }
}

#[no_mangle]
pub extern "C" fn function_based_event_logging_adapter_create(
    start_fn: extern "C" fn(),
    log_events_fn: extern "C" fn(request_json: *const c_char) -> SafeOptBool,
    shutdown_fn: extern "C" fn(),
) -> u64 {
    let adapter = FunctionBasedEventLoggingAdapterC {
        start_fn,
        log_events_fn,
        shutdown_fn,
    };

    InstanceRegistry::register(adapter).unwrap_or_else(|| {
        log_e!(TAG, "Failed to create FunctionBasedEventLoggingAdapterC");
        0
    })
}

#[no_mangle]
pub extern "C" fn function_based_event_logging_adapter_release(adapter_ref: u64) {
    InstanceRegistry::remove(&adapter_ref);
}
