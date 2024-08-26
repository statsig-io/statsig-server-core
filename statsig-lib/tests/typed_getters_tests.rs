mod mock_specs_adapter;
mod mock_event_logging_adapter;
mod test_utils;

use sigstat::{Statsig, StatsigOptions, StatsigUser};
use std::sync::Arc;
use serde_json::json;
use crate::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::mock_specs_adapter::MockSpecsAdapter;

async fn setup() -> Statsig {
    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(Arc::new(MockSpecsAdapter::with_data("tests/big_number_dcs.json")));
    options.event_logging_adapter = Some(Arc::new(MockEventLoggingAdapter::new()));

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();
    statsig
}

#[tokio::test]
async fn test_dynamic_config_getters() {
    let statsig = setup().await;
    let user = StatsigUser::with_user_id("a_user_id".into());

    let config = statsig.get_dynamic_config(&user, "big_number");

    let expected = 0.9999999999999991;
    assert_eq!(config.get_f64("f64").unwrap(), expected);
}
