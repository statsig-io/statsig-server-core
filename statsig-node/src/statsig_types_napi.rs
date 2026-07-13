use napi_derive::napi;
use serde::Serialize;
use serde_json::Value;
use statsig_rust::{
    statsig_types::ExperimentGroup as ExperimentGroupActual,
    statsig_types::ExperimentGroupsResult as ExperimentGroupsResultActual,
    statsig_types::ParameterStore as ParameterStoreActual,
    EvaluationDetails as EvaluationDetailsActual, SecondaryExposure as SecondaryExposureActual,
};
use std::collections::HashMap;

use crate::statsig_user_napi::StatsigUser;

#[napi(object)]
#[derive(Clone, Serialize)]
pub struct ExperimentGroup {
    pub group_name: String,
    pub rule_id: String,
    pub id_type: String,
    pub return_value: HashMap<String, Value>,
}

impl From<ExperimentGroupActual> for ExperimentGroup {
    fn from(value: ExperimentGroupActual) -> Self {
        ExperimentGroup {
            group_name: value.group_name,
            rule_id: value.rule_id,
            id_type: value.id_type,
            return_value: value.return_value,
        }
    }
}

#[napi(object)]
#[derive(Clone, Serialize)]
pub struct ExperimentGroupsResult {
    /// Omitted when the name does not refer to an experiment (unknown name or a
    /// dynamic config); otherwise the experiment's `isActive` state.
    pub is_experiment_active: Option<bool>,
    pub groups: Vec<ExperimentGroup>,
}

impl From<ExperimentGroupsResultActual> for ExperimentGroupsResult {
    fn from(value: ExperimentGroupsResultActual) -> Self {
        ExperimentGroupsResult {
            is_experiment_active: value.is_experiment_active,
            groups: value
                .groups
                .into_iter()
                .map(ExperimentGroup::from)
                .collect(),
        }
    }
}

#[napi(object)]
#[derive(Clone, Serialize)]
pub struct EvaluationDetails {
    pub reason: String,
    pub lcut: Option<i64>,
    pub received_at: Option<i64>,
    pub version: Option<u32>,
}

impl From<EvaluationDetailsActual> for EvaluationDetails {
    fn from(value: EvaluationDetailsActual) -> Self {
        EvaluationDetails {
            reason: value.reason,
            lcut: value.lcut.map(|lcut| lcut as i64),
            received_at: value.received_at.map(|t| t as i64),
            version: value.version,
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
            gate: value.gate.unperformant_to_string(),
            gate_value: value.gate_value.unperformant_to_string(),
            rule_id: value.rule_id.unperformant_to_string(),
        }
    }
}

#[napi]
pub struct ParameterStore<'a> {
    pub name: String,

    pub(crate) user: &'a StatsigUser,
    pub(crate) inner: ParameterStoreActual<'a>,
}

#[napi]
impl ParameterStore<'_> {
    #[napi(ts_type = "<T>(paramName: string, fallback?: T): T")]
    pub fn get_value(&self, param_name: String, fallback: Option<Value>) -> Value {
        self.inner
            .get_json_value(self.user.as_inner(), &param_name, fallback)
    }

    #[napi]
    pub fn get_evaluation_details(&self) -> EvaluationDetails {
        self.inner.details.clone().into()
    }

    #[napi(js_name = "toJSON")]
    pub fn to_json(&self) -> serde_json::Map<String, serde_json::Value> {
        let v = serde_json::to_value(&self.inner);
        match v {
            Ok(serde_json::Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        }
    }
}
