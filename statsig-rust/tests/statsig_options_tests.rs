use std::sync::Arc;

use serial_test::serial;
use statsig_rust::event_logging::event_logger;
use statsig_rust::{Statsig, StatsigOptions};

mod utils;

#[test]
#[serial]
fn test_statsig_options() {
    std::env::remove_var("STATSIG_RUNNING_TESTS");
    let mut invalid_option = StatsigOptions::new();
    invalid_option.specs_sync_interval_ms = Some(10);
    invalid_option.id_lists_sync_interval_ms = Some(10);
    invalid_option.specs_url = Some("".to_string());
    invalid_option.id_lists_url = Some("".to_string());
    invalid_option.log_event_url = Some("".to_string());
    let statsig = Statsig::new("secret-key", Some(Arc::new(invalid_option)));
    assert_eq!(statsig.options.specs_sync_interval_ms, None);
    assert_eq!(statsig.options.id_lists_sync_interval_ms, None);
    assert_eq!(statsig.options.log_event_url, None);
    assert_eq!(statsig.options.id_lists_url, None);
    assert_eq!(statsig.options.specs_url, None);
}

#[test]
#[serial]
fn test_event_logging_options_validation_clears_invalid_values() {
    std::env::remove_var("STATSIG_RUNNING_TESTS");

    let mut invalid_option = StatsigOptions::new();
    invalid_option.event_logging_max_queue_size = Some(999999999);
    invalid_option.event_logging_max_pending_batch_queue_size = Some(0);

    let statsig = Statsig::new("secret-key", Some(Arc::new(invalid_option)));
    assert_eq!(statsig.options.event_logging_max_queue_size, None);
    assert_eq!(
        statsig.options.event_logging_max_pending_batch_queue_size,
        None
    );
}

#[test]
#[serial]
fn test_event_logging_options_validation_keeps_valid_values() {
    std::env::remove_var("STATSIG_RUNNING_TESTS");

    let mut valid_option = StatsigOptions::new();
    valid_option.event_logging_max_queue_size = Some(50);
    valid_option.event_logging_max_pending_batch_queue_size = Some(99999);

    let statsig = Statsig::new("secret-key", Some(Arc::new(valid_option)));
    assert_eq!(statsig.options.event_logging_max_queue_size, Some(50));
    assert_eq!(
        statsig.options.event_logging_max_pending_batch_queue_size,
        Some(99999)
    );
}

#[test]
#[serial]
fn test_event_logging_options_validation_skipped_in_test_env() {
    std::env::set_var("STATSIG_RUNNING_TESTS", "true");

    let mut options = StatsigOptions::new();
    options.event_logging_max_queue_size = Some(event_logger::MIN_BATCH_SIZE - 1);
    options.event_logging_max_pending_batch_queue_size =
        Some(event_logger::DEFAULT_PENDING_BATCH_COUNT_MAX - 1);

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    assert_eq!(
        statsig.options.event_logging_max_queue_size,
        Some(event_logger::MIN_BATCH_SIZE - 1)
    );
    assert_eq!(
        statsig.options.event_logging_max_pending_batch_queue_size,
        Some(event_logger::DEFAULT_PENDING_BATCH_COUNT_MAX - 1)
    );
}
