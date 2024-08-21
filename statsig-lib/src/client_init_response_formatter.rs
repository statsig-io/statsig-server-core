use crate::evaluation::evaluation_types::AnyConfigEvaluation;
use crate::evaluation::evaluator::Evaluator;
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::evaluation::evaluator_result::{
    result_to_dynamic_config_eval, result_to_experiment_eval, result_to_gate_eval,
    result_to_layer_eval,
};
use crate::initialize_response::InitializeResponse;
use crate::memo_sha_256::MemoSha256;
use crate::read_lock_or_else;
use crate::spec_store::SpecStore;
use crate::statsig_user_internal::StatsigUserInternal;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ClientInitResponseFormatter {
    spec_store: Arc<SpecStore>,
}

impl ClientInitResponseFormatter {
    pub fn new(spec_store: &Arc<SpecStore>) -> Self {
        Self {
            spec_store: spec_store.clone(),
        }
    }
    pub fn get(
        &self,
        user_internal: StatsigUserInternal,
        sha_hasher: &MemoSha256,
    ) -> InitializeResponse {
        let data = read_lock_or_else!(self.spec_store.data, {
            return InitializeResponse::blank(user_internal);
        });

        let mut feature_gates = HashMap::new();
        let mut context = EvaluatorContext::new(&user_internal, &data, sha_hasher);

        for (name, spec) in data.values.feature_gates.iter() {
            if spec.entity == "segment" || spec.entity == "holdout" {
                continue;
            }

            context.reset_result();
            Evaluator::evaluate(&mut context, spec);

            let hashed_name = context.sha_hasher.hash_name(name);
            let eval = result_to_gate_eval(&hashed_name, spec, &mut context.result);
            feature_gates.insert(hashed_name, eval);
        }

        let mut dynamic_configs = HashMap::new();
        for (name, spec) in data.values.dynamic_configs.iter() {
            context.reset_result();
            Evaluator::evaluate(&mut context, spec);

            let hashed_name = context.sha_hasher.hash_name(name);
            if spec.entity == "dynamic_config" {
                let evaluation =
                    result_to_dynamic_config_eval(&hashed_name, spec, &mut context.result);
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

            let hashed_name = context.sha_hasher.hash_name(name);
            let evaluation = result_to_layer_eval(&hashed_name, spec, &mut context.result);
            layer_configs.insert(hashed_name, evaluation);
        }

        InitializeResponse {
            feature_gates,
            dynamic_configs,
            layer_configs,
            time: data.values.time,
            has_updates: true,
            hash_used: "sha256".to_string(),
            user: user_internal,
        }
    }
}
