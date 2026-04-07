use std::collections::HashMap;

use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::evaluation::dynamic_returnable::DynamicReturnable;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::interned_string::InternedString;
use crate::specs_response::explicit_params::ExplicitParameters;
use crate::user::StatsigUserLoggable;
use crate::{log_e, SecondaryExposure};

const TAG: &str = "SpecTypesRaw";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureGateRaw<'a> {
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

    pub fn unperformant_to_json_string(&self) -> String {
        match serde_json::to_string(self) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to convert FeatureGateRaw to string: {}", e);
                format!(r#"{{"name": "{}", "value": false}}"#, self.name)
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicConfigRaw<'a> {
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

    pub fn unperformant_to_json_string(&self) -> String {
        match serde_json::to_string(self) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to convert DynamicConfigRaw to string: {}", e);
                format!(r#"{{"name": "{}", "value": null}}"#, self.name)
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRaw<'a> {
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

    pub fn unperformant_to_json_string(&self) -> String {
        match serde_json::to_string(self) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to convert ExperimentRaw to string: {}", e);
                format!(r#"{{"name": "{}"}}"#, self.name)
            }
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
pub struct LayerRaw<'a> {
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

    pub fn unperformant_to_json_string(&self) -> String {
        match serde_json::to_string(self) {
            Ok(s) => s,
            Err(e) => {
                log_e!(TAG, "Failed to convert LayerRaw to string: {}", e);
                format!(r#"{{"name": "{}"}}"#, self.name)
            }
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(feature = "ffi-support")]
pub struct PartialLayerRaw {
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

impl SuffixedRuleId<'_> {
    pub fn try_as_unprefixed_str(&self) -> Option<&str> {
        if self.rule_id_suffix.is_some() {
            // cannot return &str if we need to concat the suffix, use unperformant_to_string instead
            return None;
        }

        Some(self.rule_id.as_str())
    }

    pub fn unperformant_to_string(&self) -> String {
        if let Some(suffix) = self.rule_id_suffix {
            return format!("{}:{}", self.rule_id.as_str(), suffix);
        }

        self.rule_id.as_str().to_string()
    }
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
