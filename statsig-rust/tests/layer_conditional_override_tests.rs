mod utils;

use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::sync::Arc;

async fn setup() -> Statsig {
    let options = StatsigOptions {
        specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
            "tests/data/layer_conditional_override_dcs.json",
        ))),
        event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
        ..StatsigOptions::default()
    };

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();
    statsig
}

/// Reproduces a bug where layer conditional overrides (the `overrides` /
/// `override_rules` fields in the DCS response) fail to resolve when the
/// override's `new_config_name` refers to an experiment (stored in
/// `dynamic_configs`).
///
/// `try_apply_config_mapping` passes the caller's `spec_type` through to
/// `Evaluator::evaluate`. When the original evaluation is for a *layer*
/// (`SpecType::Layer`), the recursive call looks up `new_config_name` in
/// `layer_configs` instead of `dynamic_configs`, so the experiment is never
/// found and the override silently fails.
///
/// The test sets up:
///   - A layer `test_layer` that normally delegates to `test_experiment`
///     (which returns `pricing_treatment_group = "control"`)
///   - A conditional override on the layer that, when matched, should
///     redirect to `test_experiment_treatment`
///     (which returns `pricing_treatment_group = "treatment"`)
///   - The override rule uses `always_pass_gate` (a public gate) so
///     the condition always passes — isolating the lookup bug from any
///     segment/ID-list issues.
///
/// Expected: the override fires and the user gets "treatment".
/// Actual (buggy): the override is silently skipped and the user gets
/// "control" from normal layer delegation.
#[tokio::test]
async fn test_layer_conditional_override_resolves_experiment() {
    let statsig = setup().await;
    let user = StatsigUser::with_user_id("test_user".to_string());

    let layer = statsig.get_layer(&user, "test_layer");

    let treatment_group = layer.get::<String>("pricing_treatment_group", "".to_string());

    // The conditional override should fire (the gate always passes) and
    // redirect to test_experiment_treatment which returns "treatment".
    // Due to the spec_type bug, the experiment lookup fails silently and
    // we fall through to the normal delegation which returns "control".
    assert_eq!(
        treatment_group, "treatment",
        "Layer conditional override should resolve the experiment from dynamic_configs. \
         Got 'control' instead, which means try_apply_config_mapping failed to find \
         new_config_name='test_experiment_treatment' because it was looked up in \
         layer_configs (SpecType::Layer) instead of dynamic_configs (SpecType::Experiment)."
    );

    statsig.shutdown().await.unwrap();
}
