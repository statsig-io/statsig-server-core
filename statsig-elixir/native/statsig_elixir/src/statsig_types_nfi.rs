use rustler::{Decoder, Encoder, Env, NifResult, NifStruct, Term};
use serde_json::Value;
use statsig_rust::statsig_core_api_options::{
    DynamicConfigEvaluationOptions as DynamicConfigEvaluationOptionsActual,
    ExperimentEvaluationOptions as ExperimentEvaluationOptionsActual,
    FeatureGateEvaluationOptions as FeatureGateEvaluationOptionsActual,
    LayerEvaluationOptions as LayerEvaluationOptionsActual,
};
use statsig_rust::statsig_types::{
    DynamicConfig as DynamicConfigActual, Experiment as ExperimentActual,
    FeatureGate as FeatureGateActual,
};
use statsig_rust::DynamicValue;
use statsig_rust::{ClientInitResponseOptions as ClientInitResponseOptionsActual, HashAlgorithm};
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
        } else if let Ok(f) = f64::decode(term) {
            Ok(AllowedPrimitive::Float(f))
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

#[derive(NifStruct)]
#[module = "Statsig.ExperimentEvaluationOptions"]
pub struct ExperimentEvaluationOptions {
    pub disable_exposure_logging: bool,
}

impl From<ExperimentEvaluationOptions> for ExperimentEvaluationOptionsActual {
    fn from(option: ExperimentEvaluationOptions) -> Self {
        ExperimentEvaluationOptionsActual {
            disable_exposure_logging: option.disable_exposure_logging,
            ..Default::default()
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.FeatureGateEvaluationOptions"]
pub struct FeatureGateEvaluationOptions {
    pub disable_exposure_logging: bool,
}

impl From<FeatureGateEvaluationOptions> for FeatureGateEvaluationOptionsActual {
    fn from(option: FeatureGateEvaluationOptions) -> Self {
        FeatureGateEvaluationOptionsActual {
            disable_exposure_logging: option.disable_exposure_logging,
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.LayerEvaluationOptions"]
pub struct LayerEvaluationOptions {
    pub disable_exposure_logging: bool,
}

impl From<LayerEvaluationOptions> for LayerEvaluationOptionsActual {
    fn from(option: LayerEvaluationOptions) -> Self {
        LayerEvaluationOptionsActual {
            disable_exposure_logging: option.disable_exposure_logging,
            ..Default::default()
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.DynamicConfigEvaluationOptions"]
pub struct DynamicConfigEvaluationOptions {
    pub disable_exposure_logging: bool,
}

impl From<DynamicConfigEvaluationOptions> for DynamicConfigEvaluationOptionsActual {
    fn from(option: DynamicConfigEvaluationOptions) -> Self {
        DynamicConfigEvaluationOptionsActual {
            disable_exposure_logging: option.disable_exposure_logging,
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.ClientInitResponseOptions"]
pub struct ClientInitResponseOptions {
    pub hash_algorithm: Option<String>,
    pub client_sdk_key: Option<String>,
    pub include_local_overrides: Option<bool>,
}

impl From<ClientInitResponseOptions> for ClientInitResponseOptionsActual {
    fn from(option: ClientInitResponseOptions) -> Self {
        ClientInitResponseOptionsActual {
            hash_algorithm: option
                .hash_algorithm
                .and_then(|v| HashAlgorithm::from_string(v.as_str())),
            client_sdk_key: option.client_sdk_key,
            include_local_overrides: option.include_local_overrides,
            ..Default::default()
        }
    }
}
