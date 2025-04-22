use crate::{
    evaluation::{
        dynamic_value::DynamicValue,
        evaluation_types::{AnyConfigEvaluation, SecondaryExposure},
        evaluator::{Evaluator, SpecType},
        evaluator_context::EvaluatorContext,
        evaluator_result::{
            result_to_dynamic_config_eval, result_to_experiment_eval, result_to_gate_eval,
            result_to_layer_eval, EvaluatorResult,
        },
    },
    hashing::{HashAlgorithm, HashUtil},
    initialize_response::InitializeResponse,
    read_lock_or_else,
    spec_store::SpecStore,
    specs_response::param_store_types::{
        DynamicConfigParameter, ExperimentParameter, GateParameter, LayerParameter, Parameter,
        ParameterStore,
    },
    specs_response::spec_types::Spec,
    statsig_metadata::StatsigMetadata,
    user::StatsigUserInternal,
    OverrideAdapter,
};

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Default, Deserialize)]
pub struct ClientInitResponseOptions {
    pub hash_algorithm: Option<HashAlgorithm>,
    pub client_sdk_key: Option<String>,
    pub include_local_overrides: Option<bool>,
}

pub struct ClientInitResponseFormatter {
    spec_store: Arc<SpecStore>,
    default_options: ClientInitResponseOptions,
    override_adapter: Option<Arc<dyn OverrideAdapter>>,
}

impl ClientInitResponseFormatter {
    pub fn new(
        spec_store: &Arc<SpecStore>,
        override_adapter: &Option<Arc<dyn OverrideAdapter>>,
    ) -> Self {
        Self {
            spec_store: spec_store.clone(),
            override_adapter: override_adapter.as_ref().map(Arc::clone),
            default_options: ClientInitResponseOptions {
                hash_algorithm: Some(HashAlgorithm::Djb2),
                client_sdk_key: None,
                include_local_overrides: Some(false),
            },
        }
    }

    pub fn get_default_options(&self) -> &ClientInitResponseOptions {
        &self.default_options
    }

