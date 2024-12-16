use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::{
    DynamicConfigEvaluationOptions, ExperimentEvaluationOptions, FeatureGateEvaluationOptions,
    LayerEvaluationOptions, StatsigUser,
};

pub trait OverrideAdapter: Send + Sync {
    fn get_gate_override(
        &self,
        user: &StatsigUser,
        gate_name: &str,
        options: &FeatureGateEvaluationOptions,
        result: &mut EvaluatorResult<'_>,
    ) -> bool;

    fn get_dynamic_config_override(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
        options: &DynamicConfigEvaluationOptions,
        result: &mut EvaluatorResult<'_>,
    ) -> bool;

    fn get_experiment_override(
        &self,
        user: &StatsigUser,
        experiment_name: &str,
        options: &ExperimentEvaluationOptions,
        result: &mut EvaluatorResult<'_>,
    ) -> bool;

    fn get_layer_override(
        &self,
        user: &StatsigUser,
        layer_name: &str,
        options: &LayerEvaluationOptions,
        result: &mut EvaluatorResult<'_>,
    ) -> bool;
}
