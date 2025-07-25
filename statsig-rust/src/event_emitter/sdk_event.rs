use crate::{
    statsig_types::{DynamicConfig, Experiment, Layer},
    StatsigErr,
};
use serde::Serialize;
use serde_json::json;
use std::borrow::Cow;

#[derive(Serialize, Clone)]
pub enum SdkEvent<'a> {
    InitSuccess {
        duration: f64,
    },
    InitFailure {
        reason: String,
        duration: f64,
        error: Option<StatsigErr>,
    },
    RulesetsUpdated {
        lcut: u64,
        raw_value: String,
    },
    GateEvaluated {
        gate_name: Cow<'a, str>,
        rule_id: Cow<'a, str>,
        value: bool,
        reason: Cow<'a, str>,
    },
    DynamicConfigEvaluated {
        dynamic_config: Cow<'a, DynamicConfig>,
    },
    ExperimentEvaluated {
        experiment: Cow<'a, Experiment>,
    },
    LayerEvaluated {
        layer: Cow<'a, Layer>,
    },
}

impl<'a> SdkEvent<'a> {
    pub const ALL: &'static str = "*";
    pub const INIT_SUCCESS: &'static str = "init_success";
    pub const INIT_FAILURE: &'static str = "init_failure";
    pub const RULESETS_UPDATED: &'static str = "rulesets_updated";
    pub const GATE_EVALUATED: &'static str = "gate_evaluated";
    pub const DYNAMIC_CONFIG_EVALUATED: &'static str = "dynamic_config_evaluated";
    pub const EXPERIMENT_EVALUATED: &'static str = "experiment_evaluated";
    pub const LAYER_EVALUATED: &'static str = "layer_evaluated";

    pub fn get_code_from_name(name: &str) -> usize {
        match name {
            SdkEvent::ALL => 1,
            SdkEvent::INIT_SUCCESS => 2,
            SdkEvent::INIT_FAILURE => 3,
            SdkEvent::RULESETS_UPDATED => 4,
            SdkEvent::GATE_EVALUATED => 5,
            SdkEvent::DYNAMIC_CONFIG_EVALUATED => 6,
            SdkEvent::EXPERIMENT_EVALUATED => 7,
            SdkEvent::LAYER_EVALUATED => 8,
            _ => 0,
        }
    }

    pub fn get_code(&self) -> usize {
        let name = self.get_name();
        SdkEvent::get_code_from_name(name)
    }

    pub fn get_name(&self) -> &str {
        match self {
            SdkEvent::InitSuccess { .. } => SdkEvent::INIT_SUCCESS,
            SdkEvent::InitFailure { .. } => SdkEvent::INIT_FAILURE,
            SdkEvent::RulesetsUpdated { .. } => SdkEvent::RULESETS_UPDATED,
            SdkEvent::GateEvaluated { .. } => SdkEvent::GATE_EVALUATED,
            SdkEvent::DynamicConfigEvaluated { .. } => SdkEvent::DYNAMIC_CONFIG_EVALUATED,
            SdkEvent::ExperimentEvaluated { .. } => SdkEvent::EXPERIMENT_EVALUATED,
            SdkEvent::LayerEvaluated { .. } => SdkEvent::LAYER_EVALUATED,
        }
    }

    pub fn to_json_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let name = self.get_name();
        let mut map = serde_json::Map::new();
        map.insert("event_name".to_string(), json!(name));

        match self {
            SdkEvent::InitSuccess { duration } => {
                map.insert("duration".to_string(), json!(duration));
            }
            SdkEvent::InitFailure {
                reason,
                duration,
                error,
            } => {
                map.insert("reason".to_string(), json!(reason));
                map.insert("duration".to_string(), json!(duration));
                map.insert("error".to_string(), json!(error));
            }
            SdkEvent::RulesetsUpdated { lcut, raw_value } => {
                map.insert("lcut".to_string(), json!(lcut));
                map.insert("raw_value".to_string(), json!(raw_value));
            }
            SdkEvent::GateEvaluated {
                gate_name,
                rule_id,
                value,
                reason,
            } => {
                map.insert("gate_name".to_string(), json!(gate_name));
                map.insert("value".to_string(), json!(value));
                map.insert("reason".to_string(), json!(reason));
                map.insert("rule_id".to_string(), json!(rule_id));
            }
            SdkEvent::DynamicConfigEvaluated { dynamic_config } => {
                map.insert("dynamic_config".to_string(), json!(dynamic_config));
            }
            SdkEvent::ExperimentEvaluated { experiment } => {
                map.insert("experiment".to_string(), json!(experiment));
            }
            SdkEvent::LayerEvaluated { layer } => {
                map.insert("layer".to_string(), json!(layer));
            }
        }

        map
    }
}
