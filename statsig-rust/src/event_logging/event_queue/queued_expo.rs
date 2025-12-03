use std::{collections::HashMap, fmt::Display};

use chrono::Utc;

use crate::{
    evaluation::{evaluation_types::ExtraExposureInfo, evaluator_result::EvaluatorResult},
    event_logging::{
        event_logger::ExposureTrigger,
        exposure_sampling::{EvtSamplingDecision, ExposureSamplingKey},
        exposure_utils::get_statsig_metadata_with_sampling_decision,
        statsig_event::StatsigEvent,
        statsig_event_internal::{
            StatsigEventInternal, CONFIG_EXPOSURE_EVENT_NAME, GATE_EXPOSURE_EVENT_NAME,
            LAYER_EXPOSURE_EVENT_NAME,
        },
    },
    interned_string::InternedString,
    specs_response::explicit_params::ExplicitParameters,
    statsig_types_raw::PartialLayerRaw,
    user::{StatsigUserInternal, StatsigUserLoggable},
    EvaluationDetails, SecondaryExposure,
};

use super::queued_event::{EnqueueOperation, QueuedEvent, QueuedExposure};

// Flow:
// IN(EVAL) |> EnqueueOp [sampling]> QueuedEvent [bg thread]> StatsigEventInternal |> OUT(LOGGED)

pub enum UserLoggableOrInternal<'a> {
    Loggable(StatsigUserLoggable),
    Internal(&'a StatsigUserInternal<'a, 'a>),
}

pub struct EnqueueExposureOp<'a> {
    user: UserLoggableOrInternal<'a>,
    data: ExposureData,
}

pub struct ExposureData {
    pub event_name: &'static str,
    pub spec_name: InternedString,
    pub rule_id: Option<InternedString>,
    pub exposure_time: u64,
    pub trigger: ExposureTrigger,
    pub evaluation_details: EvaluationDetails,
    pub secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub version: Option<u32>,
    pub override_spec_name: Option<InternedString>,
    pub exposure_info: Option<ExtraExposureInfo>,

    // Gate Only
    pub gate_value: Option<bool>,

    // DynamicConfig
    pub rule_passed: Option<bool>,

    // Experiment Only
    pub is_user_in_experiment: Option<bool>,

    // Layer Only
    pub parameter_name: Option<InternedString>,
    pub explicit_params: Option<ExplicitParameters>,
    pub allocated_experiment: Option<InternedString>,
}

impl<'a> EnqueueExposureOp<'a> {
    pub fn gate_exposure(
        user: &'a StatsigUserInternal<'a, 'a>,
        spec_name: &InternedString,
        trigger: ExposureTrigger,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
    ) -> Self {
        let gate_value = result.as_ref().is_some_and(|r| r.bool_value);
        let mut op = Self::new(
            GATE_EXPOSURE_EVENT_NAME,
            UserLoggableOrInternal::Internal(user),
            spec_name,
            trigger,
            details,
            result,
        );

        op.data.gate_value = Some(gate_value);
        op
    }

    pub fn dynamic_config_exposure(
        user: &'a StatsigUserInternal<'a, 'a>,
        spec_name: &InternedString,
        trigger: ExposureTrigger,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
    ) -> Self {
        let rule_passed = result.as_ref().is_some_and(|r| r.bool_value);

        let mut op = Self::new(
            CONFIG_EXPOSURE_EVENT_NAME,
            UserLoggableOrInternal::Internal(user),
            spec_name,
            trigger,
            details,
            result,
        );

        op.data.rule_passed = Some(rule_passed);
        op
    }

    pub fn experiment_exposure(
        user: &'a StatsigUserInternal<'a, 'a>,
        spec_name: &InternedString,
        trigger: ExposureTrigger,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
    ) -> Self {
        let is_user_in_experiment = result.as_ref().is_some_and(|r| r.is_experiment_group);
        let mut op = Self::new(
            CONFIG_EXPOSURE_EVENT_NAME,
            UserLoggableOrInternal::Internal(user),
            spec_name,
            trigger,
            details,
            result,
        );

        op.data.is_user_in_experiment = Some(is_user_in_experiment);
        op
    }

    pub fn layer_param_exposure(
        user: &'a StatsigUserInternal<'a, 'a>,
        spec_name: &InternedString,
        parameter_name: InternedString,
        trigger: ExposureTrigger,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
    ) -> Self {
        let mut result = result;
        let mut allocated_experiment = None;
        let mut explicit_params = None;
        let mut undelegated_secondary_exposures = None;

        if let Some(result) = result.as_mut() {
            allocated_experiment = result.config_delegate.clone();
            explicit_params = result.explicit_parameters.clone();
            undelegated_secondary_exposures = result.undelegated_secondary_exposures.take();
        }

        let mut op = Self::new(
            LAYER_EXPOSURE_EVENT_NAME,
            UserLoggableOrInternal::Internal(user),
            spec_name,
            trigger,
            details,
            result,
        );

        op.data.parameter_name = Some(parameter_name);
        op.data.explicit_params = explicit_params;
        op.data.allocated_experiment = allocated_experiment;
        op.data.undelegated_secondary_exposures = undelegated_secondary_exposures;

        op
    }

