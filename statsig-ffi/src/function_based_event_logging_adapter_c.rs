use crate::ffi_utils::{c_char_to_string, string_to_c_char, SafeOptBool};
use async_trait::async_trait;
use statsig_rust::{
    log_e, log_event_payload::LogEventRequest, EventLoggingAdapter, InstanceRegistry, StatsigErr,
    StatsigRuntime,
};
use std::{ffi::c_char, sync::Arc};

const TAG: &str = "FunctionBasedEventLoggingAdapterC";

const THREAD_ERROR_MESSAGE: &str =
    "Attempted to run FunctionBasedEventLoggingAdapterC on a thread other than the one it was registered on";

pub struct FunctionBasedEventLoggingAdapterC {
    pub start_fn: extern "C" fn(),
    pub log_events_fn: extern "C" fn(request_json: *const c_char) -> SafeOptBool,
    pub shutdown_fn: extern "C" fn(),

    bindings_language: String,
    registering_thread_id: std::thread::ThreadId,
}

impl FunctionBasedEventLoggingAdapterC {
    fn verify_thread_requirements(&self) -> Result<(), StatsigErr> {
        if self.bindings_language != "php" {
            return Ok(());
        }

        let current_thread_id = std::thread::current().id();
        if self.registering_thread_id != current_thread_id {
            log_e!(TAG, "{THREAD_ERROR_MESSAGE}",);
            return Err(StatsigErr::ThreadFailure(THREAD_ERROR_MESSAGE.to_string()));
        }

        Ok(())
    }
}

#[async_trait]
impl EventLoggingAdapter for FunctionBasedEventLoggingAdapterC {
    async fn start(&self, _statsig_runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        self.verify_thread_requirements()?;

        (self.start_fn)();
        Ok(())
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        self.verify_thread_requirements()?;

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
        self.verify_thread_requirements()?;

        (self.shutdown_fn)();
        Ok(())
    }

    fn should_schedule_background_flush(&self) -> bool {
        false
    }
}

#[no_mangle]
pub extern "C" fn function_based_event_logging_adapter_create(
    bindings_language: *const c_char,
    start_fn: extern "C" fn(),
    log_events_fn: extern "C" fn(request_json: *const c_char) -> SafeOptBool,
    shutdown_fn: extern "C" fn(),
) -> u64 {
    let bindings_language = match c_char_to_string(bindings_language) {
        Some(name) => name,
        None => {
            log_e!(TAG, "Failed to convert bindings_language to string");
            return 0;
        }
    };

    let adapter = FunctionBasedEventLoggingAdapterC {
        start_fn,
        log_events_fn,
        shutdown_fn,
        bindings_language,
        registering_thread_id: std::thread::current().id(),
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
