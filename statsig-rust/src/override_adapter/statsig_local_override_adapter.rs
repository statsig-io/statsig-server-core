use crate::{log_d, log_e, read_lock_or_return, OverrideAdapter, StatsigUser};
use std::{collections::HashMap, sync::RwLock};

use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::specs_response::spec_types::Spec;
use lazy_static::lazy_static;
use serde_json::Value;

enum ExperimentOverrides {
    Value(HashMap<String, Value>),
    GroupName(String),
}

#[derive(Default)]
struct OverrideStore {
    pub gate: HashMap<String, HashMap<String, bool>>,
    pub config: HashMap<String, HashMap<String, HashMap<String, Value>>>,
    pub experiment: HashMap<String, HashMap<String, ExperimentOverrides>>,
    pub layer: HashMap<String, HashMap<String, HashMap<String, Value>>>,
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
    #[must_use]
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

        let gate_overrides = match store.gate.get(gate_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        log_d!(TAG, "gate_overrides found for {}", gate_name);

        if let Some(user_id) = &_user.user_id {
            if let Some(id_string) = &user_id.string_value {
                if let Some(override_value) = gate_overrides.get(id_string) {
                    log_d!(TAG, "gate_overrides found for {}", id_string);
                    result.bool_value = *override_value;
                    result.override_reason = Some(LOCAL_OVERRIDE_REASON);
                    result.rule_id = Some(&OVERRIDE_RULE_ID);
                    return true;
                }
            }
        }

        if let Some(custom_ids) = &_user.custom_ids {
            for custom_id_value in custom_ids.values() {
                log_d!(TAG, "looking for gate_overrides for {:?}", custom_id_value);
                if let Some(id_string) = &custom_id_value.string_value {
                    log_d!(TAG, "gate_overrides found for {}", id_string);
                    if let Some(override_value) = gate_overrides.get(id_string) {
                        result.bool_value = *override_value;
                        result.override_reason = Some(LOCAL_OVERRIDE_REASON);
                        result.rule_id = Some(&OVERRIDE_RULE_ID);
                        return true;
                    }
                }
            }
        }

        log_d!(TAG, "looking for gate_overrides for empty string");
        if let Some(override_value) = gate_overrides.get("") {
            log_d!(TAG, "gate_overrides found for empty string");
            result.bool_value = *override_value;
            result.override_reason = Some(LOCAL_OVERRIDE_REASON);
            result.rule_id = Some(&OVERRIDE_RULE_ID);
            return true;
        }

        log_d!(TAG, "no gate_overrides found");
        false
    }

