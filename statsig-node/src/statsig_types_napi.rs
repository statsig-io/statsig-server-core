use std::collections::HashMap;

use napi_derive::napi;
use serde_json::Value;
use statsig_rust::{
    statsig_types::{
        DynamicConfig as DynamicConfigActual, Experiment as ExperimentActual,
        FeatureGate as FeatureGateActual, Layer as LayerActual,
    },
    EvaluationDetails as EvaluationDetailsActual, SecondaryExposure as SecondaryExposureActual,
};

#[napi(object)]
#[derive(Clone)]
pub struct EvaluationDetails {
    pub reason: String,
    pub lcut: Option<i64>,
    pub received_at: Option<i64>,
}

impl From<EvaluationDetailsActual> for EvaluationDetails {
    fn from(value: EvaluationDetailsActual) -> Self {
        EvaluationDetails {
            reason: value.reason,
            lcut: value.lcut.map(|lcut| lcut as i64),
            received_at: value.received_at.map(|t| t as i64),
        }
    }
}

#[napi(object)]
pub struct SecondaryExposure {
    pub gate: String,
    pub gate_value: String,
    pub rule_id: String,
}

impl From<SecondaryExposureActual> for SecondaryExposure {
    fn from(value: SecondaryExposureActual) -> Self {
        SecondaryExposure {
            gate: value.gate,
            gate_value: value.gate_value,
            rule_id: value.rule_id,
        }
    }
}

#[napi(object)]
pub struct FeatureGate {
    pub name: String,
    pub value: bool,
    #[napi(js_name = "ruleID")]
    pub rule_id: String,
    pub id_type: String,
}

impl From<FeatureGateActual> for FeatureGate {
    fn from(feature_gate: FeatureGateActual) -> Self {
        FeatureGate {
            name: feature_gate.name,
            value: feature_gate.value,
            rule_id: feature_gate.rule_id,
            id_type: feature_gate.id_type,
        }
    }
}

#[napi]
pub struct DynamicConfig {
    pub name: String,
    pub value: HashMap<String, Value>,
    #[napi(js_name = "ruleID")]
    pub rule_id: String,
    pub id_type: String,

    inner: DynamicConfigActual,
}

impl From<DynamicConfigActual> for DynamicConfig {
    fn from(config: DynamicConfigActual) -> Self {
        DynamicConfig {
            name: config.name.clone(),
            rule_id: config.rule_id.clone(),
            id_type: config.id_type.clone(),
            value: config.value.clone(),
            inner: config,
        }
    }
}

#[napi]
impl DynamicConfig {
    #[napi(
        ts_args_type = "param_name: string, fallback: boolean | number | string | object | Array<any> | null"
    )]
    pub fn get_value(&self, param_name: String, fallback: Value) -> Value {
        self.inner.get(&param_name, fallback)
    }

    #[napi]
    pub fn get_rule_id(&self) -> String {
        self.inner.rule_id.clone()
    }

    #[napi]
    pub fn get_id_type(&self) -> String {
        self.inner.id_type.clone()
    }

    #[napi]
    pub fn get_evaluation_details(&self) -> EvaluationDetails {
        self.inner.details.clone().into()
    }

    #[napi]
    pub fn get_secondary_exposures(&self) -> Option<Vec<SecondaryExposure>> {
        let maybe_eval = &self.inner.__evaluation;
        match maybe_eval {
            Some(eval) => {
                let res: Vec<SecondaryExposure> = eval
                    .base
                    .secondary_exposures
                    .clone()
                    .into_iter()
                    .map(|exp| exp.into())
                    .collect();
                Some(res)
            }
            None => None,
        }
    }
}

#[napi]
pub struct Experiment {
    pub name: String,
    pub value: HashMap<String, Value>,
    #[napi(js_name = "ruleID")]
    pub rule_id: String,
    pub id_type: String,
    pub group_name: Option<String>,

    inner: ExperimentActual,
}

impl From<ExperimentActual> for Experiment {
    fn from(experiment: ExperimentActual) -> Self {
        Experiment {
            name: experiment.name.clone(),
            rule_id: experiment.rule_id.clone(),
            id_type: experiment.id_type.clone(),
            group_name: experiment.group_name.clone(),
            value: experiment.value.clone(),
            inner: experiment,
        }
    }
}

#[napi]
impl Experiment {
    #[napi(
        ts_args_type = "param_name: string, fallback: boolean | number | string | object | Array<any> | null"
    )]
    pub fn get_value(&self, param_name: String, fallback: Value) -> Value {
        self.inner.get(&param_name, fallback)
    }

    #[napi]
    pub fn get_rule_id(&self) -> String {
        self.inner.rule_id.clone()
    }

    #[napi]
    pub fn get_id_type(&self) -> String {
        self.inner.id_type.clone()
    }

    #[napi]
    pub fn get_group_name(&self) -> Option<String> {
        self.inner.group_name.clone()
    }

    #[napi]
    pub fn get_evaluation_details(&self) -> EvaluationDetails {
        self.inner.details.clone().into()
    }

    #[napi]
    pub fn get_secondary_exposures(&self) -> Option<Vec<SecondaryExposure>> {
        let maybe_eval = &self.inner.__evaluation;
        match maybe_eval {
            Some(eval) => {
                let res: Vec<SecondaryExposure> = eval
                    .base
                    .secondary_exposures
                    .clone()
                    .into_iter()
                    .map(|exp| exp.into())
                    .collect();
                Some(res)
            }
            None => None,
        }
    }
}

#[napi]
pub struct Layer {
    pub name: String,
    #[napi(js_name = "ruleID")]
    pub rule_id: String,
    pub group_name: Option<String>,
    pub allocated_experiment_name: Option<String>,

    pub(crate) inner: LayerActual,
}

#[napi]
impl Layer {
    #[napi(
        ts_args_type = "param_name: string, fallback: boolean | number | string | object | Array<any> | null"
    )]
    pub fn get_value(&self, param_name: String, fallback: Value) -> Value {
        self.inner.get(&param_name, fallback)
    }

    #[napi]
    pub fn get_rule_id(&self) -> String {
        self.inner.rule_id.clone()
    }

    #[napi]
    pub fn get_group_name(&self) -> Option<String> {
        self.inner.group_name.clone()
    }

    #[napi]
    pub fn get_evaluation_details(&self) -> EvaluationDetails {
        self.inner.details.clone().into()
    }

    #[napi]
    pub fn get_secondary_exposures(&self) -> Option<Vec<SecondaryExposure>> {
        let maybe_eval = &self.inner.__evaluation;
        match maybe_eval {
            Some(eval) => {
                let res: Vec<SecondaryExposure> = eval
                    .base
                    .secondary_exposures
                    .clone()
                    .into_iter()
                    .map(|exp| exp.into())
                    .collect();
                Some(res)
            }
            None => None,
        }
    }
}

impl From<LayerActual> for Layer {
    fn from(layer: LayerActual) -> Self {
        Layer {
            name: layer.name.clone(),
            rule_id: layer.rule_id.clone(),
            group_name: layer.group_name.clone(),
            allocated_experiment_name: layer.allocated_experiment_name.clone(),
            inner: layer,
        }
    }
}
