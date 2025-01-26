use crate::{log_e, read_lock_or_return, OverrideAdapter, StatsigUser};
use std::{collections::HashMap, sync::RwLock};

use crate::evaluation::evaluator_result::EvaluatorResult;
use lazy_static::lazy_static;
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
lazy_static! {
    static ref OVERRIDE_RULE_ID: String = "override".to_string();
}

#[derive(Default)]
pub struct StatsigLocalOverrideAdapter {
    store: RwLock<OverrideStore>,
}

impl StatsigLocalOverrideAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OverrideAdapter for StatsigLocalOverrideAdapter {
    fn get_gate_override(
        &self,
        _user: &StatsigUser,
        gate_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);
        if let Some(override_value) = store.gate.get(gate_name) {
            result.bool_value = *override_value;
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            result.rule_id = Some(&OVERRIDE_RULE_ID);
            return true;
        }

        false
    }

    fn get_dynamic_config_override(
        &self,
        _user: &StatsigUser,
        dynamic_config_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        if let Some(override_value) = store.config.get(dynamic_config_name) {
            result.json_value = Some(override_value.clone());
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            result.rule_id = Some(&OVERRIDE_RULE_ID);
            return true;
        }

        false
    }

    fn get_experiment_override(
        &self,
        _user: &StatsigUser,
        experiment_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        if let Some(override_value) = store.experiment.get(experiment_name) {
            result.json_value = Some(override_value.clone());
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            result.rule_id = Some(&OVERRIDE_RULE_ID);
            return true;
        }

        false
    }

    fn get_layer_override(
        &self,
        _user: &StatsigUser,
        layer_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        if let Some(override_value) = store.layer.get(layer_name) {
            result.json_value = Some(override_value.clone());
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            result.rule_id = Some(&OVERRIDE_RULE_ID);
            return true;
        }

        false
    }

    fn override_gate(&self, key: &str, value: bool) {
        match self.store.write() {
            Ok(mut store) => {
                store.gate.insert(key.to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set gate override: {}", e.to_string()),
        }
    }

    fn override_dynamic_config(&self, key: &str, value: HashMap<String, Value>) {
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

    fn override_experiment(&self, key: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store.experiment.insert(key.to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set experiment override: {}", e.to_string()),
        }
    }

    fn override_layer(&self, key: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store.layer.insert(key.to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set layer override: {}", e.to_string()),
        }
    }
}