    pub fn get(
        &self,
        user_internal: StatsigUserInternal,
        hashing: &HashUtil,
        options: &ClientInitResponseOptions,
    ) -> InitializeResponse {
        let data = read_lock_or_else!(self.spec_store.data, {
            return InitializeResponse::blank(user_internal);
        });

        let mut sec_expo_hash_memo = HashMap::new();
        let mut app_id = data.values.app_id.as_ref();

        if let Some(client_sdk_key) = &options.client_sdk_key {
            if let Some(app_id_value) = &data.values.sdk_keys_to_app_ids {
                app_id = app_id_value.get(client_sdk_key);
            }
            if let Some(app_id_value) = &data.values.hashed_sdk_keys_to_app_ids {
                let hashed_key = &hashing.hash(client_sdk_key, &HashAlgorithm::Djb2);
                app_id = app_id_value.get(hashed_key);
            }
        }
        let include_local_overrides = options.include_local_overrides.unwrap_or(false);
        let mut feature_gates = HashMap::new();
        let mut context = EvaluatorContext::new(
            &user_internal,
            &data,
            hashing,
            &app_id,
            if include_local_overrides {
                &self.override_adapter
            } else {
                &None
            },
        );

        let hash_used = options
            .hash_algorithm
            .as_ref()
            .unwrap_or(&HashAlgorithm::Djb2);

        for (name, spec) in &data.values.feature_gates {
            if spec.entity == "segment" || spec.entity == "holdout" {
                continue;
            }

            if should_filter_spec_for_app(spec, &app_id, &options.client_sdk_key) {
                continue;
            }

            context.reset_between_top_level_evaluations();
            if let Err(_err) = Evaluator::evaluate(&mut context, name, &SpecType::Gate) {
                return InitializeResponse::blank(user_internal);
            }

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(
                &mut context.result,
                hashing,
                hash_used,
                &mut sec_expo_hash_memo,
            );

            let eval = result_to_gate_eval(&hashed_name, &mut context.result);
            feature_gates.insert(hashed_name, eval);
        }

        let mut dynamic_configs = HashMap::new();
        for (name, spec) in &data.values.dynamic_configs {
            if should_filter_spec_for_app(spec, &app_id, &options.client_sdk_key) {
                continue;
            }

            context.reset_between_top_level_evaluations();
            let spec_type = if spec.entity == "dynamic_config" {
                &SpecType::DynamicConfig
            } else {
                &SpecType::Experiment
            };
            if let Err(_err) = Evaluator::evaluate(&mut context, name, spec_type) {
                return InitializeResponse::blank(user_internal);
            }

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(
                &mut context.result,
                hashing,
                hash_used,
                &mut sec_expo_hash_memo,
            );

            if spec.entity == "dynamic_config" {
                let evaluation = result_to_dynamic_config_eval(&hashed_name, &mut context.result);
                dynamic_configs.insert(hashed_name, AnyConfigEvaluation::DynamicConfig(evaluation));
            } else {
                let mut evaluation =
                    result_to_experiment_eval(&hashed_name, Some(spec), &mut context.result);
                evaluation.undelegated_secondary_exposures = None;
                dynamic_configs.insert(hashed_name, AnyConfigEvaluation::Experiment(evaluation));
            }
        }

        let mut layer_configs = HashMap::new();
        for (name, spec) in &data.values.layer_configs {
            if should_filter_spec_for_app(spec, &app_id, &options.client_sdk_key) {
                continue;
            }

            context.reset_between_top_level_evaluations();
            if let Err(_err) = Evaluator::evaluate(&mut context, name, &SpecType::Layer) {
                return InitializeResponse::blank(user_internal);
            }

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(
                &mut context.result,
                hashing,
                hash_used,
                &mut sec_expo_hash_memo,
            );

            let mut evaluation = result_to_layer_eval(&hashed_name, &mut context.result);

            if let Some(allocated_experiment_name) = evaluation.allocated_experiment_name {
                evaluation.allocated_experiment_name =
                    Some(context.hashing.hash(&allocated_experiment_name, hash_used));
            }

            layer_configs.insert(hashed_name, evaluation);
        }

        let mut param_stores = HashMap::new();
        let default_store = HashMap::new();
        let stores = match &data.values.param_stores {
            Some(stores) => stores,
            None => &default_store,
        };
        for (name, store) in stores {
            if should_filter_config_for_app(&store.target_app_ids, &app_id, &options.client_sdk_key)
            {
                continue;
            }

            let hashed_name = context.hashing.hash(name, hash_used);
            let parameters = get_parameters_from_store(store, hash_used, &context);
            param_stores.insert(hashed_name, parameters);
        }

        let evaluated_keys = get_evaluated_keys(&user_internal);
        let metadata = StatsigMetadata::get_metadata();
        InitializeResponse {
            feature_gates,
            dynamic_configs,
            layer_configs,
            time: data.values.time,
            has_updates: true,
            hash_used: hash_used.to_string(),
            user: user_internal.to_loggable(),
            sdk_params: HashMap::new(),
            evaluated_keys,
            sdk_info: HashMap::from([
                ("sdkType".to_string(), metadata.sdk_type),
                ("sdkVersion".to_string(), metadata.sdk_version),
            ]),
            param_stores,
        }
    }
}

fn get_parameters_from_store(
    store: &ParameterStore,
    hash_used: &HashAlgorithm,
    context: &EvaluatorContext,
) -> HashMap<String, Parameter> {
    let mut parameters = HashMap::new();
    for (param_name, param) in &store.parameters {
        match param {
            Parameter::StaticValue(_static_value) => {
                parameters.insert(param_name.clone(), param.clone());
            }
            Parameter::Gate(gate) => {
                let new_param = GateParameter {
                    ref_type: gate.ref_type.clone(),
                    param_type: gate.param_type.clone(),
                    gate_name: context.hashing.hash(&gate.gate_name, hash_used),
                    pass_value: gate.pass_value.clone(),
                    fail_value: gate.fail_value.clone(),
                };
                parameters.insert(param_name.clone(), Parameter::Gate(new_param));
            }
            Parameter::DynamicConfig(dynamic_config) => {
                let new_param = DynamicConfigParameter {
                    ref_type: dynamic_config.ref_type.clone(),
                    param_type: dynamic_config.param_type.clone(),
                    config_name: context.hashing.hash(&dynamic_config.config_name, hash_used),
                    param_name: dynamic_config.param_name.clone(),
                };
                parameters.insert(param_name.clone(), Parameter::DynamicConfig(new_param));
            }
            Parameter::Experiment(experiment) => {
                let new_param = ExperimentParameter {
                    ref_type: experiment.ref_type.clone(),
                    param_type: experiment.param_type.clone(),
                    experiment_name: context.hashing.hash(&experiment.experiment_name, hash_used),
                    param_name: experiment.param_name.clone(),
                };
                parameters.insert(param_name.clone(), Parameter::Experiment(new_param));
            }
            Parameter::Layer(layer) => {
                let new_param = LayerParameter {
                    ref_type: layer.ref_type.clone(),
                    param_type: layer.param_type.clone(),
                    layer_name: context.hashing.hash(&layer.layer_name, hash_used),
                    param_name: layer.param_name.clone(),
                };
                parameters.insert(param_name.clone(), Parameter::Layer(new_param));
            }
        }
    }
    parameters
}

