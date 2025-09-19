use std::sync::{Arc, Weak};

use crate::{
    evaluation::{
        dynamic_returnable::DynamicReturnable,
        evaluator_result::{eval_result_to_experiment_eval, result_to_layer_eval, EvaluatorResult},
    },
    event_logging::event_logger::EventLogger,
    get_persistent_storage_key,
    interned_string::InternedString,
    log_d, log_e, make_experiment_from_sticky_value, make_layer_from_sticky_value,
    make_sticky_value_from_experiment, make_sticky_value_from_layer,
    spec_store::SpecStoreData,
    statsig_types::{Experiment, Layer},
    unwrap_or_return,
    user::StatsigUserInternal,
    ExperimentEvaluationOptions, LayerEvaluationOptions, PersistentStorage, StickyValues,
};
pub struct PersistentValuesManager {
    pub persistent_storage: Arc<dyn PersistentStorage>,
}

const TAG: &str = "PersistentValuesManager";

fn sticky_value_to_evaluator_result(sticky_value: &StickyValues) -> EvaluatorResult<'_> {
    EvaluatorResult {
        bool_value: sticky_value.value,
        json_value: Some(DynamicReturnable::from_map(
            sticky_value.json_value.clone().unwrap_or_default(),
        )),
        rule_id: sticky_value.rule_id.as_ref(),
        group_name: sticky_value
            .group_name
            .as_ref()
            .map(|g| InternedString::from_str_ref(g.as_str())),
        secondary_exposures: sticky_value.secondary_exposures.clone(),
        undelegated_secondary_exposures: sticky_value.undelegated_secondary_exposures.clone(),
        config_delegate: sticky_value
            .config_delegate
            .as_ref()
            .map(|g| InternedString::from_str_ref(g.as_str())),
        version: sticky_value.config_version,
        ..Default::default()
    }
}

macro_rules! get_sticky_result {
    ($option:expr, $config_name:expr) => {{
        let mut sticky_value_ptr = None;
        let maybe_sticky_result = match $option.user_persisted_values.as_ref() {
            Some(values) => match values.get($config_name) {
                Some(sticky_value) => {
                    sticky_value_ptr = Some(sticky_value);
                    let sticky_result = sticky_value_to_evaluator_result(sticky_value);
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
    ($self: expr, $user: expr, $option:expr, $evaluator_result:expr, $maybe_sticky_result:expr, $sticky_value_ptr: expr,  $make_sticky_value_fn:expr, $is_experiment_active:expr) => {{
        let config_name = $evaluator_result.name.as_str();
        let is_in_experiment = $evaluator_result
            .__evaluation
            .as_ref()
            .and_then(|e| e.is_user_in_experiment)
            .unwrap_or_default();
        let is_experiment_active = match $is_experiment_active {
            Some(active) => active,
            None => $evaluator_result
                .__evaluation
                .as_ref()
                .and_then(|e| e.is_experiment_active)
                .unwrap_or_default(),
        };
        let storage_key = unwrap_or_return!(
            get_persistent_storage_key($user.user_ref, &$evaluator_result.id_type),
            None
        );
        if $option.user_persisted_values.is_none() || !is_experiment_active {
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
        let is_experiment_active = evaluator_result
            .__evaluation
            .as_ref()
            .and_then(|e| e.is_experiment_active)
            .unwrap_or_default();
        try_finalize_sticky_storage!(
            self,
            user,
            option,
            evaluator_result,
            maybe_sticky_result,
            sticky_value_ptr,
            make_sticky_value_from_experiment,
            Some(is_experiment_active)
        );
        let sticky_value = unwrap_or_return!(sticky_value_ptr, None).clone();
        maybe_sticky_result
            .as_mut()
            .map(|sticky_result: &mut EvaluatorResult<'_>| {
                let eval = eval_result_to_experiment_eval(config_name, sticky_result);
                make_experiment_from_sticky_value(eval, sticky_value)
            })
    }

    pub fn try_update_sticky_layer_experiment_active<'a>(
        &self,
        spec_store_data: &'a SpecStoreData,
        maybe_sticky_result: &Option<EvaluatorResult<'a>>,
    ) -> Option<bool> {
        let sticky_result = unwrap_or_return!(maybe_sticky_result, None);
        let config_delegate = unwrap_or_return!(&sticky_result.config_delegate, Some(false));
        let delegate_spec = spec_store_data
            .values
            .dynamic_configs
            .get(config_delegate.as_str());
        match delegate_spec {
            Some(delegate_spec) => Some(delegate_spec.spec.is_active.unwrap_or(false)),
            None => Some(false),
        }
    }

    pub fn try_apply_sticky_value_to_layer<'a>(
        &self,
        user: &'a StatsigUserInternal,
        option: &'a LayerEvaluationOptions,
        evaluator_result: &'a Layer,
        event_logger_ptr: Option<Weak<EventLogger>>,
        disable_exposure: bool,
        spec_store_data: &'a SpecStoreData,
    ) -> Option<Layer> {
        let config_name = evaluator_result.name.as_str();
        let (mut maybe_sticky_result, sticky_value_ptr) = get_sticky_result!(option, config_name);
        let is_experiment_active =
            self.try_update_sticky_layer_experiment_active(spec_store_data, &maybe_sticky_result);
        try_finalize_sticky_storage!(
            self,
            user,
            option,
            evaluator_result,
            maybe_sticky_result,
            sticky_value_ptr,
            make_sticky_value_from_layer,
            is_experiment_active
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
                disable_exposure,
            )
        })
    }
}