    fn get_dynamic_config_override(
        &self,
        _user: &StatsigUser,
        dynamic_config_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        let config_overrides = match store.config.get(dynamic_config_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        if let Some(user_id) = &_user.user_id {
            if let Some(id_string) = &user_id.string_value {
                if let Some(override_value) = config_overrides.get(id_string) {
                    result.json_value = Some(override_value.clone());
                    result.override_reason = Some(LOCAL_OVERRIDE_REASON);
                    result.rule_id = Some(&OVERRIDE_RULE_ID);
                    return true;
                }
            }
        }

        if let Some(custom_ids) = &_user.custom_ids {
            for custom_id_value in custom_ids.values() {
                if let Some(id_string) = &custom_id_value.string_value {
                    if let Some(override_value) = config_overrides.get(id_string) {
                        result.json_value = Some(override_value.clone());
                        result.override_reason = Some(LOCAL_OVERRIDE_REASON);
                        result.rule_id = Some(&OVERRIDE_RULE_ID);
                        return true;
                    }
                }
            }
        }

        if let Some(override_value) = config_overrides.get("") {
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
        opt_spec: Option<&Spec>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        let experiment_overrides = match store.experiment.get(experiment_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        if let Some(user_id) = &_user.user_id {
            if let Some(id_string) = &user_id.string_value {
                if let Some(override_value) = experiment_overrides.get(id_string) {
                    match override_value {
                        ExperimentOverrides::Value(map) => {
                            result.json_value = Some(map.clone());
                        }
                        ExperimentOverrides::GroupName(group_name) => {
                            result.json_value =
                                self.get_experiment_with_group_name(opt_spec, group_name);
                        }
                    }
                    result.override_reason = Some(LOCAL_OVERRIDE_REASON);
                    result.rule_id = Some(&OVERRIDE_RULE_ID);
                    return true;
                }
            }
        }

        if let Some(custom_ids) = &_user.custom_ids {
            for custom_id_value in custom_ids.values() {
                if let Some(id_string) = &custom_id_value.string_value {
                    if let Some(override_value) = experiment_overrides.get(id_string) {
                        match override_value {
                            ExperimentOverrides::Value(map) => {
                                result.json_value = Some(map.clone());
                            }
                            ExperimentOverrides::GroupName(group_name) => {
                                result.json_value =
                                    self.get_experiment_with_group_name(opt_spec, group_name);
                            }
                        }
                        result.override_reason = Some(LOCAL_OVERRIDE_REASON);
                        result.rule_id = Some(&OVERRIDE_RULE_ID);
                        return true;
                    }
                }
            }
        }

        if let Some(override_value) = experiment_overrides.get("") {
            match override_value {
                ExperimentOverrides::Value(map) => {
                    result.json_value = Some(map.clone());
                }
                ExperimentOverrides::GroupName(group_name) => {
                    result.json_value = self.get_experiment_with_group_name(opt_spec, group_name);
                }
            }
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

        let layer_overrides = match store.layer.get(layer_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        if let Some(user_id) = &_user.user_id {
            if let Some(id_string) = &user_id.string_value {
                if let Some(override_value) = layer_overrides.get(id_string) {
                    result.json_value = Some(override_value.clone());
                    result.override_reason = Some(LOCAL_OVERRIDE_REASON);
                    result.rule_id = Some(&OVERRIDE_RULE_ID);
                    return true;
                }
            }
        }

        if let Some(custom_ids) = &_user.custom_ids {
            for custom_id_value in custom_ids.values() {
                if let Some(id_string) = &custom_id_value.string_value {
                    if let Some(override_value) = layer_overrides.get(id_string) {
                        result.json_value = Some(override_value.clone());
                        result.override_reason = Some(LOCAL_OVERRIDE_REASON);
                        result.rule_id = Some(&OVERRIDE_RULE_ID);
                        return true;
                    }
                }
            }
        }

        if let Some(override_value) = layer_overrides.get("") {
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
                store
                    .gate
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert("".to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set gate override: {}", e.to_string()),
        }
    }

    fn override_gate_for_id(&self, key: &str, id: &str, value: bool) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .gate
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(id.to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set gate override for ID: {}", e.to_string()),
        }
    }

    fn override_dynamic_config(&self, key: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .config
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert("".to_string(), value);
            }
            Err(e) => log_e!(
                TAG,
                "Failed to set dynamic config override: {}",
                e.to_string()
            ),
        }
    }

    fn override_dynamic_config_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .config
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(id.to_string(), value);
            }
            Err(e) => log_e!(
                TAG,
                "Failed to set dynamic config override for ID: {}",
                e.to_string()
            ),
        }
    }

    fn override_experiment(&self, key: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .experiment
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert("".to_string(), ExperimentOverrides::Value(value));
            }
            Err(e) => log_e!(TAG, "Failed to set experiment override: {}", e.to_string()),
        }
    }

    fn override_experiment_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .experiment
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(id.to_string(), ExperimentOverrides::Value(value));
            }
            Err(e) => log_e!(
                TAG,
                "Failed to set experiment override for ID: {}",
                e.to_string()
            ),
        }
    }

    fn override_experiment_by_group_name(&self, key: &str, group_name: &str) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .experiment
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(
                        "".to_string(),
                        ExperimentOverrides::GroupName(group_name.to_string()),
                    );
            }
            Err(e) => log_e!(
                TAG,
                "Failed to set experiment override by group name: {}",
                e.to_string()
            ),
        }
    }

    fn override_experiment_by_group_name_for_id(&self, key: &str, id: &str, group_name: &str) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .experiment
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(
                        id.to_string(),
                        ExperimentOverrides::GroupName(group_name.to_string()),
                    );
            }
            Err(e) => log_e!(
                TAG,
                "Failed to set experiment override by group name for ID: {}",
                e.to_string()
            ),
        }
    }

    fn override_layer(&self, key: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .layer
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert("".to_string(), value);
            }
            Err(e) => log_e!(TAG, "Failed to set layer override: {}", e.to_string()),
        }
    }

    fn override_layer_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>) {
        match self.store.write() {
            Ok(mut store) => {
                store
                    .layer
                    .entry(key.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(id.to_string(), value);
            }
            Err(e) => log_e!(
                TAG,
                "Failed to set layer override for ID: {}",
                e.to_string()
            ),
        }
    }
}

impl StatsigLocalOverrideAdapter {
    fn get_experiment_with_group_name(
        &self,
        opt_spec: Option<&Spec>,
        group_name: &str,
    ) -> Option<HashMap<String, Value>> {
        if let Some(spec) = opt_spec {
            for rule in &spec.rules {
                if let Some(rule_group_name) = &rule.group_name {
                    if rule_group_name == group_name {
                        return rule.return_value.get_json();
                    }
                }
            }
        }
        None
    }
}
