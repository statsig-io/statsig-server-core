mod utils;

use parking_lot::Mutex;
use serial_test::serial;
use statsig_rust::output_logger::{initialize_output_logger, shutdown_output_logger, LogLevel};
use statsig_rust::{log_d, log_e, log_i, log_w, Statsig, StatsigOptions};
use std::sync::Arc;
use std::time::Duration;
use utils::mock_log_provider::{MockLogProvider, RecordedLog};

use crate::utils::mock_scrapi::{Endpoint, MockScrapi};

#[test]
#[serial]
fn test_custom_log_provider() {
    let provider = Arc::new(MockLogProvider {
        logs: Mutex::new(Vec::new()),
    });

    initialize_output_logger(&Some(LogLevel::Debug), Some(provider.clone()));

    let test_tag = "test_tag";

    log_d!(test_tag, "debug message");
    log_i!(test_tag, "info message");
    log_w!(test_tag, "warn message");
    log_e!(test_tag, "error message");

    shutdown_output_logger();

    let logs = provider.logs.try_lock_for(Duration::from_secs(5)).unwrap();
    assert_eq!(logs.len(), 6);

    assert_eq!(logs[0], RecordedLog::Init);
    assert_eq!(
        logs[1],
        RecordedLog::Debug(test_tag.to_string(), "debug message".to_string())
    );
    assert_eq!(
        logs[2],
        RecordedLog::Info(test_tag.to_string(), "info message".to_string())
    );
    assert_eq!(
        logs[3],
        RecordedLog::Warn(test_tag.to_string(), "warn message".to_string())
    );
    assert_eq!(
        logs[4],
        RecordedLog::Error(test_tag.to_string(), "error message".to_string())
    );
    assert_eq!(logs[5], RecordedLog::Shutdown);
}

#[test]
#[serial]
fn test_log_level_filtering() {
    let provider = Arc::new(MockLogProvider {
        logs: Mutex::new(Vec::new()),
    });

    initialize_output_logger(&Some(LogLevel::Warn), Some(provider.clone()));

    let test_tag = "test_tag";

    log_d!(test_tag, "debug message");
    log_i!(test_tag, "info message");
    log_w!(test_tag, "warn message");
    log_e!(test_tag, "error message");

    shutdown_output_logger();

    let logs = provider.logs.try_lock_for(Duration::from_secs(5)).unwrap();
    assert_eq!(logs.len(), 4); // Init + Warn + Error + Shutdown

    assert_eq!(logs[0], RecordedLog::Init);
    assert_eq!(
        logs[1],
        RecordedLog::Warn(test_tag.to_string(), "warn message".to_string())
    );
    assert_eq!(
        logs[2],
        RecordedLog::Error(test_tag.to_string(), "error message".to_string())
    );
    assert_eq!(logs[3], RecordedLog::Shutdown);
}

#[test]
#[serial]
fn test_message_truncation() {
    let provider = Arc::new(MockLogProvider {
        logs: Mutex::new(Vec::new()),
    });

    initialize_output_logger(&Some(LogLevel::Debug), Some(provider.clone()));

    let test_tag = "test_tag";
    let long_message = "x".repeat(500);
    log_d!(test_tag, "{}", long_message);

    let logs = {
        let mut guard = provider.logs.try_lock_for(Duration::from_secs(5)).unwrap();
        std::mem::take(&mut *guard)
    };
    assert_eq!(logs.len(), 2);

    if let RecordedLog::Debug(_, msg) = &logs[1] {
        assert!(msg.len() <= 400 + 13);
        assert!(msg.ends_with("...[TRUNCATED]"));
    } else {
        panic!("Expected Debug log level");
    }

    shutdown_output_logger();
}

#[test]
#[serial]
fn test_secret_sanitization() {
    let provider = Arc::new(MockLogProvider {
        logs: Mutex::new(Vec::new()),
    });

    initialize_output_logger(&Some(LogLevel::Debug), Some(provider.clone()));

    let test_tag = "test_tag";
    let message = "secret-key12345 and secret-abcde";
    log_d!(test_tag, "{}", message);

    let logs = {
        let mut guard = provider.logs.try_lock_for(Duration::from_secs(5)).unwrap();
        std::mem::take(&mut *guard)
    };
    assert_eq!(logs.len(), 2); // Init + Debug

    if let RecordedLog::Debug(_, msg) = &logs[1] {
        assert_eq!(msg, "secret-key12***** and secret-abcde*****");
    } else {
        panic!("Expected Debug log level");
    }

    shutdown_output_logger();
}

#[tokio::test]
async fn test_default_logger_no_error_on_multiple_instances() {
    let mock_scrapi = MockScrapi::new().await;
    let options = StatsigOptions {
        log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
        specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
        ..StatsigOptions::new()
    };

    // checking for uncaught panics
    let statsig1 = Statsig::new("secret-key12345", Some(Arc::new(options.clone())));
    let statsig2 = Statsig::new("secret-key67890", Some(Arc::new(options)));

    let _ = statsig1.initialize().await;
    let _ = statsig2.initialize().await;

    let _ = statsig1.shutdown().await;
    let _ = statsig2.shutdown().await;
}

#[tokio::test]
async fn test_custom_logger_no_error_on_multiple_instances() {
    // checking for uncaught panics
    let provider = Arc::new(MockLogProvider {
        logs: Mutex::new(Vec::new()),
    });

    let provider2 = provider.clone();

    let mock_scrapi = MockScrapi::new().await;
    let mut options1 = StatsigOptions {
        log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
        specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
        ..StatsigOptions::new()
    };
    options1.output_logger_provider = Some(provider.clone());

    let mut options2 = StatsigOptions {
        log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
        specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
        ..StatsigOptions::new()
    };
    options2.output_logger_provider = Some(provider2.clone());

    let statsig1 = Statsig::new("secret-key12345", Some(Arc::new(options1)));
    let statsig2 = Statsig::new("secret-key67890", Some(Arc::new(options2)));

    let _ = statsig1.initialize().await;
    let _ = statsig2.initialize().await;

    let _ = statsig1.shutdown().await;
    let _ = statsig2.shutdown().await;
}