    pub(crate) fn layer_param_exposure_from_partial_raw(
        parameter_name: InternedString,
        trigger: ExposureTrigger,
        partial_raw: PartialLayerRaw,
    ) -> Self {
        let version = partial_raw.details.version;
        let user = UserLoggableOrInternal::Loggable(partial_raw.user);

        let mut rule_id = partial_raw
            .parameter_rule_ids
            .as_ref()
            .and_then(|ids| ids.get(&parameter_name));

        if rule_id.is_none() {
            rule_id = partial_raw.rule_id.as_ref();
        }

        let data = ExposureData {
            event_name: LAYER_EXPOSURE_EVENT_NAME,
            spec_name: partial_raw.name,
            rule_id: rule_id.cloned(),
            exposure_time: Utc::now().timestamp_millis() as u64,
            trigger,
            evaluation_details: partial_raw.details,
            secondary_exposures: partial_raw.secondary_exposures,
            undelegated_secondary_exposures: partial_raw.undelegated_secondary_exposures,
            version,
            override_spec_name: None,
            rule_passed: None,
            exposure_info: None,
            parameter_name: Some(parameter_name),
            explicit_params: partial_raw.explicit_parameters,
            allocated_experiment: partial_raw.allocated_experiment_name,
            is_user_in_experiment: None,
            gate_value: None,
        };

        Self { user, data }
    }

    fn new(
        event_name: &'static str,
        user: UserLoggableOrInternal<'a>,
        spec_name: &InternedString,
        trigger: ExposureTrigger,
        details: EvaluationDetails,
        result: Option<EvaluatorResult>,
    ) -> Self {
        let mut data = ExposureData {
            event_name,
            spec_name: spec_name.clone(),
            rule_id: None,
            exposure_time: Utc::now().timestamp_millis() as u64,
            trigger,
            evaluation_details: details,
            secondary_exposures: None,
            undelegated_secondary_exposures: None,
            version: None,
            override_spec_name: None,
            rule_passed: None,
            exposure_info: None,
            parameter_name: None,
            explicit_params: None,
            allocated_experiment: None,
            is_user_in_experiment: None,
            gate_value: None,
        };

        if let Some(result) = result {
            data.exposure_info = Some(ExtraExposureInfo {
                sampling_rate: result.sampling_rate,
                forward_all_exposures: result.forward_all_exposures,
                has_seen_analytical_gates: result.has_seen_analytical_gates,
                override_config_name: result.override_config_name.clone(),
                version: result.version,
            });

            data.rule_id = result.rule_id;
            data.version = result.version;
            data.override_spec_name = result.override_config_name;
            data.secondary_exposures = Some(result.secondary_exposures);
        }

        Self { user, data }
    }
}

impl EnqueueOperation for EnqueueExposureOp<'_> {
    fn as_exposure(&self) -> Option<&impl QueuedExposure<'_>> {
        Some(self)
    }

    fn into_queued_event(self, sampling_decision: EvtSamplingDecision) -> QueuedEvent {
        let loggable_user = match self.user {
            UserLoggableOrInternal::Loggable(loggable) => loggable,
            UserLoggableOrInternal::Internal(internal) => internal.to_loggable(),
        };

        QueuedEvent::Exposure(QueuedExposureEvent {
            user: loggable_user,
            sampling_decision,
            data: self.data,
        })
    }
}

