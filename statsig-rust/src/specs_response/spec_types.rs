use std::collections::HashMap;

use ahash::HashMap as AHashMap;
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::skip_serializing_none;

use crate::evaluation::dynamic_string::DynamicString;
use crate::evaluation::{dynamic_returnable::DynamicReturnable, evaluator_value::EvaluatorValue};
use crate::interned_string::InternedString;
use crate::DynamicValue;

use super::{cmab_types::CMABConfig, param_store_types::ParameterStore};

// DO_NOT_CLONE: Please do not add the Clone trait to this struct. We intentionally
// avoid cloning this data at all costs as it can be quite large.

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug)] /* DO_NOT_CLONE */
#[serde(rename_all = "camelCase")]
pub struct Spec {
    #[serde(rename = "type")]
    pub _type: InternedString,
    pub salt: InternedString,
    pub default_value: DynamicReturnable,
    pub enabled: bool,
    pub rules: Vec<Rule>,
    pub id_type: InternedString,
    pub explicit_parameters: Option<Vec<InternedString>>,
    pub entity: InternedString,
    pub has_shared_params: Option<bool>,
    pub is_active: Option<bool>,
    pub version: Option<u32>,
    #[serde(rename = "targetAppIDs")]
    pub target_app_ids: Option<Vec<InternedString>>,
    pub forward_all_exposures: Option<bool>,
    pub fields_used: Option<Vec<InternedString>>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug)] /* DO_NOT_CLONE */
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub name: InternedString,
    pub pass_percentage: f64,
    pub return_value: DynamicReturnable,
    pub id: InternedString,
    pub salt: Option<InternedString>,
    pub conditions: Vec<InternedString>,
    pub id_type: DynamicString,
    pub group_name: Option<InternedString>,
    pub config_delegate: Option<InternedString>,
    pub is_experiment_group: Option<bool>,
    pub sampling_rate: Option<u64>,
}

#[derive(Serialize, PartialEq, Debug)] /* DO_NOT_CLONE */
#[serde(rename_all = "camelCase")]
pub struct Condition {
    #[serde(rename = "type")]
    pub condition_type: InternedString,
    pub target_value: Option<EvaluatorValue>,
    pub operator: Option<InternedString>,
    pub field: Option<DynamicString>,
    pub additional_values: Option<HashMap<InternedString, InternedString>>,
    pub id_type: DynamicString,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug)] /* DO_NOT_CLONE */
pub struct OverrideRule {
    pub rule_name: String,
    pub start_time: Option<i64>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug)] /* DO_NOT_CLONE */
pub struct ConfigMapping {
    pub new_config_name: String,
    pub rules: Vec<OverrideRule>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug)] /* DO_NOT_CLONE */
pub struct SessionReplayTrigger {
    pub sampling_rate: Option<f64>,
    pub values: Option<Vec<String>>,
    pub passes_sampling: Option<bool>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug)] /* DO_NOT_CLONE */
pub struct SessionReplayInfo {
    pub sampling_rate: Option<f64>,
    pub targeting_gate: Option<String>,
    pub recording_blocked: Option<bool>,
    pub session_recording_event_triggers: Option<HashMap<String, SessionReplayTrigger>>,
    pub session_recording_exposure_triggers: Option<HashMap<String, SessionReplayTrigger>>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug, Default)] /* DO_NOT_CLONE */
pub struct SpecsResponseFull {
    pub company_id: Option<String>,
    pub feature_gates: AHashMap<InternedString, Spec>,
    pub dynamic_configs: AHashMap<InternedString, Spec>,
    pub layer_configs: AHashMap<InternedString, Spec>,
    pub condition_map: AHashMap<InternedString, Condition>,
    pub experiment_to_layer: HashMap<String, String>,
    pub has_updates: bool,
    pub time: u64,
    pub checksum: Option<String>,
    pub default_environment: Option<String>,
    pub app_id: Option<DynamicValue>,
    pub sdk_keys_to_app_ids: Option<HashMap<String, DynamicValue>>,
    pub hashed_sdk_keys_to_app_ids: Option<HashMap<String, DynamicValue>>,
    pub diagnostics: Option<HashMap<String, f64>>,
    pub param_stores: Option<HashMap<InternedString, ParameterStore>>,
    pub sdk_configs: Option<HashMap<String, DynamicValue>>,
    pub sdk_flags: Option<HashMap<String, bool>>,
    pub cmab_configs: Option<HashMap<String, CMABConfig>>,
    pub overrides: Option<HashMap<String, Vec<ConfigMapping>>>,
    pub override_rules: Option<HashMap<String, Rule>>,
    pub id_lists: Option<HashMap<String, bool>>,
    pub response_format: Option<String>,
    pub session_replay_info: Option<SessionReplayInfo>,
}

#[skip_serializing_none]
#[derive(Deserialize)]
pub struct SpecsResponseNoUpdates {
    pub has_updates: bool,
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ConditionInternal {
            #[serde(rename = "type")]
            condition_type: InternedString,
            target_value: Option<EvaluatorValue>,
            operator: Option<InternedString>,
            field: Option<DynamicString>,
            additional_values: Option<HashMap<InternedString, InternedString>>,
            id_type: DynamicString,
        }

        let mut internal = ConditionInternal::deserialize(deserializer)?;

        if let Some(ref op) = internal.operator {
            if op == "str_matches" {
                if let Some(ref mut tv) = internal.target_value {
                    tv.compile_regex();
                }
            }
        }

        Ok(Condition {
            condition_type: internal.condition_type,
            target_value: internal.target_value,
            operator: internal.operator,
            field: internal.field,
            additional_values: internal.additional_values,
            id_type: internal.id_type,
        })
    }
}
