use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{
    DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
};
use crate::event_logging::event_logger::{EventLogger, QueuedEventPayload};
use crate::event_logging::layer_exposure::LayerExposure;
use crate::statsig_user_internal::StatsigUserInternal;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, Value};
use std::collections::HashMap;
use std::sync::Weak;

#[derive(Serialize, Deserialize, Clone)]
pub struct FeatureGate {
    pub name: String,
    pub value: bool,
    pub rule_id: String,
    pub id_type: String,
    pub details: EvaluationDetails,

    pub(crate) __evaluation: Option<GateEvaluation>,
    pub __version: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DynamicConfig {
    pub name: String,
    pub value: HashMap<String, Value>,
    pub rule_id: String,
    pub id_type: String,
    pub details: EvaluationDetails,

    pub(crate) __evaluation: Option<DynamicConfigEvaluation>,
    pub __version: Option<u32>,
}

impl DynamicConfig {
    pub fn get<T: DeserializeOwned>(&self, param_name: &str, fallback: T) -> T {
        if !self.value.contains_key(param_name) {
            return fallback;
        }

        if let Ok(value) = from_value(self.value[param_name].clone()) {
            return value;
        }

        fallback
    }

    pub fn get_bool(&self, param_name: &str, fallback: bool) -> bool {
        self.get(param_name, fallback)
    }

    pub fn get_f64(&self, param_name: &str, fallback: f64) -> f64 {
        self.get(param_name, fallback)
    }

    pub fn get_i64(&self, param_name: &str, fallback: i64) -> i64 {
        self.get(param_name, fallback)
    }

    pub fn get_string(&self, param_name: &str, fallback: String) -> String {
        self.get(param_name, fallback)
    }

    pub fn get_array(&self, param_name: &str, fallback: Vec<Value>) -> Vec<Value> {
        self.get(param_name, fallback)
    }

    pub fn get_object(&self, param_name: &str, fallback: HashMap<String, Value>) -> HashMap<String, Value> {
        self.get(param_name, fallback)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Experiment {
    pub name: String,
    pub value: HashMap<String, Value>,
    pub rule_id: String,
    pub id_type: String,
    pub group_name: Option<String>,
    pub details: EvaluationDetails,

    pub(crate) __evaluation: Option<ExperimentEvaluation>,
    pub __version: Option<u32>,
}

impl Experiment {
    pub fn get<T: DeserializeOwned>(&self, param_name: &str, fallback: T) -> T {
        if !self.value.contains_key(param_name) {
            return fallback;
        }

        if let Ok(value) = from_value(self.value[param_name].clone()) {
            return value;
        }

        fallback
    }

    pub fn get_bool(&self, param_name: &str, fallback: bool) -> bool {
        self.get(param_name, fallback)
    }

    pub fn get_f64(&self, param_name: &str, fallback: f64) -> f64 {
        self.get(param_name, fallback)
    }

    pub fn get_i64(&self, param_name: &str, fallback: i64) -> i64 {
        self.get(param_name, fallback)
    }

    pub fn get_string(&self, param_name: &str, fallback: String) -> String {
        self.get(param_name, fallback)
    }

    pub fn get_array(&self, param_name: &str, fallback: Vec<Value>) -> Vec<Value> {
        self.get(param_name, fallback)
    }

    pub fn get_object(&self, param_name: &str, fallback: HashMap<String, Value>) -> HashMap<String, Value> {
        self.get(param_name, fallback)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Layer {
    pub name: String,
    pub rule_id: String,

    pub group_name: Option<String>,
    pub details: EvaluationDetails,
    pub allocated_experiment_name: Option<String>,

    pub __evaluation: Option<LayerEvaluation>,
    pub __value: HashMap<String, Value>,
    pub __user: StatsigUserInternal,
    pub __version: Option<u32>,
    pub __disable_exposure: bool,

    #[serde(skip_serializing, skip_deserializing)]
    pub __event_logger_ptr: Option<Weak<EventLogger>>,
}

impl Layer {
    pub fn get<T: DeserializeOwned>(&self, param_name: &str, fallback: T) -> T {
        if !self.__value.contains_key(param_name) {
            return fallback;
        }

        if let Ok(value) = from_value(self.__value[param_name].clone()) {
            self.log_param_exposure(param_name);
            return value;
        }

        fallback
    }

    pub fn get_bool(&self, param_name: &str, fallback: bool) -> bool {
        self.get(param_name, fallback)
    }

    pub fn get_f64(&self, param_name: &str, fallback: f64) -> f64 {
        self.get(param_name, fallback)
    }

    pub fn get_i64(&self, param_name: &str, fallback: i64) -> i64 {
        self.get(param_name, fallback)
    }

    pub fn get_string(&self, param_name: &str, fallback: String) -> String {
        self.get(param_name, fallback)
    }

    pub fn get_array(&self, param_name: &str, fallback: Vec<Value>) -> Vec<Value> {
        self.get(param_name, fallback)
    }

    pub fn get_object(&self, param_name: &str, fallback: HashMap<String, Value>) -> HashMap<String, Value> {
        self.get(param_name, fallback)
    }

    fn log_param_exposure(&self, param_name: &str) -> Option<()> {
        if self.__disable_exposure {
            if let Some(ptr) = &self.__event_logger_ptr {
                ptr.upgrade()?
                    .increment_non_exposure_checks_count(self.name.clone());
            }
            return None;
        }
        if let Some(ptr) = &self.__event_logger_ptr {
            ptr.upgrade()?
                .enqueue(QueuedEventPayload::LayerExposure(LayerExposure {
                    user: self.__user.clone(),
                    layer_name: self.name.clone(),
                    parameter_name: param_name.to_string(),
                    evaluation: self.__evaluation.clone(),
                    evaluation_details: self.details.clone(),
                    version: self.__version,
                    is_manual_exposure: false,
                }))
        }

        None
    }
}