impl<'a> QueuedExposure<'a> for EnqueueExposureOp<'a> {
    fn create_exposure_sampling_key(&self) -> ExposureSamplingKey {
        let spec_name_hash = self.data.spec_name.hash;
        let rule_id_hash = self.data.rule_id.as_ref().map_or(0, |id| id.hash);
        let user_values_hash = match &self.user {
            UserLoggableOrInternal::Loggable(loggable) => loggable.data.create_user_values_hash(),
            UserLoggableOrInternal::Internal(internal) => {
                internal.user_ref.data.create_user_values_hash()
            }
        };

        let mut additional_hash = 0u64;
        if let Some(gate_value) = self.data.gate_value {
            additional_hash = gate_value as u64;
        } else if let Some(rule_passed) = self.data.rule_passed {
            additional_hash = rule_passed as u64;
        } else if let Some(is_user_in_experiment) = self.data.is_user_in_experiment {
            additional_hash = is_user_in_experiment as u64;
        } else if let Some(parameter_name) = &self.data.parameter_name {
            additional_hash = parameter_name.hash;
        }

        ExposureSamplingKey {
            spec_name_hash,
            rule_id_hash,
            user_values_hash,
            additional_hash,
        }
    }

    fn get_rule_id_ref(&'a self) -> &'a str {
        self.data.rule_id.as_ref().map_or("", |id| id.as_str())
    }

    fn get_extra_exposure_info_ref(&'a self) -> Option<&'a ExtraExposureInfo> {
        self.data.exposure_info.as_ref()
    }
}

pub struct QueuedExposureEvent {
    pub user: StatsigUserLoggable,
    pub sampling_decision: EvtSamplingDecision,
    pub data: ExposureData,
}

impl QueuedExposureEvent {
    pub fn into_statsig_event_internal(self) -> StatsigEventInternal {
        let mut data = self.data;
        let mut builder = MetadataBuilder::new();

        let mut should_use_undelegated_secondary_exposures = false;

        builder
            .try_add_gate_fields(&mut data)
            .try_add_config_fields(&mut data)
            .try_add_layer_fields(&mut data, &mut should_use_undelegated_secondary_exposures)
            .add_eval_details(data.evaluation_details)
            .add_interned_str("ruleID", data.rule_id.as_ref())
            .try_add("configVersion", data.version.as_ref());

        if data.trigger == ExposureTrigger::Manual {
            builder.add_bool("isManualExposure", true);
        }

        if let Some(name) = data.override_spec_name {
            builder.add_interned_str("overrideConfigName", Some(&name));
        }

        let statsig_metadata = get_statsig_metadata_with_sampling_decision(self.sampling_decision);

        let event = StatsigEvent {
            event_name: data.event_name.into(),
            value: None,
            metadata: Some(builder.build()),
            statsig_metadata: Some(statsig_metadata),
        };

        let secondary_exposures = if should_use_undelegated_secondary_exposures {
            data.undelegated_secondary_exposures
        } else {
            data.secondary_exposures
        };

        StatsigEventInternal::new(data.exposure_time, self.user, event, secondary_exposures)
    }
}

struct MetadataBuilder {
    metadata: HashMap<String, String>,
}

impl MetadataBuilder {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
        }
    }

    pub fn add_eval_details(&mut self, evaluation_details: EvaluationDetails) -> &mut Self {
        self.add_string("reason", evaluation_details.reason);
        self.try_add("lcut", evaluation_details.lcut.as_ref());
        self.try_add("receivedAt", evaluation_details.received_at.as_ref());
        self
    }

    // This is called on an async thread, so it's somewhat ok to use unperformant_to_string
    pub fn add_interned_str(&mut self, key: &str, value: Option<&InternedString>) -> &mut Self {
        self.metadata.insert(
            key.into(),
            value
                .map(|v| v.unperformant_to_string())
                .unwrap_or_default(),
        );
        self
    }

    pub fn add_string(&mut self, key: &str, value: String) -> &mut Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn add_bool(&mut self, key: &str, value: bool) -> &mut Self {
        self.metadata.insert(key.into(), value.to_string());
        self
    }

    pub fn try_add<T>(&mut self, key: &str, value: Option<&T>) -> &mut Self
    where
        T: Display,
    {
        if let Some(value) = value {
            self.metadata.insert(key.into(), value.to_string());
        }
        self
    }

    pub fn try_add_gate_fields(&mut self, data: &mut ExposureData) -> &mut Self {
        if data.event_name != GATE_EXPOSURE_EVENT_NAME {
            return self;
        }

        self.add_interned_str("gate", Some(&data.spec_name));
        self.add_bool("gateValue", data.gate_value == Some(true));
        self
    }

    pub fn try_add_config_fields(&mut self, data: &mut ExposureData) -> &mut Self {
        if data.event_name != CONFIG_EXPOSURE_EVENT_NAME {
            return self;
        }

        self.add_interned_str("config", Some(&data.spec_name));
        self.try_add("rulePassed", data.rule_passed.as_ref());
        self
    }

    pub fn try_add_layer_fields(
        &mut self,
        data: &mut ExposureData,
        should_use_undelegated_secondary_exposures: &mut bool,
    ) -> &mut Self {
        if data.event_name != LAYER_EXPOSURE_EVENT_NAME {
            return self;
        }

        let param_name = data.parameter_name.take().unwrap_or_default();
        let is_explicit = data
            .explicit_params
            .take()
            .is_some_and(|params| params.contains(&param_name));

        self.add_interned_str("config", Some(&data.spec_name));
        self.add_interned_str("parameterName", Some(&param_name));
        self.add_bool("isExplicitParameter", is_explicit);

        if is_explicit {
            self.add_interned_str("allocatedExperiment", data.allocated_experiment.as_ref());
        } else {
            self.add_interned_str("allocatedExperiment", None);
            *should_use_undelegated_secondary_exposures = true;
        }

        self
    }

    pub fn build(self) -> HashMap<String, String> {
        self.metadata
    }
}
