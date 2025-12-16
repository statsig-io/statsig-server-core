use std::sync::Arc;

use chrono::Utc;

use crate::{
    evaluation::evaluation_types::{BaseEvaluation, ExperimentEvaluation, LayerEvaluation},
    get_persistent_storage_key,
    interned_string::InternedString,
    log_d, log_error_to_statsig_and_console, make_sticky_value_from_experiment,
    make_sticky_value_from_layer,
    observability::{ops_stats::OpsStatsForInstance, sdk_errors_observer::ErrorBoundaryEvent},
    read_lock_or_else,
    spec_store::SpecStoreData,
    statsig_types::{Experiment, Layer},
    user::StatsigUserInternal,
    DynamicReturnable, EvaluationDetails, ExperimentEvaluationOptions, LayerEvaluationOptions,
    PersistentStorage, SpecStore, StatsigErr, StickyValues,
};

#[cfg(feature = "ffi-support")]
use crate::evaluation::evaluator_result::EvaluatorResult;

pub struct PersistentValuesManager {
    pub persistent_storage: Arc<dyn PersistentStorage>,
}

const TAG: &str = "PersistentValuesManager";

#[cfg(feature = "ffi-support")]
impl PersistentValuesManager {
    pub fn try_apply_sticky_value_to_raw_layer<'a>(
        manager: &'a Option<Arc<PersistentValuesManager>>,
        user: &'a StatsigUserInternal,
        options: &'a LayerEvaluationOptions,
        spec_store: &Arc<SpecStore>,
        ops_stats: &Arc<OpsStatsForInstance>,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
    ) -> (Option<EvaluatorResult>, EvaluationDetails) {
        let manager = match manager {
            Some(manager) => manager,
            None => {
                return (result, details);
            }
        };

        let result = match result {
            Some(result) => result,
            None => {
                return (None, details);
            }
        };

        let spec_store_data = read_lock_or_else!(spec_store.data, {
            log_error_to_statsig_and_console!(
                &ops_stats,
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return (Some(result), details);
        });

        let (result, details) = manager.try_apply_sticky_value_to_raw_layer_impl(
            user,
            options,
            &spec_store_data,
            details,
            result,
        );

        (Some(result), details)
    }

    pub fn try_apply_sticky_value_to_raw_experiment<'a>(
        manager: &'a Option<Arc<PersistentValuesManager>>,
        user: &'a StatsigUserInternal,
        options: &'a ExperimentEvaluationOptions,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
    ) -> (Option<EvaluatorResult>, EvaluationDetails) {
        let manager = match manager {
            Some(manager) => manager,
            None => {
                return (result, details);
            }
        };

        let result = match result {
            Some(result) => result,
            None => {
                return (None, details);
            }
        };

        let (result, details) =
            manager.try_apply_sticky_value_to_raw_experiment_impl(user, options, details, result);

        (Some(result), details)
    }

    fn try_apply_sticky_value_to_raw_experiment_impl<'a>(
        &self,
        user: &'a StatsigUserInternal,
        options: &'a ExperimentEvaluationOptions,
        curr_details: EvaluationDetails,
        curr_result: EvaluatorResult,
    ) -> (EvaluatorResult, EvaluationDetails) {
        let id_type = match &curr_result.id_type {
            Some(id_type) => id_type,
            None => {
                return (curr_result, curr_details);
            }
        };

        let config_name = match &curr_result.name {
            Some(name) => name,
            None => {
                return (curr_result, curr_details);
            }
        };

        let storage_key = match get_persistent_storage_key(user.user_ref, id_type.as_str()) {
            Some(key) => key,
            None => {
                return (curr_result, curr_details);
            }
        };

        // Exit Early: Caller does not want sticky, or experiment is not active
        if options.user_persisted_values.is_none() || !curr_result.is_experiment_active {
            self.delete_sticky_value(&storage_key, config_name);
            return (curr_result, curr_details);
        }

        let sticky_value = options
            .user_persisted_values
            .as_ref()
            .and_then(|values| values.get(config_name.as_str()));

        // Exit Early: Found a Sticky Value
        if let Some(found) = sticky_value {
            let sticky_details = Self::make_evaluation_details_from_sticky_value(found);
            let sticky_result = Self::make_evaluation_result_from_sticky_value(curr_result, found);
            return (sticky_result, sticky_details);
        }

        if curr_result.is_experiment_active && curr_result.is_experiment_group {
            let new_sticky_value =
                Self::make_sticky_value_from_evaluation_result(&curr_result, curr_details.lcut);

            self.persistent_storage
                .save(&storage_key, config_name, new_sticky_value);
        }

        (curr_result, curr_details)
    }

    fn try_apply_sticky_value_to_raw_layer_impl<'a>(
        &self,
        user: &'a StatsigUserInternal,
        options: &'a LayerEvaluationOptions,
        spec_store_data: &'a SpecStoreData,
        curr_details: EvaluationDetails,
        curr_result: EvaluatorResult,
    ) -> (EvaluatorResult, EvaluationDetails) {
        let id_type = match &curr_result.id_type {
            Some(id_type) => id_type,
            None => {
                return (curr_result, curr_details);
            }
        };

        let config_name = match &curr_result.name {
            Some(name) => name,
            None => {
                return (curr_result, curr_details);
            }
        };

        let storage_key = match get_persistent_storage_key(user.user_ref, id_type.as_str()) {
            Some(key) => key,
            None => {
                return (curr_result, curr_details);
            }
        };

        let sticky_value = options
            .user_persisted_values
            .as_ref()
            .and_then(|values| values.get(config_name.as_str()));

        let is_experiment_active = self.get_sticky_aware_is_experiment_active(
            curr_result.is_experiment_active,
            spec_store_data,
            sticky_value,
        );

        // Exit Early: Caller does not want sticky, or experiment is not active
        if options.user_persisted_values.is_none() || !is_experiment_active {
            self.delete_sticky_value(&storage_key, config_name);
            return (curr_result, curr_details);
        }

        // Exit Early: Found a Sticky Value
        if let Some(found) = sticky_value {
            let sticky_details = Self::make_evaluation_details_from_sticky_value(found);
            let sticky_result = Self::make_evaluation_result_from_sticky_value(curr_result, found);
            return (sticky_result, sticky_details);
        }

        if curr_result.is_experiment_active && curr_result.is_experiment_group {
            let new_sticky_value =
                Self::make_sticky_value_from_evaluation_result(&curr_result, curr_details.lcut);

            self.persistent_storage
                .save(&storage_key, config_name, new_sticky_value);
        }

        (curr_result, curr_details)
    }

    // -------------------------------------------------------------------------- [ Raw Experiment/Layer Helpers ]

    fn make_evaluation_details_from_sticky_value(sticky_value: &StickyValues) -> EvaluationDetails {
        EvaluationDetails {
            reason: "Persisted".to_owned(),
            lcut: sticky_value.time,
            received_at: Some(Utc::now().timestamp_millis() as u64),
            version: sticky_value.config_version,
        }
    }

    fn make_evaluation_result_from_sticky_value(
        curr_result: EvaluatorResult,
        sticky_value: &StickyValues,
    ) -> EvaluatorResult {
        EvaluatorResult {
            // transfer from current
            name: curr_result.name,
            id_type: curr_result.id_type,
            bool_value: sticky_value.value,

            // clone from sticky
            rule_id: sticky_value.rule_id.clone(),
            group_name: sticky_value.group_name.clone(),
            json_value: sticky_value.json_value.clone(),
            version: sticky_value.config_version,
            config_delegate: sticky_value.config_delegate.clone(),
            explicit_parameters: sticky_value.explicit_parameters.clone(),
            secondary_exposures: sticky_value.secondary_exposures.clone(),
            undelegated_secondary_exposures: sticky_value.undelegated_secondary_exposures.clone(),

            // these are always true for sticky values
            is_experiment_active: true,
            is_experiment_group: true,

            // Not yet consumed by raw logic
            unsupported: false,
            is_in_layer: false,
            rule_id_suffix: None,
            override_reason: None,
            sampling_rate: None,
            forward_all_exposures: None,
            override_config_name: None,
            has_seen_analytical_gates: None,
            parameter_rule_ids: None,
        }
    }

    fn make_sticky_value_from_evaluation_result(
        curr_result: &EvaluatorResult,
        lcut: Option<u64>,
    ) -> StickyValues {
        StickyValues {
            value: curr_result.bool_value,
            json_value: curr_result.json_value.clone(),
            rule_id: curr_result.rule_id.clone(),
            group_name: curr_result.group_name.clone(),
            secondary_exposures: curr_result.secondary_exposures.clone(),
            undelegated_secondary_exposures: curr_result.undelegated_secondary_exposures.clone(),
            config_delegate: curr_result.config_delegate.clone(),
            explicit_parameters: curr_result.explicit_parameters.clone(),
            time: lcut,
            config_version: curr_result.version,
        }
    }
}

