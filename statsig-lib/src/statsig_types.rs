use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{
    DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
};
use crate::statsig_user_internal::StatsigUserInternal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Weak;
use crate::event_logging::event_logger::{EventLogger, QueuedEventPayload};
use crate::event_logging::layer_exposure::LayerExposure;

#[derive(Serialize, Deserialize, Clone)]
pub struct FeatureGate {
    pub name: String,
    pub value: bool,
    pub rule_id: String,
    pub id_type: String,
    pub details: EvaluationDetails,

    pub(crate) __evaluation: Option<GateEvaluation>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DynamicConfig {
    pub name: String,
    pub value: HashMap<String, DynamicValue>,
    pub rule_id: String,
    pub id_type: String,
    pub details: EvaluationDetails,

    pub(crate) __evaluation: Option<DynamicConfigEvaluation>,
}

impl DynamicConfig {
    pub fn get_f64(&self, param_name: &str) -> Option<f64> {
        self.value.get(param_name)?.float_value.clone()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Experiment {
    pub name: String,
    pub value: HashMap<String, DynamicValue>,
    pub rule_id: String,
    pub id_type: String,
    pub group_name: Option<String>,
    pub details: EvaluationDetails,

    pub(crate) __evaluation: Option<ExperimentEvaluation>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Layer {
    pub name: String,
    pub rule_id: String,
    pub id_type: String,
    pub group_name: Option<String>,
    pub details: EvaluationDetails,

    pub __evaluation: Option<LayerEvaluation>,
    pub __value: HashMap<String, DynamicValue>,
    pub __user: StatsigUserInternal,

    #[serde(skip_serializing, skip_deserializing)]
    pub __event_logger_ptr: Option<Weak<EventLogger>>,
}

impl Layer {
    pub fn get_string(&self, param_name: &str) -> Option<String> {
        let value = self.__value.get(param_name)?.string_value.clone();

        self.log_param_exposure(param_name);

        value
    }

    pub fn get_f64(&self, param_name: &str) -> Option<f64> {
        let value = self.__value.get(param_name)?.float_value.clone();

        self.log_param_exposure(param_name);

        value
    }

    fn log_param_exposure(&self, param_name: &str) -> Option<()> {
        if let Some(ptr) = &self.__event_logger_ptr {
            ptr.upgrade()?.enqueue(QueuedEventPayload::LayerExposure(LayerExposure {
                user: self.__user.clone(),
                layer_name: self.name.clone(),
                parameter_name: param_name.to_string(),
                evaluation: self.__evaluation.clone(),
                evaluation_details: self.details.clone(),
            }))
        }

        None
    }
}
