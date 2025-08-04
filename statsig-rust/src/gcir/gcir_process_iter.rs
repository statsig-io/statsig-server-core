use super::target_app_id_utils::should_filter_spec_for_app;
use crate::{
    evaluation::{
        evaluator::{Evaluator, SpecType},
        evaluator_context::EvaluatorContext,
        evaluator_result::EvaluatorResult,
    },
    hashing::HashUtil,
    specs_response::{spec_directory::SpecDirectory, spec_types::Spec},
    ClientInitResponseOptions, HashAlgorithm, SecondaryExposure, StatsigErr,
};
use std::collections::{HashMap, HashSet};

pub(crate) fn gcir_process_iter<T>(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
    sec_expo_hash_memo: &mut HashMap<String, String>,
    spec_directory: &SpecDirectory,
    get_spec_type: impl Fn(&Spec) -> SpecType,
    mut evaluation_factory: impl FnMut(&str, &str, &mut EvaluatorContext) -> T,
) -> Result<HashMap<String, T>, StatsigErr> {
    let mut results = HashMap::new();

    for spec_addy in spec_directory.specs.values() {
        let spec = &spec_addy.spec;
        let name = &spec_addy.name;

        if spec.entity == "segment" || spec.entity == "holdout" {
            continue;
        }

        if should_filter_entity(spec, name.as_str(), options) {
            continue;
        }

        if should_filter_spec_for_app(spec, &context.app_id, &options.client_sdk_key) {
            continue;
        }

        context.reset_result();

        let spec_type = get_spec_type(spec);
        Evaluator::evaluate(context, spec_addy.name.as_str(), &spec_type)?;

        let hashed_name = context
            .hashing
            .hash(spec_addy.name.as_str(), options.get_hash_algorithm());
        hash_secondary_exposures(
            &mut context.result,
            context.hashing,
            options.get_hash_algorithm(),
            sec_expo_hash_memo,
        );

        let eval = evaluation_factory(&spec.entity, &hashed_name, context);
        results.insert(hashed_name, eval);
    }

    Ok(results)
}

fn should_filter_entity(spec: &Spec, name: &str, options: &ClientInitResponseOptions) -> bool {
    match spec.entity.as_str() {
        "feature_gate" => options
            .feature_gate_filter
            .as_ref()
            .is_some_and(|f| !f.contains(name)),
        "experiment" => options
            .experiment_filter
            .as_ref()
            .is_some_and(|f| !f.contains(name)),
        "dynamic_config" => options
            .dynamic_config_filter
            .as_ref()
            .is_some_and(|f| !f.contains(name)),
        "layer" => options
            .layer_filter
            .as_ref()
            .is_some_and(|f| !f.contains(name)),
        _ => false,
    }
}

pub fn hash_secondary_exposures(
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