fn should_filter_spec_for_app(
    spec: &Spec,
    app_id: &Option<&DynamicValue>,
    client_sdk_key: &Option<String>,
) -> bool {
    should_filter_config_for_app(&spec.target_app_ids, app_id, client_sdk_key)
}

fn should_filter_config_for_app(
    target_app_ids: &Option<Vec<String>>,
    app_id: &Option<&DynamicValue>,
    client_sdk_key: &Option<String>,
) -> bool {
    let _client_sdk_key = match client_sdk_key {
        Some(client_sdk_key) => client_sdk_key,
        None => return false,
    };

    let app_id = match app_id {
        Some(app_id) => app_id,
        None => return false,
    };

    let string_app_id = match app_id.string_value.as_ref() {
        Some(string_app_id) => string_app_id,
        None => return false,
    };

    let target_app_ids = match target_app_ids {
        Some(target_app_ids) => target_app_ids,
        None => return true,
    };

    if !target_app_ids.contains(string_app_id) {
        return true;
    }
    false
}

fn get_evaluated_keys(user_internal: &StatsigUserInternal) -> HashMap<String, String> {
    let mut evaluated_keys = HashMap::new();

    if let Some(user_id) = user_internal.user_data.user_id.as_ref() {
        evaluated_keys.insert(
            "userID".to_string(),
            user_id.string_value.clone().unwrap_or_default(),
        );
    }

    if let Some(custom_ids) = user_internal.user_data.custom_ids.as_ref() {
        for (key, value) in custom_ids {
            evaluated_keys.insert(key.clone(), value.string_value.clone().unwrap_or_default());
        }
    }

    evaluated_keys
}

fn hash_secondary_exposures(
    result: &mut EvaluatorResult,
    hashing: &HashUtil,
    hash_algorithm: &HashAlgorithm,
    memo: &mut HashMap<String, String>,
) {
    fn loop_filter_n_hash(
        exposures: &mut Vec<SecondaryExposure>,
        hashing: &HashUtil,
        hash_algorithm: &HashAlgorithm,
        memo: &mut HashMap<String, String>,
    ) {
        let mut seen = HashSet::<String>::with_capacity(exposures.len());
        exposures.retain_mut(|expo| {
            let expo_key = expo.get_dedupe_key();
            if seen.contains(&expo_key) {
                return false;
            }
            seen.insert(expo_key);

            match memo.get(&expo.gate) {
                Some(hash) => {
                    expo.gate = hash.clone();
                }
                None => {
                    let hash = hashing.hash(&expo.gate, hash_algorithm).to_string();
                    let old = std::mem::replace(&mut expo.gate, hash.clone());
                    memo.insert(old, hash);
                }
            }
            true
        });
    }

    if !result.secondary_exposures.is_empty() {
        loop_filter_n_hash(
            &mut result.secondary_exposures,
            hashing,
            hash_algorithm,
            memo,
        );
    }

    if let Some(undelegated_secondary_exposures) = result.undelegated_secondary_exposures.as_mut() {
        loop_filter_n_hash(
            undelegated_secondary_exposures,
            hashing,
            hash_algorithm,
            memo,
        );
    }
}
