use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::specs_response::spec_types::Spec;
use crate::StatsigUser;
use serde_json::Value;
use std::collections::HashMap;

pub trait OverrideAdapter: Send + Sync {
    fn get_gate_override(
        &self,
        user: &StatsigUser,
        gate_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool;

    fn get_dynamic_config_override(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool;

    fn get_experiment_override(
        &self,
        user: &StatsigUser,
        experiment_name: &str,
        result: &mut EvaluatorResult<'_>,
        opt_spec: Option<&Spec>,
    ) -> bool;

    fn get_layer_override(
        &self,
        user: &StatsigUser,
        layer_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool;

    fn override_gate(&self, key: &str, value: bool);
    fn override_gate_for_id(&self, key: &str, id: &str, value: bool);
    fn override_dynamic_config(&self, key: &str, value: HashMap<String, Value>);
    fn override_dynamic_config_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>);
    fn override_experiment(&self, key: &str, value: HashMap<String, Value>);
    fn override_experiment_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>);
    fn override_experiment_by_group_name(&self, key: &str, group_name: &str);
    fn override_experiment_by_group_name_for_id(&self, key: &str, id: &str, group_name: &str);
    fn override_layer(&self, key: &str, value: HashMap<String, Value>);
    fn override_layer_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>);
}
