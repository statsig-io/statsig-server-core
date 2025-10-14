use std::collections::HashMap;

use crate::{
    evaluation::evaluator_context::EvaluatorContext,
    specs_response::param_store_types::{
        DynamicConfigParameter, ExperimentParameter, GateParameter, LayerParameter, Parameter,
        ParameterStore,
    },
    ClientInitResponseOptions, HashAlgorithm,
};

use super::target_app_id_utils::should_filter_config_for_app;

type SerializeableParamStore = HashMap<String, HashMap<String, Parameter>>;

pub(crate) fn get_serializeable_param_stores(
    context: &mut EvaluatorContext,
    options: &ClientInitResponseOptions,
) -> SerializeableParamStore {
    let stores = match &context.specs_data.param_stores {
        Some(stores) => stores,
        None => return HashMap::new(),
    };

    let mut param_stores = HashMap::new();

    for (name, store) in stores {
        if should_filter_config_for_app(
            store.target_app_ids.as_ref(),
            &context.app_id,
            &options.client_sdk_key,
        ) {
            continue;
        }

        if let Some(param_store_filter) = &options.param_store_filter {
            if !param_store_filter.contains(name) {
                continue;
            }
        }

        let hash_algorithm = options.get_hash_algorithm();
        let hashed_name = context.hashing.hash(name, hash_algorithm);
        let parameters = get_parameters_from_store(store, hash_algorithm, context);
        param_stores.insert(hashed_name, parameters);
    }

    param_stores
}

fn get_parameters_from_store(
    store: &ParameterStore,
    hash_used: &HashAlgorithm,
    context: &EvaluatorContext,
) -> HashMap<String, Parameter> {
    // todo: what the heck is this mess?
    let mut parameters: HashMap<String, Parameter> = HashMap::new();
    for (param_name, param) in &store.parameters {
        match param {
            Parameter::StaticValue(_static_value) => {
                parameters.insert(param_name.clone(), param.clone());
            }

            Parameter::Gate(gate) => {
                let new_param = GateParameter {
                    ref_type: gate.ref_type.clone(),
                    param_type: gate.param_type.clone(),
                    gate_name: context.hashing.hash(&gate.gate_name, hash_used),
                    pass_value: gate.pass_value.clone(),
                    fail_value: gate.fail_value.clone(),
                };
                parameters.insert(param_name.clone(), Parameter::Gate(new_param));
            }

            Parameter::DynamicConfig(dynamic_config) => {
                let new_param = DynamicConfigParameter {
                    ref_type: dynamic_config.ref_type.clone(),
                    param_type: dynamic_config.param_type.clone(),
                    config_name: context.hashing.hash(&dynamic_config.config_name, hash_used),
                    param_name: dynamic_config.param_name.clone(),
                };
                parameters.insert(param_name.clone(), Parameter::DynamicConfig(new_param));
            }

            Parameter::Experiment(experiment) => {
                let new_param = ExperimentParameter {
                    ref_type: experiment.ref_type.clone(),
                    param_type: experiment.param_type.clone(),
                    experiment_name: context.hashing.hash(&experiment.experiment_name, hash_used),
                    param_name: experiment.param_name.clone(),
                };
                parameters.insert(param_name.clone(), Parameter::Experiment(new_param));
            }

            Parameter::Layer(layer) => {
                let new_param = LayerParameter {
                    ref_type: layer.ref_type.clone(),
                    param_type: layer.param_type.clone(),
                    layer_name: context.hashing.hash(&layer.layer_name, hash_used),
                    param_name: layer.param_name.clone(),
                };
                parameters.insert(param_name.clone(), Parameter::Layer(new_param));
            }
        }
    }
    parameters
}
