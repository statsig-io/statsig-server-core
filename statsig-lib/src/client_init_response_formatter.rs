use serde::Deserialize;

use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluation_types::{AnyConfigEvaluation, SecondaryExposure};
use crate::evaluation::evaluator::{Evaluator, SpecType};
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::evaluation::evaluator_result::{
    result_to_dynamic_config_eval, result_to_experiment_eval, result_to_gate_eval,
    result_to_layer_eval, EvaluatorResult,
};
use crate::hashing::{HashAlgorithm, HashUtil};
use crate::initialize_response::InitializeResponse;
use crate::spec_store::SpecStore;
use crate::spec_types::Spec;
use crate::statsig_metadata::StatsigMetadata;
use crate::statsig_user_internal::{StatsigUserInternal, StatsigUserLoggable};
use crate::{read_lock_or_else, OverrideAdapter};
use std::collections::HashMap;
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

        for (name, spec) in data.values.feature_gates.iter() {
            if spec.entity == "segment" || spec.entity == "holdout" {
                continue;
            }

            if get_should_filter_config_for_app(spec, &app_id, &options.client_sdk_key) {
                continue;
            }

            context.reset_result();
            if let Err(_err) = Evaluator::evaluate(&mut context, name, &SpecType::Gate) {
                return InitializeResponse::blank(user_internal);
            }

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(&mut context.result, hashing, hash_used);

            let eval = result_to_gate_eval(&hashed_name, &mut context.result);
            feature_gates.insert(hashed_name, eval);
        }

        let mut dynamic_configs = HashMap::new();
        for (name, spec) in data.values.dynamic_configs.iter() {
            if get_should_filter_config_for_app(spec, &app_id, &options.client_sdk_key) {
                continue;
            }

            context.reset_result();
            let spec_type = if spec.entity == "dynamic_config" {
                &SpecType::DynamicConfig
            } else {
                &SpecType::Experiment
            };
            if let Err(_err) = Evaluator::evaluate(&mut context, name, spec_type) {
                return InitializeResponse::blank(user_internal);
            }

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(&mut context.result, hashing, hash_used);

            if spec.entity == "dynamic_config" {
                let evaluation = result_to_dynamic_config_eval(&hashed_name, &mut context.result);
                dynamic_configs.insert(hashed_name, AnyConfigEvaluation::DynamicConfig(evaluation));
            } else {
                let evaluation =
                    result_to_experiment_eval(&hashed_name, Some(spec), &mut context.result);
                dynamic_configs.insert(hashed_name, AnyConfigEvaluation::Experiment(evaluation));
            }
        }

        let mut layer_configs = HashMap::new();
        for (name, spec) in &data.values.layer_configs {
            if get_should_filter_config_for_app(spec, &app_id, &options.client_sdk_key) {
                continue;
            }

            context.reset_result();
            if let Err(_err) = Evaluator::evaluate(&mut context, name, &SpecType::Layer) {
                return InitializeResponse::blank(user_internal);
            }

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(&mut context.result, hashing, hash_used);

            let mut evaluation = result_to_layer_eval(&hashed_name, &mut context.result);

            if let Some(allocated_experiment_name) = evaluation.allocated_experiment_name {
                evaluation.allocated_experiment_name =
                    Some(context.hashing.hash(&allocated_experiment_name, hash_used));
            }

            layer_configs.insert(hashed_name, evaluation);
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
            user: StatsigUserLoggable::new(user_internal),
            sdk_params: HashMap::new(),
            evaluated_keys,
            sdk_info: HashMap::from([
                ("sdkType".to_string(), metadata.sdk_type),
                ("sdkVersion".to_string(), metadata.sdk_version),
            ]),
        }
    }
}

fn get_should_filter_config_for_app(
    spec: &Spec,
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

    let target_app_ids = match &spec.target_app_ids {
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
) {
    fn loop_and_hash(
        exposures: &[SecondaryExposure],
        hashing: &HashUtil,
        hash_algorithm: &HashAlgorithm,
    ) -> Vec<SecondaryExposure> {
        exposures
            .iter()
            .map(|exposure| {
                let hashed_gate = hashing.hash(&exposure.gate, hash_algorithm);
                SecondaryExposure {
                    gate: hashed_gate,
                    ..exposure.clone()
                }
            })
            .collect()
    }

    if !result.secondary_exposures.is_empty() {
        result.secondary_exposures =
            loop_and_hash(&result.secondary_exposures, hashing, hash_algorithm);
    }

    if let Some(undelegated_secondary_exposures) = result.undelegated_secondary_exposures.as_ref() {
        result.undelegated_secondary_exposures = Some(loop_and_hash(
            undelegated_secondary_exposures,
            hashing,
            hash_algorithm,
        ));
    }
}
