use std::collections::HashMap;

use crate::{
    evaluation::{
        evaluation_types::AnyConfigEvaluation,
        evaluation_types_v2::AnyConfigEvaluationV2,
        evaluator::SpecType,
        evaluator_context::EvaluatorContext,
        evaluator_result::{
            result_to_dynamic_config_eval, result_to_dynamic_config_eval_v2,
            result_to_experiment_eval, result_to_experiment_eval_v2,
        },
    },
    specs_response::spec_types::Spec,
    ClientInitResponseOptions, SecondaryExposure, StatsigErr,
};

use super::{
    gcir_process_iter::gcir_process_iter, stringify_sec_exposures::stringify_sec_exposures,
};

pub(crate) fn get_dynamic_config_evaluations(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<String, String>,
) -> Result<HashMap<String, AnyConfigEvaluation>, StatsigErr> {
    let factory = |spec: &Spec, hashed_name: &str, ctx: &mut EvaluatorContext| {
        if spec.entity == "dynamic_config" {
            return AnyConfigEvaluation::DynamicConfig(result_to_dynamic_config_eval(
                hashed_name,
                &mut ctx.result,
            ));
        }

        let mut evaluation = result_to_experiment_eval(hashed_name, Some(spec), &mut ctx.result);
        evaluation.undelegated_secondary_exposures = None;
        AnyConfigEvaluation::Experiment(evaluation)
    };

    gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.spec_store_data.values.dynamic_configs,
        get_dynamic_config_type,
        factory,
    )
}

pub(crate) fn get_dynamic_config_evaluations_v2(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<String, String>,
    exposures: &mut HashMap<String, SecondaryExposure>,
) -> Result<HashMap<String, AnyConfigEvaluationV2>, StatsigErr> {
    let factory = |spec: &Spec, hashed_name: &str, ctx: &mut EvaluatorContext| {
        stringify_sec_exposures(&ctx.result.secondary_exposures, ctx.hashing, exposures);

        if spec.entity == "dynamic_config" {
            return AnyConfigEvaluationV2::DynamicConfig(result_to_dynamic_config_eval_v2(
                hashed_name,
                &mut ctx.result,
                ctx.hashing,
            ));
        }

        let mut evaluation =
            result_to_experiment_eval_v2(hashed_name, Some(spec), &mut ctx.result, ctx.hashing);
        evaluation.undelegated_secondary_exposures = None;
        AnyConfigEvaluationV2::Experiment(evaluation)
    };

    gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.spec_store_data.values.dynamic_configs,
        get_dynamic_config_type,
        factory,
    )
}

fn get_dynamic_config_type(spec: &Spec) -> SpecType {
    if spec.entity == "dynamic_config" {
        SpecType::DynamicConfig
    } else {
        SpecType::Experiment
    }
}
