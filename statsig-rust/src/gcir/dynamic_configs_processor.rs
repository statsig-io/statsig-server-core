use ahash::AHashMap;
use std::collections::HashMap;

use crate::{
    evaluation::{
        dynamic_returnable::DynamicReturnable,
        evaluation_types::AnyConfigEvaluation,
        evaluation_types_initialize_v2::AnyConfigEvaluationInitV2,
        evaluation_types_v2::AnyConfigEvaluationV2,
        evaluator::{Evaluator, SpecType},
        evaluator_context::EvaluatorContext,
        evaluator_result::{
            result_to_dynamic_config_eval, result_to_dynamic_config_eval_init_v2,
            result_to_dynamic_config_eval_v2, result_to_experiment_eval,
            result_to_experiment_eval_init_v2, result_to_experiment_eval_v2,
        },
        secondary_exposure_key::SecondaryExposureKey,
    },
    gcir::{
        gcir_process_iter::hash_secondary_exposures,
        target_app_id_utils::should_filter_config_for_app,
    },
    interned_string::InternedString,
    specs_response::spec_types::Spec,
    ClientInitResponseOptions, SecondaryExposure, StatsigErr,
};

use super::{
    gcir_process_iter::gcir_process_iter, stringify_sec_exposures::stringify_sec_exposures,
};

pub(crate) fn get_dynamic_config_evaluations(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<InternedString, InternedString>,
) -> Result<HashMap<String, AnyConfigEvaluation>, StatsigErr> {
    let factory = |spec_entity: &str, hashed_name: &str, ctx: &mut EvaluatorContext| {
        if spec_entity == "dynamic_config" {
            let mut res = result_to_dynamic_config_eval(hashed_name, &mut ctx.result);
            if options.remove_id_type.unwrap_or(false) {
                res.id_type = None
            }
            return AnyConfigEvaluation::DynamicConfig(res);
        }

        let mut evaluation =
            result_to_experiment_eval(hashed_name, Some(spec_entity), &mut ctx.result);
        if options.remove_id_type.unwrap_or(false) {
            evaluation.id_type = None
        }
        evaluation.undelegated_secondary_exposures = None;
        AnyConfigEvaluation::Experiment(evaluation)
    };

    let mut result = gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.specs_data.dynamic_configs,
        get_dynamic_config_type,
        factory,
    )?;

    let cmab_configs = match &context.specs_data.cmab_configs {
        Some(cmab_configs) => cmab_configs,
        None => return Ok(result),
    };

    for config_name in cmab_configs.keys() {
        let hashed_name = context
            .hashing
            .hash(config_name.as_str(), options.get_hash_algorithm());
        if options
            .experiment_filter
            .as_ref()
            .is_some_and(|f| !f.contains(config_name))
        {
            continue;
        }
        if should_filter_config_for_app(
            cmab_configs
                .get(config_name)
                .and_then(|c| c.target_app_ids.as_ref()),
            &context.app_id,
            &options.client_sdk_key,
        ) {
            continue;
        }
        context.reset_result();
        let _ = Evaluator::evaluate(context, config_name.as_str(), &SpecType::Experiment);
        hash_secondary_exposures(
            &mut context.result,
            context.hashing,
            options.get_hash_algorithm(),
            sec_expo_hash_memo,
        );
        let eval = factory("autotune", &hashed_name, context);
        result.insert(hashed_name, eval);
    }

    Ok(result)
}

