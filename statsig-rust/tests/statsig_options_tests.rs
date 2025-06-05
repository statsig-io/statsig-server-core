use std::sync::Arc;

use statsig_rust::{Statsig, StatsigOptions};

mod utils;

#[test]
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
