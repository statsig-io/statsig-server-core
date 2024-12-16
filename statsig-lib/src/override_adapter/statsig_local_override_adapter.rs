use crate::{
    log_e, read_lock_or_return, DynamicConfigEvaluationOptions, ExperimentEvaluationOptions,
    FeatureGateEvaluationOptions, LayerEvaluationOptions, OverrideAdapter, StatsigUser,
};
use std::{collections::HashMap, sync::RwLock};

use crate::evaluation::evaluator_result::EvaluatorResult;
use serde_json::Value;

#[derive(Default)]
struct OverrideStore {
    pub gate: HashMap<String, bool>,
    pub config: HashMap<String, HashMap<String, Value>>,
    pub experiment: HashMap<String, HashMap<String, Value>>,
    pub layer: HashMap<String, HashMap<String, Value>>,
}

const TAG: &str = stringify!(StatsigLocalOverrideAdapter);
const LOCAL_OVERRIDE_REASON: &str = "LocalOverride";

#[derive(Default)]
pub struct StatsigLocalOverrideAdapter {
    store: RwLock<OverrideStore>,
}

impl StatsigLocalOverrideAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn override_gate(&self, key: &str, value: bool) {
        match self.store.write() {
            Ok(mut store) => {
                store.gate.insert(key.to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set gate override: {}", e.to_string()),
        }
    }

    pub fn override_dynamic_config(&self, key: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store.config.insert(key.to_string(), value);
            }
            Err(e) => log_e!(
                TAG,
                "Failed to set dynamic config override: {}",
                e.to_string()
            ),
        }
    }

    pub fn override_experiment(&self, key: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store.experiment.insert(key.to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set experiment override: {}", e.to_string()),
        }
    }

    pub fn override_layer(&self, key: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store.layer.insert(key.to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set layer override: {}", e.to_string()),
        }
    }
}

impl OverrideAdapter for StatsigLocalOverrideAdapter {
    fn get_gate_override(
        &self,
        _user: &StatsigUser,
        gate_name: &str,
        _options: &FeatureGateEvaluationOptions,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        if let Some(override_value) = store.gate.get(gate_name) {
            result.bool_value = *override_value;
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            return true;
        }

        false
    }

    fn get_dynamic_config_override(
        &self,
        _user: &StatsigUser,
        dynamic_config_name: &str,
        _options: &DynamicConfigEvaluationOptions,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        if let Some(override_value) = store.config.get(dynamic_config_name) {
            result.json_value = Some(override_value.clone());
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            return true;
        }

        false
    }

    fn get_experiment_override(
        &self,
        _user: &StatsigUser,
        experiment_name: &str,
        _options: &ExperimentEvaluationOptions,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        if let Some(override_value) = store.experiment.get(experiment_name) {
            result.json_value = Some(override_value.clone());
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            return true;
        }

        false
    }

    fn get_layer_override(
        &self,
        _user: &StatsigUser,
        layer_name: &str,
        _options: &LayerEvaluationOptions,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        if let Some(override_value) = store.layer.get(layer_name) {
            result.json_value = Some(override_value.clone());
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            return true;
        }

        false
    }
}
