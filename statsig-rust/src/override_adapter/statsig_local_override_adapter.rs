use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluator_result::EvaluatorResult;
use crate::interned_string::InternedString;
use crate::specs_response::param_store_types::{Parameter, StaticValueParameter};
use crate::specs_response::spec_types::Spec;
use crate::{log_d, read_lock_or_return, write_lock_or_noop, OverrideAdapter, StatsigUser};
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;

enum ExperimentOverrides {
    Value(DynamicReturnable),
    GroupName(String),
}

#[derive(Default)]
struct OverrideStore {
    pub gate: HashMap<String, HashMap<String, bool>>,
    pub config: HashMap<String, HashMap<String, DynamicReturnable>>,
    pub experiment: HashMap<String, HashMap<String, ExperimentOverrides>>,
    pub layer: HashMap<String, HashMap<String, DynamicReturnable>>,
    pub parameter_store: HashMap<String, HashMap<String, HashMap<String, Parameter>>>,
}

const TAG: &str = stringify!(StatsigLocalOverrideAdapter);
const LOCAL_OVERRIDE_REASON: &str = "LocalOverride";
const NO_ID_OVERRIDE: &str = "__STATSIG_NO_ID__";

lazy_static::lazy_static! {
    static ref OVERRIDE_RULE_ID: InternedString = InternedString::from_str_ref("override");
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
        result: &mut EvaluatorResult,
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
        result: &mut EvaluatorResult,
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
        result: &mut EvaluatorResult,
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
        result: &mut EvaluatorResult,
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

    fn get_parameter_store_override(
        &self,
        user: &StatsigUser,
        parameter_store_name: &str,
        result: &mut EvaluatorResult,
    ) -> bool {
        let store = read_lock_or_return!(TAG, self.store, false);

        let parameter_store_overrides = match store.parameter_store.get(parameter_store_name) {
            Some(overrides) => overrides,
            None => return false,
        };

        find_override_for_user(
            user,
            parameter_store_overrides,
            |value, res| {
                let mut serialized = HashMap::with_capacity(value.len());
                for (param_name, parameter) in value.iter() {
                    if let Ok(serialized_param) = serde_json::to_value(parameter) {
                        serialized.insert(param_name.clone(), serialized_param);
                    }
                }
                res.json_value = Some(DynamicReturnable::from_map(serialized));
            },
            result,
        )
    }

    fn override_gate(&self, key: &str, value: bool, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        let id_str = id.unwrap_or(NO_ID_OVERRIDE);
        store
            .gate
            .entry(key.to_string())
            .or_default()
            .insert(id_str.to_string(), value);
    }

    fn override_dynamic_config(&self, key: &str, value: HashMap<String, Value>, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        let id_str = id.unwrap_or(NO_ID_OVERRIDE);
        store
            .config
            .entry(key.to_string())
            .or_default()
            .insert(id_str.to_string(), DynamicReturnable::from_map(value));
    }

    fn override_experiment(&self, key: &str, value: HashMap<String, Value>, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        let id_str = id.unwrap_or(NO_ID_OVERRIDE);
        store.experiment.entry(key.to_string()).or_default().insert(
            id_str.to_string(),
            ExperimentOverrides::Value(DynamicReturnable::from_map(value)),
        );
    }

    fn override_experiment_by_group_name(&self, key: &str, group_name: &str, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        let id_str = id.unwrap_or(NO_ID_OVERRIDE);
        store.experiment.entry(key.to_string()).or_default().insert(
            id_str.to_string(),
            ExperimentOverrides::GroupName(group_name.to_string()),
        );
    }

    fn override_layer(&self, key: &str, value: HashMap<String, Value>, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        let id_str = id.unwrap_or(NO_ID_OVERRIDE);
        store
            .layer
            .entry(key.to_string())
            .or_default()
            .insert(id_str.to_string(), DynamicReturnable::from_map(value));
    }

    fn override_parameter_store(&self, key: &str, value: HashMap<String, Value>, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        let id_str = id.unwrap_or(NO_ID_OVERRIDE);
        let mut parameters = HashMap::with_capacity(value.len());

        for (param_name, param_value) in value {
            let parameter = match serde_json::from_value::<Parameter>(param_value.clone()) {
                Ok(parameter) => parameter,
                Err(_) => Parameter::StaticValue(StaticValueParameter {
                    ref_type: "static".to_string(),
                    param_type: infer_param_type(&param_value).to_string(),
                    value: param_value,
                    name: None,
                }),
            };
            parameters.insert(param_name, parameter);
        }

        store
            .parameter_store
            .entry(key.to_string())
            .or_default()
            .insert(id_str.to_string(), parameters);
    }

    fn remove_gate_override(&self, key: &str, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        match id {
            None => {
                store.gate.remove(key);
            }
            Some(id_str) => {
                if let Some(overrides) = store.gate.get_mut(key) {
                    overrides.remove(&id_str.to_string());
                }
            }
        }
    }

    fn remove_dynamic_config_override(&self, key: &str, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        match id {
            None => {
                store.config.remove(key);
            }
            Some(id_str) => {
                if let Some(overrides) = store.config.get_mut(key) {
                    overrides.remove(&id_str.to_string());
                }
            }
        }
    }

    fn remove_experiment_override(&self, key: &str, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        match id {
            None => {
                store.experiment.remove(key);
            }
            Some(id_str) => {
                if let Some(overrides) = store.experiment.get_mut(key) {
                    overrides.remove(&id_str.to_string());
                }
            }
        }
    }

    fn remove_layer_override(&self, key: &str, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        match id {
            None => {
                store.layer.remove(key);
            }
            Some(id_str) => {
                if let Some(overrides) = store.layer.get_mut(key) {
                    overrides.remove(&id_str.to_string());
                }
            }
        }
    }

    fn remove_parameter_store_override(&self, key: &str, id: Option<&str>) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        match id {
            None => {
                store.parameter_store.remove(key);
            }
            Some(id_str) => {
                if let Some(overrides) = store.parameter_store.get_mut(key) {
                    overrides.remove(&id_str.to_string());
                }
            }
        }
    }

    fn remove_all_overrides(&self) {
        let mut store = write_lock_or_noop!(TAG, self.store);
        store.gate.clear();
        store.config.clear();
        store.experiment.clear();
        store.layer.clear();
        store.parameter_store.clear();
    }
}