impl PersistentValuesManager {
    pub fn try_apply_sticky_value_to_experiment<'a>(
        manager: &'a Option<Arc<PersistentValuesManager>>,
        user: &'a StatsigUserInternal,
        options: &'a ExperimentEvaluationOptions,
        curr_experiment: Experiment,
    ) -> Experiment {
        let manager = match manager {
            Some(manager) => manager,
            None => return curr_experiment,
        };

        manager.try_apply_sticky_value_to_experiment_impl(user, options, curr_experiment)
    }

    fn try_apply_sticky_value_to_experiment_impl<'a>(
        &self,
        user: &'a StatsigUserInternal,
        options: &'a ExperimentEvaluationOptions,
        curr_experiment: Experiment,
    ) -> Experiment {
        // 1. Check if the caller requested sticky, if not, delete and return current
        // 2. Check if current is active, if not, delete and return current
        // 3. Check if there is a sticky value for the experiment, if so,
        //    return the found sticky experiment, dropping the current experiment
        // 4. If no sticky value was found, and the current experiment is active,
        //    create a new sticky value and save it, returning the current experiment

        let id_type = curr_experiment.id_type.as_str();
        let storage_key = match get_persistent_storage_key(user.user_ref, id_type) {
            Some(key) => key,
            None => return curr_experiment,
        };

        let config_name = curr_experiment.name.as_str();
        let is_experiment_active = curr_experiment.is_experiment_active;

        // Exit Early: Caller does not want sticky, or experiment is not active
        if options.user_persisted_values.is_none() || !is_experiment_active {
            log_d!(
                TAG,
                "No sticky value found for experiment, deleting sticky value if it exists"
            );
            self.delete_sticky_value(&storage_key, config_name);
            return curr_experiment;
        }

        let sticky_value = options
            .user_persisted_values
            .as_ref()
            .and_then(|values| values.get(config_name));

        // Exit Early: Found a Sticky Value
        if let Some(found) = sticky_value {
            return make_experiment_from_sticky_value(curr_experiment, found);
        }

        let is_in_experiment = curr_experiment
            .__evaluation
            .as_ref()
            .and_then(|e| e.is_user_in_experiment)
            .unwrap_or_default();

        if is_in_experiment && is_experiment_active {
            let new_sticky_value = match make_sticky_value_from_experiment(&curr_experiment) {
                Some(sticky_value) => sticky_value,
                None => return curr_experiment,
            };

            self.persistent_storage
                .save(&storage_key, config_name, new_sticky_value);
        }

        curr_experiment
    }

    pub fn try_apply_sticky_value_to_layer<'a>(
        manager: &'a Option<Arc<PersistentValuesManager>>,
        user: &'a StatsigUserInternal,
        options: &'a LayerEvaluationOptions,
        spec_store: &Arc<SpecStore>,
        ops_stats: &Arc<OpsStatsForInstance>,
        curr_layer: Layer,
    ) -> Layer {
        let manager = match manager {
            Some(manager) => manager,
            None => return curr_layer,
        };

        let data = read_lock_or_else!(spec_store.data, {
            log_error_to_statsig_and_console!(
                &ops_stats,
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return curr_layer;
        });

        manager.try_apply_sticky_value_to_layer_impl(user, options, &data, curr_layer)
    }

    fn try_apply_sticky_value_to_layer_impl<'a>(
        &self,
        user: &'a StatsigUserInternal,
        options: &'a LayerEvaluationOptions,
        spec_store_data: &'a SpecStoreData,
        curr_layer: Layer,
    ) -> Layer {
        // Logic is similar to the experiment flow, but we need to be sure to check if the sticky experiment is still active using the spec store data

        let id_type = curr_layer.id_type.as_str();
        let config_name = curr_layer.name.as_str();
        let storage_key = match get_persistent_storage_key(user.user_ref, id_type) {
            Some(key) => key,
            None => return curr_layer,
        };

        let sticky_value = options
            .user_persisted_values
            .as_ref()
            .and_then(|values| values.get(config_name));

        let is_experiment_active = self.get_sticky_aware_is_experiment_active(
            curr_layer.is_experiment_active,
            spec_store_data,
            sticky_value,
        );

        // Exit Early: No provided values, or sticky experiment/layer is not active
        if options.user_persisted_values.is_none() || !is_experiment_active {
            self.delete_sticky_value(&storage_key, config_name);
            return curr_layer;
        }

        // Exit Early: Found a Sticky Value
        if let Some(found) = sticky_value {
            return make_layer_from_sticky_value(curr_layer, found);
        }

        let is_in_experiment = curr_layer
            .__evaluation
            .as_ref()
            .and_then(|e| e.is_user_in_experiment)
            .unwrap_or_default();

        if is_in_experiment && is_experiment_active {
            let new_sticky_value = match make_sticky_value_from_layer(&curr_layer) {
                Some(sticky_value) => sticky_value,
                None => return curr_layer,
            };

            self.persistent_storage
                .save(&storage_key, config_name, new_sticky_value);
        }

        curr_layer
    }

    fn delete_sticky_value(&self, storage_key: &str, config_name: &str) {
        self.persistent_storage.delete(storage_key, config_name);
    }

    fn get_sticky_aware_is_experiment_active(
        &self,
        layer_is_active: bool,
        spec_store_data: &SpecStoreData,
        sticky_value: Option<&StickyValues>,
    ) -> bool {
        let fallback = layer_is_active;

        let sticky_values = match sticky_value {
            Some(sticky_value) => sticky_value,
            None => return fallback,
        };

        let config_delegate = match sticky_values.config_delegate.as_ref() {
            Some(delegate) => InternedString::from_str_ref(delegate.as_str()),
            None => return fallback,
        };

        let delegate = spec_store_data.values.dynamic_configs.get(&config_delegate);
        match delegate {
            Some(delegate) => delegate.inner.is_active.unwrap_or(fallback),
            None => fallback,
        }
    }
}

