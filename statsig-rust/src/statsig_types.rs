use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{
    DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
};
use crate::event_logging::event_logger::{EventLogger, ExposureTrigger};
use crate::event_logging::event_queue::queued_layer_param_expo::EnqueueLayerParamExpoOp;
use crate::specs_response::param_store_types::Parameter;
use crate::statsig_core_api_options::ParameterStoreEvaluationOptions;
use crate::user::StatsigUserLoggable;
use crate::Statsig;
use crate::StatsigUser;

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
    pub __override_config_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DynamicConfig {
    pub name: String,
    pub value: HashMap<String, Value>,
    pub rule_id: String,
    pub id_type: String,
    pub details: EvaluationDetails,

    pub __evaluation: Option<DynamicConfigEvaluation>,
    pub __version: Option<u32>,
    pub __override_config_name: Option<String>,
}

impl DynamicConfig {
    #[must_use]
    pub fn get_opt<T: DeserializeOwned>(&self, param_name: &str) -> Option<T> {
        match self.value.get(param_name) {
            Some(value) => from_value(value.clone()).ok(),
            None => None,
        }
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

    pub __evaluation: Option<ExperimentEvaluation>,
    pub __version: Option<u32>,
    pub __override_config_name: Option<String>,
}

impl Experiment {
    #[must_use]
    pub fn get_opt<T: DeserializeOwned>(&self, param_name: &str) -> Option<T> {
        match self.value.get(param_name) {
            Some(value) => from_value(value.clone()).ok(),
            None => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Layer {
    pub name: String,
    pub rule_id: String,
    pub id_type: String,

    pub group_name: Option<String>,
    pub details: EvaluationDetails,
    pub allocated_experiment_name: Option<String>,

    pub __evaluation: Option<LayerEvaluation>,
    pub __value: HashMap<String, Value>,
    pub __user: StatsigUserLoggable,
    pub __version: Option<u32>,
    pub __disable_exposure: bool,
    pub __override_config_name: Option<String>,

    #[serde(skip_serializing, skip_deserializing)]
    pub __event_logger_ptr: Option<Weak<EventLogger>>,
}

impl Layer {
    pub fn get_opt<T: DeserializeOwned>(&self, param_name: &str) -> Option<T> {
        let value = match self.__value.get(param_name) {
            Some(value) => value.clone(),
            None => return None,
        };

        match from_value(value.clone()) {
            Ok(value) => {
                self.log_param_exposure(param_name);
                Some(value)
            }
            Err(_) => None,
        }
    }

    pub fn get_raw_value(&self, param_name: &str) -> Option<Value> {
        match self.__value.get(param_name) {
            Some(value) => {
                self.log_param_exposure(param_name);
                Some(value.clone())
            }
            None => None,
        }
    }

    fn log_param_exposure(&self, param_name: &str) -> Option<()> {
        let logger = self.__event_logger_ptr.as_ref()?.upgrade()?;

        if self.__disable_exposure {
            logger.increment_non_exposure_checks(&self.name);
            return None;
        }

        logger.enqueue(EnqueueLayerParamExpoOp::LayerRef(
            self,
            param_name,
            ExposureTrigger::Auto,
        ));

        None
    }
}

macro_rules! impl_common_get_methods {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn get<T: DeserializeOwned>(&self, param_name: &str, fallback: T) -> T {
                self.get_opt(param_name).unwrap_or_else(|| fallback)
            }

            #[must_use]
            pub fn get_bool(&self, param_name: &str, fallback: bool) -> bool {
                self.get(param_name, fallback)
            }

            #[must_use]
            pub fn get_f64(&self, param_name: &str, fallback: f64) -> f64 {
                self.get(param_name, fallback)
            }

            #[must_use]
            pub fn get_i64(&self, param_name: &str, fallback: i64) -> i64 {
                self.get(param_name, fallback)
            }

            #[must_use]
            pub fn get_string(&self, param_name: &str, fallback: String) -> String {
                self.get(param_name, fallback)
            }

            #[must_use]
            pub fn get_array(&self, param_name: &str, fallback: Vec<Value>) -> Vec<Value> {
                self.get(param_name, fallback)
            }

            #[must_use]
            pub fn get_object(
                &self,
                param_name: &str,
                fallback: HashMap<String, Value>,
            ) -> HashMap<String, Value> {
                self.get(param_name, fallback)
            }
        }
    };
}

#[derive(Serialize, Clone)]
pub struct ParameterStore<'a> {
    pub name: String,
    pub details: EvaluationDetails,
    pub parameters: HashMap<String, Parameter>,
    pub options: ParameterStoreEvaluationOptions,

