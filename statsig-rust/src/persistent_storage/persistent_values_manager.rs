use std::sync::Arc;

use chrono::Utc;

use crate::{
    evaluation::evaluation_types::{
        BaseEvaluation, ExperimentEvaluation, ExtraExposureInfo, LayerEvaluation,
    },
    evaluation::evaluator::SpecType,
    get_persistent_storage_key,
    interned_string::InternedString,
    log_d, make_sticky_value_from_experiment, make_sticky_value_from_layer,
    spec_store::SpecStoreData,
    statsig_types::{Experiment, Layer},
    user::StatsigUserInternal,
    DynamicReturnable, EvaluationDetails, ExperimentEvaluationOptions, LayerEvaluationOptions,
    PersistentStorage, StickyValues,
};

#[cfg(feature = "ffi-support")]
use crate::evaluation::evaluator_result::EvaluatorResult;

pub struct PersistentValuesManager {
    pub persistent_storage: Arc<dyn PersistentStorage>,
}

const TAG: &str = "PersistentValuesManager";

/// Which subset of a spec's rules to consider when re-evaluating to decide
/// whether a persisted sticky value should still be honored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StickyRuleFilter {
    Targeting,
    Overrides,
}

/// Re-evaluates a spec (an experiment / dynamic config) restricted to a single
/// rule subset, returning the resulting boolean value. Supplied by `Statsig` so
/// the manager can decide whether a persisted sticky value should still win
/// without holding an evaluator or spec store reference itself.
pub type StickyReeval<'a> = &'a dyn Fn(&str, &SpecType, StickyRuleFilter) -> bool;

/// Mirrors the legacy Java server SDK `Evaluator.evaluateShouldReturnSticky`.
///
/// Returns `true` if the persisted sticky value should be honored, and `false`
/// if the live evaluation should win instead because the user no longer passes
/// targeting (`enforce_targeting`) or a matching console override rule applies
/// (`enforce_overrides`).
fn should_return_sticky_value(
    enforce_overrides: bool,
    enforce_targeting: bool,
    reeval_spec_name: &str,
    reeval: StickyReeval,
) -> bool {
    if enforce_targeting {
        let excluded_by_targeting = reeval(
            reeval_spec_name,
            &SpecType::Experiment,
            StickyRuleFilter::Targeting,
        );
        if excluded_by_targeting {
            return false;
        }
    }

    if enforce_overrides {
        let is_overridden = reeval(
            reeval_spec_name,
            &SpecType::Experiment,
            StickyRuleFilter::Overrides,
        );
        if is_overridden {
            return false;
        }
    }

    true
}

/// Layer variant of [`should_return_sticky_value`]. A layer's sticky value is
/// backed by an allocated experiment (`config_delegate`); that experiment is the
/// spec re-evaluated for override / targeting enforcement. When there is no
/// delegate there is nothing to re-evaluate, so the sticky value is honored.
///
/// Note: honoring a delegate-less sticky value intentionally deviates from the
/// legacy Java server SDK, which deletes the sticky value and re-evaluates live
/// when the delegate spec cannot be resolved (`Evaluator.kt`,
/// `evaluateLayerImpl`). It instead matches the pre-existing Rust behavior
/// (`get_sticky_aware_is_experiment_active` falls back to the live layer's
/// active state). The SDK never writes a layer sticky value without a delegate,
/// so this only affects corrupted or legacy storage entries.
fn layer_should_return_sticky_value(
    options: &LayerEvaluationOptions,
    sticky_value: &StickyValues,
    reeval: StickyReeval,
) -> bool {
    if !(options.enforce_overrides || options.enforce_targeting) {
        return true;
    }

    let delegate = match sticky_value.config_delegate.as_ref() {
        Some(delegate) => delegate,
        None => return true,
    };

    should_return_sticky_value(
        options.enforce_overrides,
        options.enforce_targeting,
        delegate.as_str(),
        reeval,
    )
}

#[cfg(feature = "ffi-support")]
impl PersistentValuesManager {
    pub fn try_apply_sticky_value_to_raw_layer<'a>(
        manager: &'a Option<Arc<PersistentValuesManager>>,
        user: &'a StatsigUserInternal,
        options: &'a LayerEvaluationOptions,
        spec_store_data: &SpecStoreData,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
        reeval: StickyReeval,
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