fn find_override_for_user<T, F>(
    user: &StatsigUser,
    overrides: &HashMap<String, T>,
    apply_override: F,
    result: &mut EvaluatorResult,
) -> bool
where
    F: Fn(&T, &mut EvaluatorResult),
{
    if check_user_id_override(user, overrides, &apply_override, result) {
        return true;
    }

    if check_custom_ids_override(user, overrides, &apply_override, result) {
        return true;
    }

    check_default_override(overrides, apply_override, result)
}

fn mark_result_as_override(result: &mut EvaluatorResult) {
    result.override_reason = Some(LOCAL_OVERRIDE_REASON);
    result.rule_id = Some(OVERRIDE_RULE_ID.clone());
}

fn check_default_override<T, F>(
    overrides: &HashMap<String, T>,
    apply_override: F,
    result: &mut EvaluatorResult,
) -> bool
where
    F: Fn(&T, &mut EvaluatorResult),
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
    result: &mut EvaluatorResult,
) -> bool
where
    F: Fn(&T, &mut EvaluatorResult),
{
    let user_id = match &user.data.user_id {
        Some(id) => id,
        None => return false,
    };

    let id_string = match &user_id.string_value {
        Some(s) => &s.value,
        None => return false,
    };

    let override_value = match overrides.get(id_string.as_str()) {
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
    result: &mut EvaluatorResult,
) -> bool
where
    F: Fn(&T, &mut EvaluatorResult),
{
    let custom_ids = match &user.data.custom_ids {
        Some(ids) => ids,
        None => return false,
    };

    for custom_id_value in custom_ids.values() {
        let id_string = match &custom_id_value.string_value {
            Some(s) => &s.value,
            None => continue,
        };

        let override_value = match overrides.get(id_string.as_str()) {
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
) -> Option<DynamicReturnable> {
    let spec = opt_spec?;

    for rule in &spec.rules {
        let rule_group_name = match &rule.group_name {
            Some(rule_group_name) => rule_group_name,
            None => continue,
        };

        if rule_group_name == group_name {
            return Some(rule.return_value.clone());
        }
    }

    None
}

fn infer_param_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
