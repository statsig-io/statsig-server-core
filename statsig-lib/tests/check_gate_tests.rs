mod mock_specs_adapter;
#[cfg(test)]
mod check_gate_tests {
    use super::*;
    use mock_specs_adapter::MockSpecsAdapter;
    use lazy_static::lazy_static;
    use sigstat::{dyn_value, DynamicValue, Statsig, StatsigOptions, StatsigUser};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;

    lazy_static! {
        static ref USER: StatsigUser = StatsigUser {
            custom: Some(HashMap::from([
                ("flavor".to_string(), dyn_value!("chocolate")),
                ("region".to_string(), dyn_value!("somewhere")),
                ("service".to_string(), dyn_value!("some-service")),
            ])),
            ..StatsigUser::with_custom_ids(HashMap::from([(
                "hubID".to_string(),
                "some-service-123".to_string()
            )]))
        };
    }

    const SDK_KEY: &str = "secret-key";

    #[tokio::test]
    async fn test_many_rule_perf() {
        let user = USER.clone();

        let statsig = Statsig::new(
            SDK_KEY,
            Some(Arc::new(StatsigOptions {
                environment: Some("staging".to_string()),
                specs_adapter: Some(Arc::new(
                    MockSpecsAdapter::with_data("tests/check_gate_perf_dcs.json")
                )),
                ..StatsigOptions::new()
            })),
        );
        statsig.initialize().await.unwrap();

        let start = Instant::now();
        let mut result = false;
        for _ in 0..1000 {
            result = statsig.check_gate(&user, "many_rules");
        }

        let duration = start.elapsed();
        println!("many_rules: {:.2} ms", duration.as_secs_f64() * 1000.0);

        let _ = statsig.shutdown().await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_single_rule_env_perf() {
        let user = USER.clone();

        let statsig = Statsig::new(
            SDK_KEY,
            Some(Arc::new(StatsigOptions {
                environment: Some("development".to_string()),
                specs_adapter: Some(Arc::new(
                    MockSpecsAdapter::with_data("tests/check_gate_perf_dcs.json")
                )),
                ..StatsigOptions::new()
            })),
        );
        statsig.initialize().await.unwrap();

        let start = Instant::now();
        let mut result = false;
        for _ in 0..1000 {
            result = statsig.check_gate(&user, "public_dev_only");
        }

        let duration = start.elapsed();
        println!("public_dev_only: {:.2} ms", duration.as_secs_f64() * 1000.0);

        let _ = statsig.shutdown().await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_no_gate_perf() {
        let user = USER.clone();

        let statsig = Statsig::new(
            SDK_KEY,
            Some(Arc::new(StatsigOptions {
                environment: Some("development".to_string()),
                specs_adapter: Some(Arc::new(
                    MockSpecsAdapter::with_data("tests/check_gate_perf_dcs.json")
                )),
                ..StatsigOptions::new()
            })),
        );
        statsig.initialize().await.unwrap();

        let start = Instant::now();
        let mut result = false;
        for _ in 0..1000 {
            result = statsig.check_gate(&user, "not_a_gate");
        }

        let duration = start.elapsed();
        println!("public_dev_only: {:.2} ms", duration.as_secs_f64() * 1000.0);

        let _ = statsig.shutdown().await;

        assert!(!result);
    }
}