        let (result, details) = manager.try_apply_sticky_value_to_raw_layer_impl(
            user,
            options,
            spec_store_data,
            details,
            result,
            reeval,
        );

        (Some(result), details)
    }

    pub fn try_apply_sticky_value_to_raw_experiment<'a>(
        manager: &'a Option<Arc<PersistentValuesManager>>,
        user: &'a StatsigUserInternal,
        options: &'a ExperimentEvaluationOptions,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
        reeval: StickyReeval,
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

        let (result, details) = manager
            .try_apply_sticky_value_to_raw_experiment_impl(user, options, details, result, reeval);

        (Some(result), details)
    }

    fn try_apply_sticky_value_to_raw_experiment_impl<'a>(
        &self,
        user: &'a StatsigUserInternal,
        options: &'a ExperimentEvaluationOptions,
        curr_details: EvaluationDetails,
        curr_result: EvaluatorResult,
        reeval: StickyReeval,
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
            let keep_sticky = !(options.enforce_overrides || options.enforce_targeting)
                || should_return_sticky_value(
                    options.enforce_overrides,
                    options.enforce_targeting,
                    config_name.as_str(),
                    reeval,
                );

            if keep_sticky {
                let sticky_details = Self::make_evaluation_details_from_sticky_value(found);
                let sticky_result =
                    Self::make_evaluation_result_from_sticky_value(curr_result, found);
                return (sticky_result, sticky_details);
            }

            // enforceOverrides / enforceTargeting: the live evaluation wins, so
            // return it as-is without persisting a new sticky value.
            return (curr_result, curr_details);
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
        reeval: StickyReeval,
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
            if !layer_should_return_sticky_value(options, found, reeval) {
                // enforceOverrides / enforceTargeting: the live evaluation wins.
                return (curr_result, curr_details);
            }

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

            unsupported: false,
            is_in_layer: false,
            rule_id_suffix: None,
            override_reason: None,
            sampling_rate: curr_result.sampling_rate,
            forward_all_exposures: curr_result.forward_all_exposures,
            override_config_name: curr_result.override_config_name,
            has_seen_analytical_gates: curr_result.has_seen_analytical_gates,
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
        reeval: StickyReeval,
    ) -> Experiment {
        let manager = match manager {
            Some(manager) => manager,
            None => return curr_experiment,
        };

        manager.try_apply_sticky_value_to_experiment_impl(user, options, curr_experiment, reeval)
    }

    fn try_apply_sticky_value_to_experiment_impl<'a>(
        &self,
        user: &'a StatsigUserInternal,
        options: &'a ExperimentEvaluationOptions,
        curr_experiment: Experiment,
        reeval: StickyReeval,
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
            let keep_sticky = !(options.enforce_overrides || options.enforce_targeting)
                || should_return_sticky_value(
                    options.enforce_overrides,
                    options.enforce_targeting,
                    config_name,
                    reeval,
                );

            if keep_sticky {
                return make_experiment_from_sticky_value(curr_experiment, found);
            }

            // enforceOverrides / enforceTargeting: the live evaluation wins, so
            // return it as-is without persisting a new sticky value.
            return curr_experiment;
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
        spec_store_data: &SpecStoreData,
        curr_layer: Layer,
        reeval: StickyReeval,
    ) -> Layer {
        let manager = match manager {
            Some(manager) => manager,
            None => return curr_layer,
        };

        manager.try_apply_sticky_value_to_layer_impl(
            user,
            options,
            spec_store_data,
            curr_layer,
            reeval,
        )
    }

    fn try_apply_sticky_value_to_layer_impl<'a>(
        &self,
        user: &'a StatsigUserInternal,
        options: &'a LayerEvaluationOptions,
        spec_store_data: &'a SpecStoreData,
        curr_layer: Layer,
        reeval: StickyReeval,
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
            if !layer_should_return_sticky_value(options, found, reeval) {
                // enforceOverrides / enforceTargeting: the live evaluation wins.
                return curr_layer;
            }

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
            Some(delegate) => delegate.as_spec_ref().is_active.unwrap_or(fallback),
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
            exposure_info: curr_experiment
                .__evaluation
                .as_ref()
                .and_then(|eval| eval.base.exposure_info.clone()),
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
    let exposure_info = get_current_layer_exposure_info(&curr_layer);

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
        __exposure_info: exposure_info,

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
            exposure_info: get_current_layer_exposure_info(curr_layer),
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

