use serde::Serialize;

use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::interned_string::InternedString;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeatureGateRaw<'a> {
    pub name: &'a str,

    pub value: bool,

    #[serde(rename = "ruleID")]
    pub rule_id: Option<&'a InternedString>,

    pub id_type: Option<&'a InternedString>,

    pub details: &'a EvaluationDetails,
}

impl<'a> FeatureGateRaw<'a> {
    pub fn empty(name: &'a str, details: &'a EvaluationDetails) -> Self {
        Self {
            name,
            value: false,
            details,
            rule_id: None,
            id_type: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DynamicConfigRaw<'a> {
    pub name: &'a str,

    pub value: Option<&'a DynamicReturnable>,

    #[serde(rename = "ruleID")]
    pub rule_id: Option<&'a InternedString>,

    pub id_type: Option<&'a InternedString>,

    pub details: &'a EvaluationDetails,
}

impl<'a> DynamicConfigRaw<'a> {
    pub fn empty(name: &'a str, details: &'a EvaluationDetails) -> Self {
        Self {
            name,
            value: None,
            details,
            rule_id: None,
            id_type: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExperimentRaw<'a> {
    pub name: &'a str,

    pub value: Option<&'a DynamicReturnable>,

    #[serde(rename = "ruleID")]
    pub rule_id: Option<&'a InternedString>,

    pub id_type: Option<&'a InternedString>,

    pub group_name: Option<&'a InternedString>,

    pub is_experiment_active: Option<bool>,

    pub details: &'a EvaluationDetails,
}

impl<'a> ExperimentRaw<'a> {
    pub fn empty(name: &'a str, details: &'a EvaluationDetails) -> Self {
        Self {
            name,
            value: None,
            details,
            rule_id: None,
            id_type: None,
            group_name: None,
            is_experiment_active: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LayerRaw<'a> {
    pub name: &'a str,

    #[serde(rename = "ruleID")]
    pub rule_id: Option<&'a InternedString>,

    pub id_type: Option<&'a InternedString>,

    pub group_name: Option<&'a InternedString>,

    pub is_experiment_active: Option<bool>,

    pub allocated_experiment_name: Option<&'a InternedString>,

    pub details: &'a EvaluationDetails,
}

impl<'a> LayerRaw<'a> {
    pub fn empty(name: &'a str, details: &'a EvaluationDetails) -> Self {
        Self {
            name,
            details,
            rule_id: None,
            id_type: None,
            group_name: None,
            is_experiment_active: None,
            allocated_experiment_name: None,
        }
    }
}
