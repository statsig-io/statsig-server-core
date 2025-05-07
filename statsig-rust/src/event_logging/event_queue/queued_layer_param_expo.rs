use crate::{
    evaluation::evaluation_types::ExtraExposureInfo,
    event_logging::{
        event_logger::ExposureTrigger,
        exposer_sampling::EvtSamplingDecision,
        exposure_utils::{get_metadata_with_details, get_statsig_metadata_with_sampling_decision},
        statsig_event::StatsigEvent,
        statsig_event_internal::{StatsigEventInternal, LAYER_EXPOSURE_EVENT_NAME},
    },
    statsig_types::Layer,
    user::StatsigUserLoggable,
    EvaluationDetails, SecondaryExposure,
};

use super::queued_event::{EnqueueOperation, QueuedEvent, QueuedExposure};

pub enum EnqueueLayerParamExpoOp<'a> {
    LayerRef(&'a Layer, &'a str, ExposureTrigger),
    LayerOwned(Layer, String, ExposureTrigger),
}

impl<'a> EnqueueLayerParamExpoOp<'a> {
    fn get_layer_ref(&'a self) -> &'a Layer {
        match self {
            EnqueueLayerParamExpoOp::LayerRef(layer, _, _) => layer,
            EnqueueLayerParamExpoOp::LayerOwned(layer, _, _) => layer,
        }
    }

    fn get_parameter_name_ref(&'a self) -> &'a str {
        match self {
            EnqueueLayerParamExpoOp::LayerRef(_, parameter_name, _) => parameter_name,
            EnqueueLayerParamExpoOp::LayerOwned(_, parameter_name, _) => parameter_name.as_str(),
        }
    }
}

impl EnqueueOperation for EnqueueLayerParamExpoOp<'_> {
    fn as_exposure(&self) -> Option<&impl QueuedExposure> {
        Some(self)
    }

    fn into_queued_event(self, sampling_decision: EvtSamplingDecision) -> QueuedEvent {
        let event = match self {
            EnqueueLayerParamExpoOp::LayerRef(layer, parameter_name, trigger) => {
                extract_from_layer_ref(layer, parameter_name, trigger, sampling_decision)
            }
            EnqueueLayerParamExpoOp::LayerOwned(layer, parameter_name, trigger) => {
                extract_from_layer_owned(layer, parameter_name, trigger, sampling_decision)
            }
        };

        QueuedEvent::LayerParamExposure(event)
    }
}

impl<'a> QueuedExposure<'a> for EnqueueLayerParamExpoOp<'a> {
    fn create_exposure_sampling_key(&self) -> String {
        let layer = self.get_layer_ref();

        let experiment_name = layer.allocated_experiment_name.as_deref();

        let mut sampling_key = String::from("n:");
        sampling_key.push_str(&layer.name);
        sampling_key.push_str(";e:");
        sampling_key.push_str(experiment_name.unwrap_or_default());
        sampling_key.push_str(";p:");
        sampling_key.push_str(self.get_parameter_name_ref());
        sampling_key.push_str(";u:");
        sampling_key.push_str(&layer.__user.create_sampling_key());
        sampling_key.push_str(";r:");
        sampling_key.push_str(&layer.rule_id);
        sampling_key
    }

    fn create_spec_sampling_key(&self) -> String {
        let mut sampling_key = String::from("n:");
        sampling_key.push_str(&self.get_layer_ref().name);
        sampling_key.push_str(";r:");
        sampling_key.push_str(&self.get_layer_ref().rule_id);
        sampling_key
    }

