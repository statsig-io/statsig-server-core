use ahash::AHashMap;
use std::collections::HashMap;

use crate::{
    evaluation::{
        evaluation_types::GateEvaluation,
        evaluation_types_initialize_v2::GateEvaluationInitV2,
        evaluation_types_v2::GateEvaluationV2,
        evaluator::SpecType,
        evaluator_context::EvaluatorContext,
        evaluator_result::{
            result_to_gate_eval, result_to_gate_eval_init_v2, result_to_gate_eval_v2,
        },
        secondary_exposure_key::SecondaryExposureKey,
    },
    interned_string::InternedString,
    specs_response::spec_types::Spec,
    ClientInitResponseOptions, SecondaryExposure, StatsigErr,
};

use super::{
    gcir_process_iter::gcir_process_iter, stringify_sec_exposures::stringify_sec_exposures,
};

pub(crate) fn get_gate_evaluations(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<InternedString, InternedString>,
) -> Result<HashMap<String, GateEvaluation>, StatsigErr> {
    let factory = |_: &str, hashed_name: &str, ctx: &mut EvaluatorContext| {
        let mut res = result_to_gate_eval(hashed_name, &mut ctx.result);

        if options.remove_id_type.unwrap_or(false) {
            res.id_type = None
        }
        res
    };

    gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.specs_data.feature_gates,
        get_gate_spec_type,
        factory,
    )
}

pub(crate) fn get_gate_evaluations_v2(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<InternedString, InternedString>,
    exposures: &mut HashMap<String, SecondaryExposure>,
) -> Result<HashMap<String, GateEvaluationV2>, StatsigErr> {
    let factory = |_: &str, hashed_name: &str, ctx: &mut EvaluatorContext| {
        stringify_sec_exposures(&ctx.result.secondary_exposures, ctx.hashing, exposures);
        let mut res = result_to_gate_eval_v2(hashed_name, &mut ctx.result, ctx.hashing);
        if options.remove_id_type.unwrap_or(false) {
            res.id_type = None
        }
        res
    };

    gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.specs_data.feature_gates,
        get_gate_spec_type,
        factory,
    )
}

pub(crate) fn get_gate_evaluations_init_v2(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    expo_id_to_exposure_map: &mut HashMap<InternedString, SecondaryExposure>,
    expo_key_to_expo_id_map: &mut AHashMap<SecondaryExposureKey, InternedString>,
) -> Result<HashMap<String, GateEvaluationInitV2>, StatsigErr> {
    let factory = |_: &str, _: &str, ctx: &mut EvaluatorContext| {
        let mut res = result_to_gate_eval_init_v2(
            &mut ctx.result,
            expo_id_to_exposure_map,
            expo_key_to_expo_id_map,
        );
        if options.remove_id_type.unwrap_or(false) {
            res.id_type = None
        }
        res
    };

    gcir_process_iter(
        context,
        options,
        &mut HashMap::new(),
        &context.specs_data.feature_gates,
        get_gate_spec_type,
        factory,
    )
}

fn get_gate_spec_type(_: &Spec) -> SpecType {
    SpecType::Gate
}
