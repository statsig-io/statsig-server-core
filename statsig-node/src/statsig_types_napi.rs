use std::collections::HashMap;

use napi_derive::napi;
use serde_json::Value;
use sigstat::statsig_types::{
    DynamicConfig as DynamicConfigActual, Experiment as ExperimentActual,
    FeatureGate as FeatureGateActual, Layer as LayerActual,
};

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
    #[napi]
    pub fn get(&self, param_name: String, fallback: Value) -> Value {
        self.inner.get(&param_name, fallback)
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
    #[napi]
    pub fn get(&self, param_name: String, fallback: Value) -> Value {
        self.inner.get(&param_name, fallback)
    }
}

#[napi]
pub struct Layer {
    pub name: String,
    #[napi(js_name = "ruleID")]
    pub rule_id: String,
    pub group_name: Option<String>,

    inner: LayerActual,
}

#[napi]
impl Layer {
    #[napi]
    pub fn get(&self, param_name: String, fallback: Value) -> Value {
        self.inner.get(&param_name, fallback)
    }
}

impl From<LayerActual> for Layer {
    fn from(layer: LayerActual) -> Self {
        Layer {
            name: layer.name.clone(),
            rule_id: layer.rule_id.clone(),
            group_name: layer.group_name.clone(),
            inner: layer,
        }
    }
}
