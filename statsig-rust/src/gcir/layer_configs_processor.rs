use std::collections::HashMap;

use crate::{
    evaluation::{
        evaluation_types::LayerEvaluation,
        evaluation_types_v2::LayerEvaluationV2,
        evaluator::SpecType,
        evaluator_context::EvaluatorContext,
        evaluator_result::{result_to_layer_eval, result_to_layer_eval_v2},
    },
    hashing::HashUtil,
    specs_response::spec_types::Spec,
    ClientInitResponseOptions, SecondaryExposure, StatsigErr,
};

use super::{
    gcir_process_iter::gcir_process_iter, stringify_sec_exposures::stringify_sec_exposures,
};

pub(crate) fn get_layer_evaluations(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<String, String>,
) -> Result<HashMap<String, LayerEvaluation>, StatsigErr> {
    let factory = |_: &Spec, hashed_name: &str, ctx: &mut EvaluatorContext| {
        let mut eval = result_to_layer_eval(hashed_name, &mut ctx.result);

        try_hash_allocated_experiment_name(
            ctx.hashing,
            options,
            &mut eval.allocated_experiment_name,
        );

        eval
    };

    gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.spec_store_data.values.layer_configs,
        get_layer_spec_type,
        factory,
    )
}

pub(crate) fn get_layer_evaluations_v2(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<String, String>,
    exposures: &mut HashMap<String, SecondaryExposure>,
) -> Result<HashMap<String, LayerEvaluationV2>, StatsigErr> {
    let factory = |_: &Spec, hashed_name: &str, ctx: &mut EvaluatorContext| {
        stringify_sec_exposures(&ctx.result.secondary_exposures, ctx.hashing, exposures);
        if let Some(u) = &ctx.result.undelegated_secondary_exposures {
            stringify_sec_exposures(u, ctx.hashing, exposures);
        }

        let mut eval = result_to_layer_eval_v2(hashed_name, &mut ctx.result, ctx.hashing);

        try_hash_allocated_experiment_name(
            ctx.hashing,
            options,
            &mut eval.allocated_experiment_name,
        );

        eval
    };

    gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.spec_store_data.values.layer_configs,
        get_layer_spec_type,
        factory,
    )
}

fn try_hash_allocated_experiment_name(
    hashing: &HashUtil,
    options: &ClientInitResponseOptions,
    allocated_experiment_name: &mut Option<String>,
) {
    let name = match allocated_experiment_name.take() {
        Some(name) => name,
        None => return,
    };

    allocated_experiment_name.replace(hashing.hash(&name, options.get_hash_algorithm()));
}

fn get_layer_spec_type(_: &Spec) -> SpecType {
    SpecType::Layer
}