pub(crate) fn get_dynamic_config_evaluations_v2(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<InternedString, InternedString>,
    exposures: &mut HashMap<String, SecondaryExposure>,
) -> Result<HashMap<String, AnyConfigEvaluationV2>, StatsigErr> {
    let factory = |spec_entity: &str, hashed_name: &str, ctx: &mut EvaluatorContext| {
        stringify_sec_exposures(&ctx.result.secondary_exposures, ctx.hashing, exposures);

        if spec_entity == "dynamic_config" {
            let mut res =
                result_to_dynamic_config_eval_v2(hashed_name, &mut ctx.result, ctx.hashing);
            if options.remove_id_type.unwrap_or(false) {
                res.id_type = None
            }
            return AnyConfigEvaluationV2::DynamicConfig(res);
        }

        let mut evaluation = result_to_experiment_eval_v2(
            hashed_name,
            Some(spec_entity),
            &mut ctx.result,
            ctx.hashing,
        );
        evaluation.undelegated_secondary_exposures = None;
        if options.remove_id_type.unwrap_or(false) {
            evaluation.id_type = None
        }
        AnyConfigEvaluationV2::Experiment(evaluation)
    };

    let mut result = gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.specs_data.dynamic_configs,
        get_dynamic_config_type,
        factory,
    )?;

    let cmab_configs = match &context.specs_data.cmab_configs {
        Some(cmab_configs) => cmab_configs,
        None => return Ok(result),
    };

    for config_name in cmab_configs.keys() {
        let hashed_name = context
            .hashing
            .hash(config_name.as_str(), options.get_hash_algorithm());
        if options
            .experiment_filter
            .as_ref()
            .is_some_and(|f| !f.contains(config_name))
        {
            continue;
        }

        let target_app_ids = cmab_configs
            .get(config_name)
            .and_then(|c| c.target_app_ids.as_ref());
        if should_filter_config_for_app(target_app_ids, &context.app_id, &options.client_sdk_key) {
            continue;
        }
        context.reset_result();
        let _ = Evaluator::evaluate(context, config_name.as_str(), &SpecType::Experiment);
        hash_secondary_exposures(
            &mut context.result,
            context.hashing,
            options.get_hash_algorithm(),
            sec_expo_hash_memo,
        );
        stringify_sec_exposures(
            &context.result.secondary_exposures,
            context.hashing,
            exposures,
        );

        let mut evaluation = result_to_experiment_eval_v2(
            &hashed_name,
            Some("autotune"),
            &mut context.result,
            context.hashing,
        );
        if options.remove_id_type.unwrap_or(false) {
            evaluation.id_type = None
        }
        evaluation.undelegated_secondary_exposures = None;

        result.insert(hashed_name, AnyConfigEvaluationV2::Experiment(evaluation));
    }

    Ok(result)
}

pub(crate) fn get_dynamic_config_evaluations_init_v2(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    expo_id_to_exposure_map: &mut HashMap<InternedString, SecondaryExposure>,
    expo_key_to_expo_id_map: &mut AHashMap<SecondaryExposureKey, InternedString>,
    value_id_to_value_map: &mut HashMap<InternedString, DynamicReturnable>,
    value_key_to_value_id: &mut AHashMap<u64, InternedString>,
) -> Result<HashMap<String, AnyConfigEvaluationInitV2>, StatsigErr> {
    let factory = |spec_entity: &str, _: &str, ctx: &mut EvaluatorContext| {
        if spec_entity == "dynamic_config" {
            let mut res = result_to_dynamic_config_eval_init_v2(
                &mut ctx.result,
                expo_id_to_exposure_map,
                expo_key_to_expo_id_map,
                value_id_to_value_map,
                value_key_to_value_id,
            );
            if options.remove_id_type.unwrap_or(false) {
                res.id_type = None
            }
            return AnyConfigEvaluationInitV2::DynamicConfig(res);
        }

        let mut evaluation = result_to_experiment_eval_init_v2(
            Some(spec_entity),
            &mut ctx.result,
            expo_id_to_exposure_map,
            expo_key_to_expo_id_map,
            value_id_to_value_map,
            value_key_to_value_id,
        );
        if options.remove_id_type.unwrap_or(false) {
            evaluation.id_type = None
        }
        AnyConfigEvaluationInitV2::Experiment(evaluation)
    };

    let mut result = gcir_process_iter(
        context,
        options,
        &mut HashMap::new(),
        &context.specs_data.dynamic_configs,
        get_dynamic_config_type,
        factory,
    )?;

    let cmab_configs = match &context.specs_data.cmab_configs {
        Some(cmab_configs) => cmab_configs,
        None => return Ok(result),
    };

    for config_name in cmab_configs.keys() {
        let hashed_name = context
            .hashing
            .hash(config_name.as_str(), options.get_hash_algorithm());
        if options
            .experiment_filter
            .as_ref()
            .is_some_and(|f| !f.contains(config_name))
        {
            continue;
        }

        let target_app_ids = cmab_configs
            .get(config_name)
            .and_then(|c| c.target_app_ids.as_ref());
        if should_filter_config_for_app(target_app_ids, &context.app_id, &options.client_sdk_key) {
            continue;
        }
        context.reset_result();
        let _ = Evaluator::evaluate(context, config_name.as_str(), &SpecType::Experiment);

        let mut evaluation = result_to_experiment_eval_init_v2(
            Some("autotune"),
            &mut context.result,
            expo_id_to_exposure_map,
            expo_key_to_expo_id_map,
            value_id_to_value_map,
            value_key_to_value_id,
        );
        if options.remove_id_type.unwrap_or(false) {
            evaluation.id_type = None
        }

        result.insert(
            hashed_name,
            AnyConfigEvaluationInitV2::Experiment(evaluation),
        );
    }

    Ok(result)
}

fn get_dynamic_config_type(spec: &Spec) -> SpecType {
    if spec.entity == "dynamic_config" {
        SpecType::DynamicConfig
    } else {
        SpecType::Experiment
    }
}
