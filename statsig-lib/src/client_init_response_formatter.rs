use crate::evaluation::evaluation_types::{AnyConfigEvaluation, SecondaryExposure};
use crate::evaluation::evaluator::Evaluator;
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::evaluation::evaluator_result::{
    result_to_dynamic_config_eval, result_to_experiment_eval, result_to_gate_eval,
    result_to_layer_eval, EvaluatorResult,
};
use crate::hashing::{HashAlgorithm, Hashing};
use crate::initialize_response::InitializeResponse;
use crate::read_lock_or_else;
use crate::spec_store::SpecStore;
use crate::statsig_user_internal::{StatsigUserInternal, StatsigUserLoggable};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ClientInitResponseOptions {
    pub hash_algorithm: Option<HashAlgorithm>,
    // todo: Support these options
    // pub client_sdk_key: Option<String>,
    // pub include_local_overrides: bool,
}

pub struct ClientInitResponseFormatter {
    spec_store: Arc<SpecStore>,
    default_options: ClientInitResponseOptions,
}

impl ClientInitResponseFormatter {
    pub fn new(spec_store: &Arc<SpecStore>) -> Self {
        Self {
            spec_store: spec_store.clone(),
            default_options: ClientInitResponseOptions {
                hash_algorithm: Some(HashAlgorithm::Sha256),
            },
        }
    }

    pub fn get_default_options(&self) -> &ClientInitResponseOptions {
        &self.default_options
    }

    pub fn get(
        &self,
        user_internal: StatsigUserInternal,
        hashing: &Hashing,
        options: &ClientInitResponseOptions,
    ) -> InitializeResponse {
        let data = read_lock_or_else!(self.spec_store.data, {
            return InitializeResponse::blank(user_internal);
        });

        let mut feature_gates = HashMap::new();
        let mut context = EvaluatorContext::new(&user_internal, &data, hashing);

        let hash_used = options
            .hash_algorithm
            .as_ref()
            .unwrap_or(&HashAlgorithm::Djb2);

        for (name, spec) in data.values.feature_gates.iter() {
            if spec.entity == "segment" || spec.entity == "holdout" {
                continue;
            }

            context.reset_result();
            Evaluator::evaluate(&mut context, spec);

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(&mut context.result, hashing, hash_used);

            let eval = result_to_gate_eval(&hashed_name, &mut context.result);
            feature_gates.insert(hashed_name, eval);
        }

        let mut dynamic_configs = HashMap::new();
        for (name, spec) in data.values.dynamic_configs.iter() {
            context.reset_result();
            Evaluator::evaluate(&mut context, spec);

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(&mut context.result, hashing, hash_used);

            if spec.entity == "dynamic_config" {
                let evaluation = result_to_dynamic_config_eval(&hashed_name, &mut context.result);
                dynamic_configs.insert(hashed_name, AnyConfigEvaluation::DynamicConfig(evaluation));
            } else {
                let evaluation = result_to_experiment_eval(&hashed_name, spec, &mut context.result);
                dynamic_configs.insert(hashed_name, AnyConfigEvaluation::Experiment(evaluation));
            }
        }

        let mut layer_configs = HashMap::new();
        for (name, spec) in &data.values.layer_configs {
            context.reset_result();
            Evaluator::evaluate(&mut context, spec);

            let hashed_name = context.hashing.hash(name, hash_used);
            hash_secondary_exposures(&mut context.result, hashing, hash_used);

            let mut evaluation = result_to_layer_eval(&hashed_name, &mut context.result);

            if let Some(allocated_experiment_name) = evaluation.allocated_experiment_name {
                evaluation.allocated_experiment_name = Some(context
                    .hashing
                    .hash(&allocated_experiment_name, hash_used));
            }

            layer_configs.insert(hashed_name, evaluation);
        }

        let evaluated_keys = get_evaluated_keys(&user_internal);

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
        }
    }
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
    hashing: &Hashing,
    hash_algorithm: &HashAlgorithm,
) {
    fn loop_and_hash(
        exposures: &Vec<SecondaryExposure>,
        hashing: &Hashing,
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
