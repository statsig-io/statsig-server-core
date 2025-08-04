use std::collections::HashMap;

use crate::{
    evaluation::{
        evaluation_types::AnyConfigEvaluation,
        evaluation_types_v2::AnyConfigEvaluationV2,
        evaluator::{Evaluator, SpecType},
        evaluator_context::EvaluatorContext,
        evaluator_result::{
            result_to_dynamic_config_eval, result_to_dynamic_config_eval_v2,
            result_to_experiment_eval, result_to_experiment_eval_v2,
        },
    },
    gcir::{
        gcir_process_iter::hash_secondary_exposures,
        target_app_id_utils::should_filter_config_for_app,
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
    let factory = |spec_entity: &str, hashed_name: &str, ctx: &mut EvaluatorContext| {
        if spec_entity == "dynamic_config" {
            return AnyConfigEvaluation::DynamicConfig(result_to_dynamic_config_eval(
                hashed_name,
                &mut ctx.result,
            ));
        }

        let mut evaluation =
            result_to_experiment_eval(hashed_name, Some(spec_entity), &mut ctx.result);
        evaluation.undelegated_secondary_exposures = None;
        AnyConfigEvaluation::Experiment(evaluation)
    };

    let mut result = gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.spec_store_data.values.dynamic_configs,
        get_dynamic_config_type,
        factory,
    )?;

    let cmab_configs = match &context.spec_store_data.values.cmab_configs {
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
            &cmab_configs.get(config_name).unwrap().target_app_ids,
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
    sec_expo_hash_memo: &mut HashMap<String, String>,
    exposures: &mut HashMap<String, SecondaryExposure>,
) -> Result<HashMap<String, AnyConfigEvaluationV2>, StatsigErr> {
    let factory = |spec_entity: &str, hashed_name: &str, ctx: &mut EvaluatorContext| {
        stringify_sec_exposures(&ctx.result.secondary_exposures, ctx.hashing, exposures);

        if spec_entity == "dynamic_config" {
            return AnyConfigEvaluationV2::DynamicConfig(result_to_dynamic_config_eval_v2(
                hashed_name,
                &mut ctx.result,
                ctx.hashing,
            ));
        }

        let mut evaluation = result_to_experiment_eval_v2(
            hashed_name,
            Some(spec_entity),
            &mut ctx.result,
            ctx.hashing,
        );
        evaluation.undelegated_secondary_exposures = None;
        AnyConfigEvaluationV2::Experiment(evaluation)
    };

    let mut result = gcir_process_iter(
        context,
        options,
        sec_expo_hash_memo,
        &context.spec_store_data.values.dynamic_configs,
        get_dynamic_config_type,
        factory,
    )?;

    let cmab_configs = match &context.spec_store_data.values.cmab_configs {
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
            &cmab_configs.get(config_name).unwrap().target_app_ids,
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
        evaluation.undelegated_secondary_exposures = None;

        result.insert(hashed_name, AnyConfigEvaluationV2::Experiment(evaluation));
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
