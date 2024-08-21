mod mock_specs_adapter;
mod mock_event_logging_adapter;
mod test_utils;
#[cfg(test)]
mod exposure_tests {
    use super::*;
    use mock_specs_adapter::MockSpecsAdapter;
    use mock_event_logging_adapter::MockEventLoggingAdapter;

    use statsig::{Statsig, StatsigOptions, StatsigUser};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    use crate::test_utils::{enforce_array, enforce_object, enforce_string};


    #[tokio::test]
    async fn test_gate_exposures_initialized() {
        let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
        let specs_adapter = create_bootrapped_specs_adapter();
        let user = StatsigUser::with_user_id("a_user_id".into());

        let statsig = create_statsig(&specs_adapter, &logging_adapter);
        statsig.initialize().await.unwrap();

        let _ = statsig.check_gate(&user, "test_public");
        sleep(Duration::from_millis(1)).await;
        statsig.flush_events().await;

        let reason = get_reason_from_adapter(&logging_adapter).await;
        assert_eq!(reason, "Bootstrap:Recognized");
    }

    #[tokio::test]
    async fn test_gate_exposures_uninitialized() {
        let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
        let specs_adapter = create_bootrapped_specs_adapter();
        let user = StatsigUser::with_user_id("a_user_id".into());

        let statsig = create_statsig(&specs_adapter, &logging_adapter);

        let _ = statsig.check_gate(&user, "test_public");
        sleep(Duration::from_millis(1)).await;
        statsig.flush_events().await;

        let reason = get_reason_from_adapter(&logging_adapter).await;
        assert_eq!(reason, "Uninitialized");
    }

    #[tokio::test]
    async fn test_gate_exposures_unrecognized() {
        let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
        let specs_adapter = create_bootrapped_specs_adapter();
        let user = StatsigUser::with_user_id("a_user_id".into());

        let statsig = create_statsig(&specs_adapter, &logging_adapter);
        statsig.initialize().await.unwrap();

        let _ = statsig.check_gate(&user, "not_a_gate");
        sleep(Duration::from_millis(1)).await;
        statsig.flush_events().await;

        let reason = get_reason_from_adapter(&logging_adapter).await;
        assert_eq!(reason, "Bootstrap:Unrecognized");
    }


    #[tokio::test]
    async fn test_gate_exposures_bad_network() {
        let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
        let specs_adapter = create_trowing_specs_adapter();
        let user = StatsigUser::with_user_id("a_user_id".into());

        let statsig = create_statsig(&specs_adapter, &logging_adapter);
        let _ = statsig.initialize().await;

        let _ = statsig.check_gate(&user, "not_a_gate");
        sleep(Duration::from_millis(1)).await;
        statsig.flush_events().await;

        let reason = get_reason_from_adapter(&logging_adapter).await;
        assert_eq!(reason, "NoValues");
    }

    #[tokio::test]
    async fn test_gate_exposures_not_awaited() {
        let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
        let specs_adapter = create_delayed_specs_adapter();
        let user = StatsigUser::with_user_id("a_user_id".into());

        let statsig = Arc::new(create_statsig(&specs_adapter, &logging_adapter));

        let shared_statsig = statsig.clone();
        tokio::task::spawn(async move {
            shared_statsig.initialize().await.unwrap();
        });

        sleep(Duration::from_millis(1)).await;

        let _ = statsig.check_gate(&user, "not_a_gate");
        sleep(Duration::from_millis(1)).await;
        statsig.flush_events().await;

        let reason = get_reason_from_adapter(&logging_adapter).await;
        assert_eq!(reason, "Loading:Unrecognized");
    }

    #[tokio::test]
    async fn test_check_gate_exposure_with_secondary_exposures() {
        let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
        let specs_adapter = create_bootrapped_specs_adapter();
        let user = StatsigUser::with_user_id("a_user_id".into());

        let statsig = create_statsig(&specs_adapter, &logging_adapter);
        statsig.initialize().await.unwrap();

        let _ = statsig.check_gate(&user, "test_nested_gate_condition");
        sleep(Duration::from_millis(1)).await;
        statsig.flush_events().await;

        let event = logging_adapter.force_get_first_event().await;
        let secondary_expo = enforce_array(&event["secondaryExposures"]);

        let one = enforce_object(&secondary_expo[0]);
        assert_eq!(one["gate"], "test_email");
        assert_eq!(one["ruleID"], "default");
        assert_eq!(one["gateValue"], "false");

        let two = enforce_object(&secondary_expo[1]);
        assert_eq!(two["gate"], "test_environment_tier");
        assert_eq!(two["ruleID"], "default");
        assert_eq!(two["gateValue"], "false");
    }

    #[tokio::test]
    async fn test_get_feature_gate_exposure_with_secondary_exposures() {
        let logging_adapter = Arc::new(MockEventLoggingAdapter::new());
        let specs_adapter = create_bootrapped_specs_adapter();
        let user = StatsigUser::with_user_id("a_user_id".into());

        let statsig = create_statsig(&specs_adapter, &logging_adapter);
        statsig.initialize().await.unwrap();

        let _ = statsig.get_feature_gate(&user, "test_nested_gate_condition");
        sleep(Duration::from_millis(1)).await;
        statsig.flush_events().await;

        let event = logging_adapter.force_get_first_event().await;
        let secondary_expo = enforce_array(&event["secondaryExposures"]);

        let one = enforce_object(&secondary_expo[0]);
        assert_eq!(one["gate"], "test_email");
        assert_eq!(one["ruleID"], "default");
        assert_eq!(one["gateValue"], "false");

        let two = enforce_object(&secondary_expo[1]);
        assert_eq!(two["gate"], "test_environment_tier");
        assert_eq!(two["ruleID"], "default");
        assert_eq!(two["gateValue"], "false");
    }

    fn create_bootrapped_specs_adapter() -> Arc<MockSpecsAdapter> {
        Arc::new(MockSpecsAdapter::with_data("tests/eval_proj_dcs.json"))
    }

    fn create_trowing_specs_adapter() -> Arc<MockSpecsAdapter> {
        Arc::new(MockSpecsAdapter::throwing())
    }

    fn create_delayed_specs_adapter() -> Arc<MockSpecsAdapter> {
        Arc::new(MockSpecsAdapter::delayed("tests/eval_proj_dcs.json", 100))
    }

    fn create_statsig(
        specs_adapter: &Arc<MockSpecsAdapter>,
        logging_adapter: &Arc<MockEventLoggingAdapter>,
    ) -> Statsig {
        

        Statsig::new(
            "secret-shhh",
            Some(Arc::new(StatsigOptions {
                specs_adapter: Some(specs_adapter.clone()),
                event_logging_adapter: Some(logging_adapter.clone()),
                ..StatsigOptions::new()
            })),
        )
    }

    async fn get_reason_from_adapter(logging_adapter: &MockEventLoggingAdapter) -> String {
        let event = logging_adapter.force_get_first_event().await;
        let metadata = enforce_object(&event["metadata"]);
        
        enforce_string(&metadata["reason"])
    }
}