// -------------------------------------------------------------------------- [ Experiment Helpers ]

fn make_experiment_from_sticky_value(
    curr_experiment: Experiment,
    sticky_value: &StickyValues,
) -> Experiment {
    let details = EvaluationDetails {
        reason: "Persisted".to_owned(),
        lcut: sticky_value.time,
        received_at: Some(Utc::now().timestamp_millis() as u64),
        version: sticky_value.config_version,
    };

    let (sticky_rule_id, sticky_rule_id_string) = prep_sticky_rule_id(sticky_value);
    let (sticky_group_name, sticky_group_name_string) = prep_sticky_group_name(sticky_value);

    let evaluation = sticky_value_to_experiment_evaluation(
        &curr_experiment,
        sticky_value,
        sticky_rule_id,
        sticky_group_name,
    );

    let value = sticky_value
        .json_value
        .as_ref()
        .and_then(|v| v.get_json())
        .unwrap_or_default();

    Experiment {
        // transfer from current
        name: curr_experiment.name,
        id_type: curr_experiment.id_type,

        // clone from sticky
        value,
        rule_id: sticky_rule_id_string,
        group_name: sticky_group_name_string,

        // created new
        details,
        is_experiment_active: true,
        __evaluation: Some(evaluation),
    }
}

fn sticky_value_to_experiment_evaluation(
    curr_experiment: &Experiment,
    sticky_value: &StickyValues,
    sticky_rule_id: InternedString,
    sticky_group_name: Option<InternedString>,
) -> ExperimentEvaluation {
    let name = InternedString::from_str_ref(curr_experiment.name.as_str());
    let id_type = InternedString::from_str_ref(curr_experiment.id_type.as_str());

    let mut is_device_based = false;
    let mut is_in_layer = false;
    if let Some(evaluation) = curr_experiment.__evaluation.as_ref() {
        is_device_based = evaluation.is_device_based;
        is_in_layer = evaluation.is_in_layer;
    }

    ExperimentEvaluation {
        // transfer from current
        is_device_based,
        is_in_layer,
        id_type: Some(id_type),

        // clone from sticky
        base: BaseEvaluation {
            name,

            rule_id: sticky_rule_id,
            secondary_exposures: sticky_value.secondary_exposures.clone(),
            exposure_info: None,
        },
        value: sticky_value
            .json_value
            .clone()
            .unwrap_or_else(DynamicReturnable::empty),
        explicit_parameters: sticky_value.explicit_parameters.clone(),
        group_name: sticky_group_name,
        undelegated_secondary_exposures: sticky_value.undelegated_secondary_exposures.clone(),

        // created new
        is_experiment_active: Some(true),
        is_user_in_experiment: Some(true),
    }
}

