use std::collections::HashMap;

use crate::{
    evaluation::{
        evaluation_types::GateEvaluation,
        evaluation_types_v2::GateEvaluationV2,
        evaluator::SpecType,
        evaluator_context::EvaluatorContext,
        evaluator_result::{result_to_gate_eval, result_to_gate_eval_v2},
    },
    specs_response::spec_types::Spec,
    ClientInitResponseOptions, SecondaryExposure, StatsigErr,
};

use super::{
    gcir_process_iter::gcir_process_iter, stringify_sec_exposures::stringify_sec_exposures,
};

pub(crate) fn get_gate_evaluations(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<String, String>,
) -> Result<HashMap<String, GateEvaluation>, StatsigErr> {
    let factory = |_: &Spec, hashed_name: &str, ctx: &mut EvaluatorContext| {
        result_to_gate_eval(hashed_name, &mut ctx.result)
    };

    gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.spec_store_data.values.feature_gates,
        get_gate_spec_type,
        factory,
    )
}

pub(crate) fn get_gate_evaluations_v2(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<String, String>,
    exposures: &mut HashMap<String, SecondaryExposure>,
) -> Result<HashMap<String, GateEvaluationV2>, StatsigErr> {
    let factory = |_: &Spec, hashed_name: &str, ctx: &mut EvaluatorContext| {
        stringify_sec_exposures(&ctx.result.secondary_exposures, ctx.hashing, exposures);
        result_to_gate_eval_v2(hashed_name, &mut ctx.result, ctx.hashing)
    };

    gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.spec_store_data.values.feature_gates,
        get_gate_spec_type,
        factory,
    )
}

fn get_gate_spec_type(_: &Spec) -> SpecType {
    SpecType::Gate
}
