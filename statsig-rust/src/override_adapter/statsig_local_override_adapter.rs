use crate::{log_d, log_e, read_lock_or_return, write_lock_or_noop, OverrideAdapter, StatsigUser};
use std::{collections::HashMap, sync::RwLock};

use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::specs_response::spec_types::Spec;
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
const NO_ID_OVERRIDE: &str = "__STATSIG_NO_ID__";

lazy_static::lazy_static! {
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
        user: &StatsigUser,
        gate_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        let gate_overrides = match store.gate.get(gate_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        log_d!(TAG, "gate_overrides found for {}", gate_name);

        find_override_for_user(
            user,
            gate_overrides,
            |value, res| {
                res.bool_value = *value;
            },
            result,
        )
    }

    fn get_dynamic_config_override(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        let config_overrides = match store.config.get(dynamic_config_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        find_override_for_user(
            user,
            config_overrides,
            |value, res| {
                res.json_value = Some(value.clone());
            },
            result,
        )
    }

    fn get_experiment_override(
        &self,
        user: &StatsigUser,
        experiment_name: &str,
        result: &mut EvaluatorResult<'_>,
        opt_spec: Option<&Spec>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        let experiment_overrides = match store.experiment.get(experiment_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        find_override_for_user(
            user,
            experiment_overrides,
            |override_value, res| match override_value {
                ExperimentOverrides::Value(map) => {
                    res.json_value = Some(map.clone());
                }
                ExperimentOverrides::GroupName(group_name) => {
                    res.json_value = get_experiment_with_group_name(opt_spec, group_name);
                }
            },
            result,
        )
    }

    fn get_layer_override(
        &self,
        user: &StatsigUser,
        layer_name: &str,
        result: &mut EvaluatorResult<'_>,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        let layer_overrides = match store.layer.get(layer_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        find_override_for_user(
            user,
            layer_overrides,
            |value, res| {
                res.json_value = Some(value.clone());
            },
            result,
        )
    }

    fn override_gate(&self, key: &str, value: bool) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store
            .gate
            .entry(key.to_string())
            .or_default()
            .insert(NO_ID_OVERRIDE.to_string(), value);
    }

    fn override_gate_for_id(&self, key: &str, id: &str, value: bool) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store
            .gate
            .entry(key.to_string())
            .or_default()
            .insert(id.to_string(), value);
    }

    fn override_dynamic_config(&self, key: &str, value: HashMap<String, Value>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store
            .config
            .entry(key.to_string())
            .or_default()
            .insert(NO_ID_OVERRIDE.to_string(), value);
    }

    fn override_dynamic_config_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store
            .config
            .entry(key.to_string())
            .or_default()
            .insert(id.to_string(), value);
    }

    fn override_experiment(&self, key: &str, value: HashMap<String, Value>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store.experiment.entry(key.to_string()).or_default().insert(
            NO_ID_OVERRIDE.to_string(),
            ExperimentOverrides::Value(value),
        );
    }

    fn override_experiment_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store
            .experiment
            .entry(key.to_string())
            .or_default()
            .insert(id.to_string(), ExperimentOverrides::Value(value));
    }

    fn override_experiment_by_group_name(&self, key: &str, group_name: &str) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store.experiment.entry(key.to_string()).or_default().insert(
            NO_ID_OVERRIDE.to_string(),
            ExperimentOverrides::GroupName(group_name.to_string()),
        );
    }

    fn override_experiment_by_group_name_for_id(&self, key: &str, id: &str, group_name: &str) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store.experiment.entry(key.to_string()).or_default().insert(
            id.to_string(),
            ExperimentOverrides::GroupName(group_name.to_string()),
        );
    }

    fn override_layer(&self, key: &str, value: HashMap<String, Value>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store
            .layer
            .entry(key.to_string())
            .or_default()
            .insert(NO_ID_OVERRIDE.to_string(), value);
    }

    fn override_layer_for_id(&self, key: &str, id: &str, value: HashMap<String, Value>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store
            .layer
            .entry(key.to_string())
            .or_default()
            .insert(id.to_string(), value);
    }
}

fn find_override_for_user<T, F>(
    user: &StatsigUser,
    overrides: &HashMap<String, T>,
    apply_override: F,
    result: &mut EvaluatorResult<'_>,
) -> bool
where
    F: Fn(&T, &mut EvaluatorResult<'_>),
{
    if check_user_id_override(user, overrides, &apply_override, result) {
        return true;
    }

    if check_custom_ids_override(user, overrides, &apply_override, result) {
        return true;
    }

    check_default_override(overrides, apply_override, result)
}

fn mark_result_as_override(result: &mut EvaluatorResult<'_>) {
    result.override_reason = Some(LOCAL_OVERRIDE_REASON);
    result.rule_id = Some(&OVERRIDE_RULE_ID);
}

fn check_default_override<T, F>(
    overrides: &HashMap<String, T>,
    apply_override: F,
    result: &mut EvaluatorResult<'_>,
) -> bool
where
    F: Fn(&T, &mut EvaluatorResult<'_>),
{
    if let Some(override_value) = overrides.get(NO_ID_OVERRIDE) {
        log_d!(TAG, "default override found");
        apply_override(override_value, result);
        mark_result_as_override(result);
        return true;
    }
    false
}

fn check_user_id_override<T, F>(
    user: &StatsigUser,
    overrides: &HashMap<String, T>,
    apply_override: F,
    result: &mut EvaluatorResult<'_>,
) -> bool
where
    F: Fn(&T, &mut EvaluatorResult<'_>),
{
    let user_id = match &user.user_id {
        Some(id) => id,
        None => return false,
    };

    let id_string = match &user_id.string_value {
        Some(s) => &s.value,
        None => return false,
    };

    let override_value = match overrides.get(id_string) {
        Some(v) => v,
        None => return false,
    };

    log_d!(TAG, "override found for user ID {}", id_string);
    apply_override(override_value, result);
    mark_result_as_override(result);
    true
}

fn check_custom_ids_override<T, F>(
    user: &StatsigUser,
    overrides: &HashMap<String, T>,
    apply_override: F,
    result: &mut EvaluatorResult<'_>,
) -> bool
where
    F: Fn(&T, &mut EvaluatorResult<'_>),
{
    let custom_ids = match &user.custom_ids {
        Some(ids) => ids,
        None => return false,
    };

    for custom_id_value in custom_ids.values() {
        let id_string = match &custom_id_value.string_value {
            Some(s) => &s.value,
            None => continue,
        };

        let override_value = match overrides.get(id_string) {
            Some(v) => v,
            None => continue,
        };

        log_d!(TAG, "override found for custom ID {}", id_string);
        apply_override(override_value, result);
        mark_result_as_override(result);
        return true;
    }

    false
}

fn get_experiment_with_group_name(
    opt_spec: Option<&Spec>,
    group_name: &str,
) -> Option<HashMap<String, Value>> {
    let spec = opt_spec?;

    for rule in &spec.rules {
        let rule_group_name = match &rule.group_name {
            Some(rule_group_name) => rule_group_name,
            None => continue,
        };

        if rule_group_name == group_name {
            return rule.return_value.get_json();
        }
    }

    None
}
