pub use event_logging_adapter_trait::*;
pub use statsig_http_event_logging_adapter::StatsigHttpEventLoggingAdapter;
pub mod event_logging_adapter_trait;
pub mod log_event_payload;
pub mod statsig_http_event_logging_adapter;

// depends on file locking, which is unavailable on any wasm target:
#[cfg(not(target_family = "wasm"))]
pub mod statsig_local_file_event_logging_adapter;
#[cfg(not(target_family = "wasm"))]
pub use statsig_local_file_event_logging_adapter::StatsigLocalFileEventLoggingAdapter;
