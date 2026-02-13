mod utils;

use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{output_logger::OutputLogProvider, Statsig, StatsigOptions, StatsigUser};
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;

struct SlowOutputLoggerProvider {
    is_blocked: AtomicBool,
    did_block_logger: AtomicBool,
}

impl SlowOutputLoggerProvider {
    pub fn new() -> Self {
        Self {
            is_blocked: AtomicBool::new(false),
            did_block_logger: AtomicBool::new(false),
        }
    }
}

impl OutputLogProvider for SlowOutputLoggerProvider {
    fn initialize(&self) {}

    fn debug(&self, _tag: &str, _msg: String) {}

    fn info(&self, _tag: &str, _msg: String) {}

    fn warn(&self, _tag: &str, _msg: String) {}

    fn error(&self, _tag: &str, _msg: String) {
        while self.is_blocked.load(std::sync::atomic::Ordering::SeqCst) {
            self.did_block_logger
                .store(true, std::sync::atomic::Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    fn shutdown(&self) {}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_spec_store_lock() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/dcs_error.json"));
    let specs_adapter_clone = specs_adapter.clone();

    let output_logger_provider = Arc::new(SlowOutputLoggerProvider::new());

    let options = StatsigOptions {
        specs_adapter: Some(specs_adapter.clone()),
        event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
        output_logger_provider: Some(output_logger_provider.clone()),
        ..StatsigOptions::default()
    };

    let statsig = Arc::new(Statsig::new("secret-key", Some(Arc::new(options))));
    let _ = statsig.initialize().await;

    output_logger_provider
        .is_blocked
        .store(true, Ordering::SeqCst);

    let statsig_clone = statsig.clone();
    let check_handle = tokio::task::spawn(async move {
        let user = StatsigUser::with_user_id("user1");

        for _ in 0..1000 {
            let gate = statsig_clone.get_feature_gate(&user, "test_public");
            drop(gate);
        }
    });

    let sync_handle = tokio::task::spawn(async move {
        for _ in 0..5 {
            let _ = specs_adapter_clone.resync().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let winner = tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(1)) => false,
        _ = check_handle => true,
    };

    output_logger_provider
        .is_blocked
        .store(false, Ordering::SeqCst);

    assert!(output_logger_provider
        .did_block_logger
        .load(Ordering::SeqCst));

    if !winner {
        panic!("gate checks blocked by long sync times");
    }

    drop(statsig);
    sync_handle.abort();
    std::process::exit(0);
}
