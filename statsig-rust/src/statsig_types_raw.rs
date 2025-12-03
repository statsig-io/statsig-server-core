use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::interned_string::InternedString;
use crate::specs_response::explicit_params::ExplicitParameters;
use crate::user::StatsigUserLoggable;
use crate::SecondaryExposure;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeatureGateRaw<'a> {
    pub name: &'a str,

    pub value: bool,

    #[serde(rename = "ruleID")]
    pub rule_id: SuffixedRuleId<'a>,

    pub id_type: Option<&'a InternedString>,

    pub details: &'a EvaluationDetails,
}

impl<'a> FeatureGateRaw<'a> {
    pub fn empty(name: &'a str, details: &'a EvaluationDetails) -> Self {
        Self {
            name,
            value: false,
            details,
            rule_id: SuffixedRuleId {
                rule_id: InternedString::empty_ref(),
                rule_id_suffix: None,
            },
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
    pub rule_id: SuffixedRuleId<'a>,

    pub id_type: Option<&'a InternedString>,

    pub details: &'a EvaluationDetails,
}

impl<'a> DynamicConfigRaw<'a> {
    pub fn empty(name: &'a str, details: &'a EvaluationDetails) -> Self {
        Self {
            name,
            value: None,
            details,
            rule_id: SuffixedRuleId {
                rule_id: InternedString::empty_ref(),
                rule_id_suffix: None,
            },
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
    pub rule_id: SuffixedRuleId<'a>,

    pub id_type: Option<&'a InternedString>,

    pub group_name: Option<&'a InternedString>,

    pub is_experiment_active: Option<bool>,

    pub details: &'a EvaluationDetails,

    pub secondary_exposures: Option<&'a Vec<SecondaryExposure>>,
}

impl<'a> ExperimentRaw<'a> {
    pub fn empty(name: &'a str, details: &'a EvaluationDetails) -> Self {
        Self {
            name,
            value: None,
            details,
            rule_id: SuffixedRuleId {
                rule_id: InternedString::empty_ref(),
                rule_id_suffix: None,
            },
            id_type: None,
            group_name: None,
            is_experiment_active: None,
            secondary_exposures: None,
        }
    }
}

fn is_interned_string_none_or_empty(value: &Option<&InternedString>) -> bool {
    match value {
        Some(value) => value.is_empty(),
        None => true,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub(crate) struct LayerRaw<'a> {
    pub name: &'a str,

    pub value: Option<&'a DynamicReturnable>,

    #[serde(rename = "ruleID")]
    pub rule_id: SuffixedRuleId<'a>,

    pub id_type: Option<&'a InternedString>,

    pub group_name: Option<&'a InternedString>,

    pub is_experiment_active: Option<bool>,

    pub details: &'a EvaluationDetails,

    #[serde(skip_serializing_if = "is_interned_string_none_or_empty")]
    pub allocated_experiment_name: Option<&'a InternedString>,

    pub disable_exposure: bool,

    pub user: StatsigUserLoggable,

    pub secondary_exposures: Option<&'a Vec<SecondaryExposure>>,

    pub undelegated_secondary_exposures: Option<&'a Vec<SecondaryExposure>>,

    pub explicit_parameters: Option<ExplicitParameters>,

    pub parameter_rule_ids: Option<&'a HashMap<InternedString, InternedString>>,
}

impl<'a> LayerRaw<'a> {
    pub fn empty(name: &'a str, details: &'a EvaluationDetails) -> Self {
        Self {
            name,
            details,
            rule_id: SuffixedRuleId {
                rule_id: InternedString::empty_ref(),
                rule_id_suffix: None,
            },
            id_type: None,
            group_name: None,
            is_experiment_active: None,
            value: None,
            allocated_experiment_name: None,
            disable_exposure: false,
            user: StatsigUserLoggable::null(),
            secondary_exposures: None,
            undelegated_secondary_exposures: None,
            explicit_parameters: None,
            parameter_rule_ids: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PartialLayerRaw {
    pub name: InternedString,

    #[serde(rename = "ruleID")]
    pub rule_id: Option<InternedString>,

    pub id_type: Option<InternedString>,

    pub group_name: Option<InternedString>,

    pub details: EvaluationDetails,

    pub allocated_experiment_name: Option<InternedString>,
    pub disable_exposure: bool,
    pub user: StatsigUserLoggable,
    pub secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub undelegated_secondary_exposures: Option<Vec<SecondaryExposure>>,
    pub explicit_parameters: Option<ExplicitParameters>,
    pub parameter_rule_ids: Option<HashMap<InternedString, InternedString>>,
}

pub struct SuffixedRuleId<'a> {
    pub rule_id: &'a InternedString,
    pub rule_id_suffix: Option<&'a str>,
}

impl<'a> std::fmt::Display for SuffixedRuleId<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.rule_id.as_str())?;
        if let Some(suffix) = self.rule_id_suffix {
            f.write_str(":")?;
            f.write_str(suffix)?;
        }
        Ok(())
    }
}

impl<'a> Serialize for SuffixedRuleId<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}