    fn get_rule_id_ref(&'a self) -> &'a str {
        &self.get_layer_ref().rule_id
    }

    fn get_extra_exposure_info_ref(&'a self) -> Option<&'a ExtraExposureInfo> {
        self.get_layer_ref()
            .__evaluation
            .as_ref()?
            .base
            .exposure_info
            .as_ref()
    }
}

pub struct QueuedLayerParamExposureEvent {
    pub user: StatsigUserLoggable,
    pub layer_name: String,
    pub rule_id: String,
    pub parameter_name: String,
    pub secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub evaluation_details: EvaluationDetails,
    pub version: Option<u32>,
    pub exposure_trigger: ExposureTrigger,
    pub sampling_decision: EvtSamplingDecision,
    pub override_config_name: Option<String>,
    pub is_explicit: bool,
    pub allocated_experiment: Option<String>,
}

impl QueuedLayerParamExposureEvent {
    pub fn into_statsig_event_internal(self) -> StatsigEventInternal {
        let mut metadata = get_metadata_with_details(self.evaluation_details);
        metadata.insert("config".into(), self.layer_name);
        metadata.insert("ruleID".into(), self.rule_id);
        metadata.insert(
            "allocatedExperiment".into(),
            self.allocated_experiment.unwrap_or_default(),
        );
        metadata.insert("parameterName".into(), self.parameter_name);
        metadata.insert("isExplicitParameter".into(), self.is_explicit.to_string());

        if let Some(version) = self.version {
            metadata.insert("configVersion".into(), version.to_string());
        }

        if self.exposure_trigger == ExposureTrigger::Manual {
            metadata.insert("isManualExposure".into(), "true".into());
        }

        if let Some(override_config_name) = self.override_config_name {
            metadata.insert("overrideConfigName".into(), override_config_name);
        }

        let statsig_metadata = get_statsig_metadata_with_sampling_decision(self.sampling_decision);

        let event = StatsigEvent {
            event_name: LAYER_EXPOSURE_EVENT_NAME.into(),
            value: None,
            metadata: Some(metadata),
            statsig_metadata: Some(statsig_metadata),
        };

        StatsigEventInternal::new(
            self.user,
            event,
            Some(self.secondary_exposures.unwrap_or_default()),
        )
    }
}

type ExtractFromEvaluationResult = (
    bool,
    Option<String>,
    Option<Vec<SecondaryExposure>>,
    Option<u32>,
    Option<String>,
);

fn extract_exposure_info(layer: &Layer, parameter_name: &String) -> ExtractFromEvaluationResult {
    let evaluation = match layer.__evaluation.as_ref() {
        Some(eval) => eval,
        None => return (false, None, None, None, None),
    };

    let is_explicit = evaluation.explicit_parameters.contains(parameter_name);
    let secondary_exposures;
    let mut allocated_experiment = None;

    if is_explicit {
        allocated_experiment = evaluation.allocated_experiment_name.clone();
        secondary_exposures = Some(evaluation.base.secondary_exposures.clone());
    } else {
        secondary_exposures = evaluation.undelegated_secondary_exposures.clone();
    }

    // version might be on the top level or the exposure info
    let mut version = layer.__version;
    let mut override_config_name = None;

    if let Some(exposure_info) = evaluation.base.exposure_info.as_ref() {
        version = exposure_info.version;
        override_config_name = exposure_info.override_config_name.clone();
    }

    (
        is_explicit,
        allocated_experiment,
        secondary_exposures,
        version,
        override_config_name,
    )
}

fn extract_from_layer_ref(
    layer: &Layer,
    param_name: &str,
    trigger: ExposureTrigger,
    sampling_decision: EvtSamplingDecision,
) -> QueuedLayerParamExposureEvent {
    let parameter_name = param_name.to_string();
    let (is_explicit, allocated_experiment, secondary_exposures, version, override_config_name) =
        extract_exposure_info(layer, &parameter_name);

    QueuedLayerParamExposureEvent {
        user: layer.__user.clone(),
        layer_name: layer.name.clone(),
        rule_id: layer.rule_id.clone(),
        parameter_name,
        exposure_trigger: trigger,
        evaluation_details: layer.details.clone(),
        version,
        sampling_decision,
        override_config_name,
        secondary_exposures,
        is_explicit,
        allocated_experiment,
    }
}

fn extract_from_layer_owned(
    layer: Layer,
    parameter_name: String,
    trigger: ExposureTrigger,
    sampling_decision: EvtSamplingDecision,
) -> QueuedLayerParamExposureEvent {
    let (is_explicit, allocated_experiment, secondary_exposures, version, override_config_name) =
        extract_exposure_info(&layer, &parameter_name);

    QueuedLayerParamExposureEvent {
        user: layer.__user,
        layer_name: layer.name,
        rule_id: layer.rule_id,
        parameter_name,
        exposure_trigger: trigger,
        evaluation_details: layer.details,
        version,
        sampling_decision,
        override_config_name,
        secondary_exposures,
        is_explicit,
        allocated_experiment,
    }
}