fn get_current_layer_exposure_info(curr_layer: &Layer) -> Option<ExtraExposureInfo> {
    curr_layer
        .__evaluation
        .as_ref()
        .and_then(|eval| eval.base.exposure_info.clone())
        .or(curr_layer.__exposure_info.clone())
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

#[cfg(test)]
mod sticky_gate_tests {
    use super::{layer_should_return_sticky_value, should_return_sticky_value, StickyRuleFilter};
    use crate::{LayerEvaluationOptions, StickyValues};
    use std::cell::RefCell;

    #[test]
    fn test_no_enforcement_keeps_sticky() {
        // With neither flag set the gate is never consulted; sticky always wins.
        let reeval = |_: &str, _: &_, _: StickyRuleFilter| panic!("should not re-evaluate");
        assert!(should_return_sticky_value(false, false, "exp", &reeval));
    }

    #[test]
    fn test_enforce_overrides_override_matches_drops_sticky() {
        let reeval =
            |_: &str, _: &_, filter: StickyRuleFilter| filter == StickyRuleFilter::Overrides;
        // Override rule matches -> live evaluation should win.
        assert!(!should_return_sticky_value(true, false, "exp", &reeval));
    }

    #[test]
    fn test_enforce_overrides_no_override_keeps_sticky() {
        let reeval = |_: &str, _: &_, _: StickyRuleFilter| false;
        assert!(should_return_sticky_value(true, false, "exp", &reeval));
    }

    #[test]
    fn test_enforce_targeting_not_targeted_drops_sticky() {
        // A truthy targeting evaluation means the user is gated out of targeting.
        let reeval =
            |_: &str, _: &_, filter: StickyRuleFilter| filter == StickyRuleFilter::Targeting;
        assert!(!should_return_sticky_value(false, true, "exp", &reeval));
    }

    #[test]
    fn test_enforce_targeting_still_targeted_keeps_sticky() {
        let reeval = |_: &str, _: &_, _: StickyRuleFilter| false;
        assert!(should_return_sticky_value(false, true, "exp", &reeval));
    }

    #[test]
    fn test_enforce_targeting_short_circuits_before_overrides() {
        let seen = RefCell::new(Vec::new());
        let reeval = |_: &str, _: &_, filter: StickyRuleFilter| {
            seen.borrow_mut().push(filter);
            // Targeting excludes the user.
            filter == StickyRuleFilter::Targeting
        };
        assert!(!should_return_sticky_value(true, true, "exp", &reeval));
        // Overrides must not be consulted once targeting already drops the sticky value.
        assert_eq!(*seen.borrow(), vec![StickyRuleFilter::Targeting]);
    }

    #[test]
    fn test_layer_without_delegate_keeps_sticky() {
        let options = LayerEvaluationOptions {
            enforce_overrides: true,
            enforce_targeting: true,
            ..Default::default()
        };
        let sticky = StickyValues {
            config_delegate: None,
            ..Default::default()
        };
        let reeval = |_: &str, _: &_, _: StickyRuleFilter| panic!("no delegate to re-evaluate");
        assert!(layer_should_return_sticky_value(&options, &sticky, &reeval));
    }

    #[test]
    fn test_layer_with_delegate_defers_to_gate() {
        let options = LayerEvaluationOptions {
            enforce_overrides: true,
            ..Default::default()
        };
        let sticky = StickyValues {
            config_delegate: Some(crate::interned_string::InternedString::from_str_ref(
                "allocated_experiment",
            )),
            ..Default::default()
        };
        let seen = RefCell::new(None);
        let reeval = |name: &str, _: &_, _: StickyRuleFilter| {
            *seen.borrow_mut() = Some(name.to_string());
            true // override matches -> drop sticky
        };
        assert!(!layer_should_return_sticky_value(
            &options, &sticky, &reeval
        ));
        assert_eq!(seen.borrow().as_deref(), Some("allocated_experiment"));
    }
}