    #[serde(skip_serializing, skip_deserializing)]
    pub _statsig_ref: &'a Statsig,
}

impl ParameterStore<'_> {
    pub fn get_opt<T: DeserializeOwned>(&self, user: &StatsigUser, param_name: &str) -> Option<T> {
        let param = self.parameters.get(param_name)?;
        match param {
            Parameter::StaticValue(static_value) => from_value(static_value.value.clone()).ok(),
            Parameter::Gate(gate) => {
                let res = self._statsig_ref.check_gate_with_options(
                    user,
                    &gate.gate_name,
                    self.options.into(),
                );
                let val = match res {
                    true => gate.pass_value.clone(),
                    false => gate.fail_value.clone(),
                };
                from_value(val).ok()
            }
            Parameter::DynamicConfig(dynamic_config) => {
                let res = self._statsig_ref.get_dynamic_config_with_options(
                    user,
                    &dynamic_config.config_name,
                    self.options.into(),
                );
                res.get_opt(&dynamic_config.param_name)?
            }
            Parameter::Experiment(experiment) => {
                let res = self._statsig_ref.get_experiment_with_options(
                    user,
                    &experiment.experiment_name,
                    self.options.into(),
                );
                res.get_opt(&experiment.param_name)?
            }
            Parameter::Layer(layer) => {
                let res = self._statsig_ref.get_layer_with_options(
                    user,
                    &layer.layer_name,
                    self.options.into(),
                );
                res.get_opt(&layer.param_name)?
            }
        }
    }

    pub fn get<T: DeserializeOwned>(&self, user: &StatsigUser, param_name: &str, fallback: T) -> T {
        self.get_opt(user, param_name).unwrap_or(fallback)
    }

    pub fn get_json_value(
        &self,
        user: &StatsigUser,
        param_name: &str,
        fallback: Option<Value>,
    ) -> Value {
        match fallback {
            None | Some(Value::Null) => self
                .get_opt::<Value>(user, param_name)
                .unwrap_or(Value::Null),
            Some(Value::Bool(boolean)) => self.get_bool(user, param_name, boolean).into(),
            Some(Value::Number(number)) => self.get(user, param_name, number).into(),
            Some(Value::String(string)) => self.get_string(user, param_name, string).into(),
            Some(Value::Array(vec)) => self.get_array(user, param_name, vec).into(),
            Some(Value::Object(map)) => self
                .get_object(user, param_name, map.into_iter().collect())
                .into_iter()
                .collect(),
        }
    }

    pub fn get_bool(&self, user: &StatsigUser, param_name: &str, fallback: bool) -> bool {
        self.get(user, param_name, fallback)
    }

    pub fn get_f64(&self, user: &StatsigUser, param_name: &str, fallback: f64) -> f64 {
        self.get(user, param_name, fallback)
    }

    pub fn get_i64(&self, user: &StatsigUser, param_name: &str, fallback: i64) -> i64 {
        self.get(user, param_name, fallback)
    }

    pub fn get_string(&self, user: &StatsigUser, param_name: &str, fallback: String) -> String {
        self.get(user, param_name, fallback)
    }

    pub fn get_array(
        &self,
        user: &StatsigUser,
        param_name: &str,
        fallback: Vec<Value>,
    ) -> Vec<Value> {
        self.get(user, param_name, fallback)
    }

    pub fn get_object(
        &self,
        user: &StatsigUser,
        param_name: &str,
        fallback: HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        self.get(user, param_name, fallback)
    }
}

impl_common_get_methods!(DynamicConfig);
impl_common_get_methods!(Experiment);
impl_common_get_methods!(Layer);

pub enum OverrideAdapterType {
    LocalOverride,
}
