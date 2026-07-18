use rustler::{
    serde::{Deserializer, Serializer},
    Decoder, Encoder, Env, NifResult, NifStruct, Term,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use statsig_rust::statsig_core_api_options::{
    DynamicConfigEvaluationOptions as DynamicConfigEvaluationOptionsActual,
    ExperimentEvaluationOptions as ExperimentEvaluationOptionsActual,
    FeatureGateEvaluationOptions as FeatureGateEvaluationOptionsActual,
    LayerEvaluationOptions as LayerEvaluationOptionsActual,
};
use statsig_rust::statsig_types::{
    DynamicConfig as DynamicConfigActual, Experiment as ExperimentActual,
    ExperimentGroup as ExperimentGroupActual,
    ExperimentGroupsResult as ExperimentGroupsResultActual, FeatureGate as FeatureGateActual,
};
use statsig_rust::DynamicValue;
use statsig_rust::EvaluationDetails as EvaluationDetailsActual;
use statsig_rust::{ClientInitResponseOptions as ClientInitResponseOptionsActual, HashAlgorithm};
use std::collections::HashMap;

// Wrapper type for HashMap<String, Value> that can be encoded/decoded via Rustler serde
#[derive(Clone, Debug)]
pub struct ValueMap(pub HashMap<String, Value>);

impl Encoder for ValueMap {
    fn encode<'b>(&self, env: Env<'b>) -> Term<'b> {
        let serializer = Serializer::from(env);
        self.0.serialize(serializer).unwrap()
    }
}

impl Decoder<'_> for ValueMap {
    fn decode(term: Term) -> NifResult<Self> {
        let deserializer = Deserializer::from(term);
        let map: HashMap<String, Value> =
            HashMap::deserialize(deserializer).map_err(|_| rustler::Error::BadArg)?;
        Ok(ValueMap(map))
    }
}

#[derive(NifStruct)]
#[module = "Statsig.EvaluationDetails"]
pub struct EvaluationDetails {
    pub reason: String,
    pub lcut: Option<u64>,
    pub received_at: Option<u64>,
    pub version: Option<u32>,
}

impl From<EvaluationDetailsActual> for EvaluationDetails {
    fn from(details: EvaluationDetailsActual) -> Self {
        EvaluationDetails {
            reason: details.reason,
            lcut: details.lcut,
            received_at: details.received_at,
            version: details.version,
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.Experiment"]
pub struct Experiment {
    pub name: String,
    pub rule_id: String,
    pub id_type: String,
    pub group_name: Option<String>,
    pub value: ValueMap,
    pub details: EvaluationDetails,
}

impl From<ExperimentActual> for Experiment {
    fn from(experiment: ExperimentActual) -> Self {
        Experiment {
            name: experiment.name,
            value: ValueMap(experiment.value),
            rule_id: experiment.rule_id,
            id_type: experiment.id_type,
            group_name: experiment.group_name,
            details: experiment.details.into(),
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.ExperimentGroup"]
pub struct ExperimentGroup {
    pub group_name: String,
    pub rule_id: String,
    pub id_type: String,
    pub return_value: ValueMap,
}

impl From<ExperimentGroupActual> for ExperimentGroup {
    fn from(group: ExperimentGroupActual) -> Self {
        ExperimentGroup {
            group_name: group.group_name,
            rule_id: group.rule_id,
            id_type: group.id_type,
            return_value: ValueMap(group.return_value),
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.ExperimentGroupsResult"]
pub struct ExperimentGroupsResult {
    pub is_experiment_active: Option<bool>,
    pub groups: Vec<ExperimentGroup>,
}

impl From<ExperimentGroupsResultActual> for ExperimentGroupsResult {
    fn from(result: ExperimentGroupsResultActual) -> Self {
        ExperimentGroupsResult {
            is_experiment_active: result.is_experiment_active,
            groups: result
                .groups
                .into_iter()
                .map(ExperimentGroup::from)
                .collect(),
        }
    }
}

#[derive(NifStruct)]
#[module = "Statsig.DynamicConfig"]
pub struct DynamicConfig {
    pub name: String,
    pub value: ValueMap,
    pub rule_id: String,
    pub id_type: String,
    pub details: EvaluationDetails,
}

impl From<DynamicConfigActual> for DynamicConfig {
    fn from(config: DynamicConfigActual) -> Self {
        DynamicConfig {
            name: config.name,
            value: ValueMap(config.value),
            rule_id: config.rule_id,
            id_type: config.id_type,
            details: config.details.into(),
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
    pub details: EvaluationDetails,
}

impl From<FeatureGateActual> for FeatureGate {
    fn from(feature_gate: FeatureGateActual) -> Self {
        FeatureGate {
            name: feature_gate.name,
            value: feature_gate.value,
            rule_id: feature_gate.rule_id,
            id_type: feature_gate.id_type,
            details: feature_gate.details.into(),
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
        } else if let Some(f) = decode_overflowing_int(term) {
            Ok(AllowedPrimitive::Float(f))
        } else {
            Err(rustler::Error::BadArg)
        }
    }
}

// Elixir integers have arbitrary precision; values above i64::MAX fail every numeric
// decoder above (rustler's f64::decode also falls back through i64). Decode the term as
// a BigInt and convert via its decimal representation — lossy for magnitudes above 2^53,
// but preserves the field instead of failing the NIF call. Mirrors `try_as_overflowing_int`
// in the pyo3 bridge. Returns None for magnitudes outside f64's finite range so callers
// don't end up with infinities silently serialized as JSON null downstream.
fn decode_overflowing_int(term: Term) -> Option<f64> {
    let bi = rustler::BigInt::decode(term).ok()?;
    bi.to_string().parse::<f64>().ok().filter(|f| f.is_finite())
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
