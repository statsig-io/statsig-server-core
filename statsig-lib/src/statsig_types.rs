use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{
    DynamicConfigEvaluation, ExperimentEvaluation, GateEvaluation, LayerEvaluation,
};
use crate::statsig_user_internal::StatsigUserInternal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}
