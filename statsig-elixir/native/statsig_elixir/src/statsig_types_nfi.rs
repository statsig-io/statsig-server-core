use rustler::{Decoder, Encoder, Env, NifResult, NifStruct, Term};
use serde_json::Value;
use statsig_rust::statsig_types::{
    DynamicConfig as DynamicConfigActual, Experiment as ExperimentActual,
    FeatureGate as FeatureGateActual,
};
use statsig_rust::DynamicValue;
#[derive(NifStruct)]
#[module = "Statsig.Experiment"]
pub struct Experiment {
    pub name: String,
    pub rule_id: String,
    pub id_type: String,
    pub group_name: Option<String>,
    pub value: String, // serialized value
}

impl From<ExperimentActual> for Experiment {
    fn from(experiment: ExperimentActual) -> Self {
        Experiment {
            name: experiment.name,
            value: serde_json::to_string(&experiment.value).unwrap_or("".to_string()),
            rule_id: experiment.rule_id,
            id_type: experiment.id_type,
            group_name: experiment.group_name,
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.DynamicConfig"]
pub struct DynamicConfig {
    pub name: String,
    pub value: String,
    pub rule_id: String,
    pub id_type: String,
}

impl From<DynamicConfigActual> for DynamicConfig {
    fn from(config: DynamicConfigActual) -> Self {
        DynamicConfig {
            name: config.name,
            value: serde_json::to_string(&config.value).unwrap_or("".to_string()),
            rule_id: config.rule_id,
            id_type: config.id_type,
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.FeatureGate"]
pub struct FeatureGate {
    pub name: String,
    pub value: bool,
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

#[derive(Debug, Clone)]
pub enum AllowedPrimitive {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

#[allow(clippy::needless_lifetimes)]
impl<'a> Decoder<'a> for AllowedPrimitive {
    fn decode(term: Term) -> NifResult<Self> {
        if let Ok(i) = i64::decode(term) {
            Ok(AllowedPrimitive::Int(i))
        } else if let Ok(s) = String::decode(term) {
            Ok(AllowedPrimitive::Str(s))
        } else if let Ok(b) = bool::decode(term) {
            Ok(AllowedPrimitive::Bool(b))
        } else {
            Err(rustler::Error::BadArg)
        }
    }
}

impl Encoder for AllowedPrimitive {
    fn encode<'b>(&self, env: Env<'b>) -> Term<'b> {
        match self {
            AllowedPrimitive::Int(i) => i.encode(env),
            AllowedPrimitive::Str(s) => s.encode(env),
            AllowedPrimitive::Bool(b) => b.encode(env),
            AllowedPrimitive::Float(f) => f.encode(env),
        }
    }
}

impl From<AllowedPrimitive> for DynamicValue {
    fn from(value: AllowedPrimitive) -> Self {
        match value {
            AllowedPrimitive::Int(i) => DynamicValue::from(i),
            AllowedPrimitive::Str(s) => {
                let possible_json: Value =
                    serde_json::from_str(&s).unwrap_or(Value::String(s.clone()));
                match possible_json {
                    Value::Array(a) => DynamicValue::from(a),
                    _ => DynamicValue::from(s),
                }
            }
            AllowedPrimitive::Bool(b) => DynamicValue::from(b),
            AllowedPrimitive::Float(f) => DynamicValue::from(f),
        }
    }
}
