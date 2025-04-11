use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{
    AnyEvaluation, DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
};
use crate::event_logging::event_logger::{EventLogger, QueuedEventPayload};
use crate::event_logging::layer_exposure::LayerExposure;
use crate::sampling_processor::SamplingDecision;
use crate::spec_types::Parameter;
use crate::statsig_core_api_options::ParameterStoreEvaluationOptions;
use crate::statsig_user_internal::StatsigUserLoggable;
use crate::StatsigUser;
use crate::{SamplingProcessor, Statsig};
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
    #[serde(skip_serializing, skip_deserializing)]
    pub __sampling_processor: Option<Weak<SamplingProcessor>>,
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
        if self.__disable_exposure {
            if let Some(ptr) = &self.__event_logger_ptr {
                ptr.upgrade()?
                    .increment_non_exposure_checks_count(self.name.clone());
            }
            return None;
        }

        let mut sampling_details = SamplingDecision::default();

        if let Some(ptr) = &self.__sampling_processor {
            let layer_eval = self.__evaluation.as_ref();

            sampling_details = ptr.upgrade()?.get_sampling_decision_and_details(
                &self.__user.get_sampling_key(),
                layer_eval.map(AnyEvaluation::from).as_ref(),
                Some(param_name),
            );

            if !sampling_details.should_send_exposure {
                return None;
            }
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
                    sampling_details,
                    override_config_name: self.__override_config_name.clone(),
                }));
        }

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
