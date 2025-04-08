use std::sync::{Arc, Weak};

use crate::{
    evaluation::evaluator_result::{
        eval_result_to_experiment_eval, result_to_layer_eval, EvaluatorResult,
    },
    event_logging::event_logger::EventLogger,
    get_persistent_storage_key, log_d, log_e, make_experiment_from_sticky_value,
    make_layer_from_sticky_value, make_sticky_value_from_experiment, make_sticky_value_from_layer,
    statsig_types::{Experiment, Layer},
    statsig_user_internal::StatsigUserInternal,
    unwrap_or_return, ExperimentEvaluationOptions, LayerEvaluationOptions, PersistentStorage,
    SamplingProcessor,
};
pub struct PersistentValuesManager {
    pub persistent_storage: Arc<dyn PersistentStorage>,
}

const TAG: &str = "PersistentValuesManager";

macro_rules! get_sticky_result {
    ($option:expr, $config_name:expr) => {{
        let mut sticky_value_ptr = None;
        let maybe_sticky_result = match $option.user_persisted_values.as_ref() {
            Some(values) => match values.get($config_name) {
                Some(sticky_value) => {
                    sticky_value_ptr = Some(sticky_value);
                    let sticky_result = EvaluatorResult {
                        bool_value: sticky_value.value,
                        json_value: sticky_value.json_value.clone(),
                        rule_id: sticky_value.rule_id.as_ref(),
                        group_name: sticky_value.group_name.as_ref(),
                        secondary_exposures: sticky_value.secondary_exposures.clone(),
                        undelegated_secondary_exposures: sticky_value
                            .undelegated_secondary_exposures
                            .clone(),
                        version: sticky_value.config_version,
                        config_delegate: sticky_value.config_delegate.as_ref(),
                        ..Default::default()
                    };
                    Some(sticky_result)
                }
                None => None,
            },
            None => None,
        };
        (maybe_sticky_result, sticky_value_ptr)
    }};
}

macro_rules! try_finalize_sticky_storage {
    ($self: expr, $user: expr, $option:expr, $evaluator_result:expr, $maybe_sticky_result:expr, $sticky_value_ptr: expr,  $make_sticky_value_fn:expr) => {{
        let config_name = $evaluator_result.name.as_str();
        let is_in_experiment = $evaluator_result
            .__evaluation
            .as_ref()
            .and_then(|e| e.is_user_in_experiment)
            .unwrap_or_default();
        let is_experiment_active = $evaluator_result
            .__evaluation
            .as_ref()
            .and_then(|e| e.is_experiment_active)
            .unwrap_or_default();
        let storage_key = unwrap_or_return!(
            get_persistent_storage_key($user, &$evaluator_result.id_type),
            None
        );
        if $option.user_persisted_values.is_none() {
            log_d!(TAG, "Delete persisted assignment {}", config_name);
            $self
                .persistent_storage
                .delete(storage_key.as_str(), config_name);
        } else if $sticky_value_ptr.is_none() && is_in_experiment && is_experiment_active {
            match $make_sticky_value_fn($evaluator_result) {
                Some(sticky_value) => {
                    log_d!(TAG, "Save persisted assignment {}", config_name);
                    $self
                        .persistent_storage
                        .save(&storage_key, config_name, sticky_value)
                }
                None => log_e!(TAG, "Failed to save sticky value"),
            }
        }
    }};
}

impl PersistentValuesManager {
    pub fn try_apply_sticky_value_to_experiment<'a>(
        &self,
        user: &'a StatsigUserInternal,
        option: &'a ExperimentEvaluationOptions,
        evaluator_result: &'a Experiment,
    ) -> Option<Experiment> {
        let config_name = evaluator_result.name.as_str();
        let (mut maybe_sticky_result, sticky_value_ptr) = get_sticky_result!(option, config_name);
        try_finalize_sticky_storage!(
            self,
            user,
            option,
            evaluator_result,
            maybe_sticky_result,
            sticky_value_ptr,
            make_sticky_value_from_experiment
        );
        let sticky_value = unwrap_or_return!(sticky_value_ptr, None).clone();
        maybe_sticky_result
            .as_mut()
            .map(|sticky_result: &mut EvaluatorResult<'_>| {
                let eval = eval_result_to_experiment_eval(config_name, sticky_result);
                make_experiment_from_sticky_value(eval, sticky_value)
            })
    }

    pub fn try_apply_sticky_value_to_layer<'a>(
        &self,
        user: &'a StatsigUserInternal,
        option: &'a LayerEvaluationOptions,
        evaluator_result: &'a Layer,
        event_logger_ptr: Option<Weak<EventLogger>>,
        sampling_processor: Option<Weak<SamplingProcessor>>,
        disable_exposure: bool,
    ) -> Option<Layer> {
        let config_name = evaluator_result.name.as_str();
        let (mut maybe_sticky_result, sticky_value_ptr) = get_sticky_result!(option, config_name);
        try_finalize_sticky_storage!(
            self,
            user,
            option,
            evaluator_result,
            maybe_sticky_result,
            sticky_value_ptr,
            make_sticky_value_from_layer
        );
        let sticky_value = unwrap_or_return!(sticky_value_ptr, None).clone();
        maybe_sticky_result.as_mut().map(|sticky_result| {
            let eval = result_to_layer_eval(&evaluator_result.name, sticky_result);
            make_layer_from_sticky_value(
                &evaluator_result.name,
                user,
                eval,
                sticky_value,
                event_logger_ptr,
                sampling_processor,
                disable_exposure,
            )
        })
    }
}