// -------------------------------------------------------------------------- [ Layer Helpers ]

fn make_layer_from_sticky_value(curr_layer: Layer, sticky_value: &StickyValues) -> Layer {
    let details = EvaluationDetails {
        reason: "Persisted".to_owned(),
        lcut: sticky_value.time,
        received_at: Some(Utc::now().timestamp_millis() as u64),
        version: sticky_value.config_version,
    };

    let (sticky_rule_id, sticky_rule_id_string) = prep_sticky_rule_id(sticky_value);
    let (sticky_group_name, sticky_group_name_string) = prep_sticky_group_name(sticky_value);
    let (sticky_config_delegate, sticky_config_delegate_string) =
        prep_sticky_config_delegate(sticky_value);

    let evaluation = sticky_value_to_layer_evaluation(
        &curr_layer,
        sticky_value,
        sticky_rule_id,
        sticky_group_name,
        sticky_config_delegate,
    );

    let value = sticky_value
        .json_value
        .as_ref()
        .and_then(|v| v.get_json())
        .unwrap_or_default();

    Layer {
        // transfer from current
        name: curr_layer.name,
        id_type: curr_layer.id_type,
        __user: curr_layer.__user,
        __event_logger_ptr: curr_layer.__event_logger_ptr,
        __disable_exposure: curr_layer.__disable_exposure,

        // clone from sticky
        rule_id: sticky_rule_id_string,
        group_name: sticky_group_name_string,
        allocated_experiment_name: sticky_config_delegate_string,
        __version: sticky_value.config_version,

        // created new
        details,
        is_experiment_active: true,
        __value: value,
        __evaluation: Some(evaluation),

        // not yet supported
        __parameter_rule_ids: None,
    }
}

