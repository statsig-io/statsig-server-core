mod mock_specs_adapter;
mod mock_event_logging_adapter;
#[cfg(test)]
mod eval_reasons_tests {
    use super::*;
    use mock_specs_adapter::MockSpecsAdapter;
    use mock_event_logging_adapter::MockEventLoggingAdapter;

    use sigstat::{Statsig, StatsigOptions, StatsigUser};
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use tokio::time::sleep;

    async fn init_statsig(logging_adapter: &Arc<MockEventLoggingAdapter>) -> Statsig {
        let statsig = Statsig::new(
            "secret-shhh",
            Some(Arc::new(StatsigOptions {
                environment: Some("development".to_string()),
                specs_adapter: Some(Arc::new(
                    MockSpecsAdapter::with_data("tests/eval_proj_dcs.json")
                )),
                event_logging_adapter: Some(logging_adapter.clone()),
                ..StatsigOptions::new()
            })),
        );
        statsig.initialize().await.unwrap();

        statsig
    }

    #[tokio::test]
    async fn test_feature_gate_reason() {
        let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
        let user = StatsigUser::with_user_id("a_user_id".into());

        let statsig = init_statsig(&logging_adapter).await;
        let _ = statsig.check_gate(&user, "test_public");
        sleep(Duration::from_millis(1)).await;
        statsig.flush_events().await;

        assert_eq!(logging_adapter.logged_event_count.load(Ordering::SeqCst), 1);

        let event = logging_adapter.force_get_first_event().await;

        let event_name = event["eventName"].as_str().unwrap();
        assert_eq!(event_name, "statsig::gate_exposure");

        let metadata = event["metadata"].as_object().unwrap().clone();
        let rule_id = metadata["ruleID"].as_str().unwrap();
        assert_eq!(rule_id, "6X3qJgyfwA81IJ2dxI7lYp");
    }
}