fn sticky_value_to_layer_evaluation(
    curr_layer: &Layer,
    sticky_value: &StickyValues,
    sticky_rule_id: InternedString,
    sticky_group_name: Option<InternedString>,
    sticky_config_delegate: Option<InternedString>,
) -> LayerEvaluation {
    let name = InternedString::from_str_ref(curr_layer.name.as_str());
    let id_type = InternedString::from_str_ref(curr_layer.id_type.as_str());

    let mut is_device_based = false;
    if let Some(evaluation) = curr_layer.__evaluation.as_ref() {
        is_device_based = evaluation.is_device_based;
    }

    LayerEvaluation {
        // transfer from current
        id_type: Some(id_type),
        is_device_based,

        // clone from sticky
        base: BaseEvaluation {
            name,

            rule_id: sticky_rule_id,
            secondary_exposures: sticky_value.secondary_exposures.clone(),
            exposure_info: None,
        },
        value: sticky_value
            .json_value
            .clone()
            .unwrap_or_else(DynamicReturnable::empty),
        explicit_parameters: sticky_value.explicit_parameters.clone().unwrap_or_default(),
        group_name: sticky_group_name,
        undelegated_secondary_exposures: sticky_value.undelegated_secondary_exposures.clone(),
        allocated_experiment_name: sticky_config_delegate,

        // created new
        is_experiment_active: Some(true),
        is_user_in_experiment: Some(true),

        // not yet supported
        parameter_rule_ids: None,
    }
}

fn prep_sticky_rule_id(sticky_value: &StickyValues) -> (InternedString, String) {
    match sticky_value.rule_id.as_ref() {
        Some(rule_id) => (rule_id.clone(), rule_id.unperformant_to_string()),
        None => (InternedString::empty(), "".to_owned()),
    }
}

fn prep_sticky_group_name(sticky_value: &StickyValues) -> (Option<InternedString>, Option<String>) {
    let group_name = match sticky_value.group_name.as_ref() {
        Some(group_name) => group_name.clone(),
        None => return (None, None),
    };

    let as_string = group_name.unperformant_to_string();
    (Some(group_name), Some(as_string))
}

fn prep_sticky_config_delegate(
    sticky_value: &StickyValues,
) -> (Option<InternedString>, Option<String>) {
    let config_delegate = match sticky_value.config_delegate.as_ref() {
        Some(config_delegate) => config_delegate.clone(),
        None => return (None, None),
    };

    let as_string = config_delegate.unperformant_to_string();
    (Some(config_delegate), Some(as_string))
}
