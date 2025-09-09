use crate::statsig_types::{DynamicConfig, Experiment, Layer};
use serde::Serialize;
use serde_json::json;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SdkEventCode {
    Unknown = 0,
    All = 1,
    GateEvaluated = 2,
    DynamicConfigEvaluated = 3,
    ExperimentEvaluated = 4,
    LayerEvaluated = 5,
}

impl SdkEventCode {
    pub fn from_name(name: &str) -> Self {
        match name {
            SdkEvent::ALL => SdkEventCode::All,
            SdkEvent::GATE_EVALUATED => SdkEventCode::GateEvaluated,
            SdkEvent::DYNAMIC_CONFIG_EVALUATED => SdkEventCode::DynamicConfigEvaluated,
            SdkEvent::EXPERIMENT_EVALUATED => SdkEventCode::ExperimentEvaluated,
            SdkEvent::LAYER_EVALUATED => SdkEventCode::LayerEvaluated,
            _ => SdkEventCode::Unknown,
        }
    }

    pub fn as_raw(&self) -> u8 {
        *self as u8
    }
}

#[derive(Serialize, Clone)]
pub enum SdkEvent<'a> {
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

impl SdkEvent<'_> {
    pub const ALL: &'static str = "*";
    pub const GATE_EVALUATED: &'static str = "gate_evaluated";
    pub const DYNAMIC_CONFIG_EVALUATED: &'static str = "dynamic_config_evaluated";
    pub const EXPERIMENT_EVALUATED: &'static str = "experiment_evaluated";
    pub const LAYER_EVALUATED: &'static str = "layer_evaluated";

    pub fn get_code(&self) -> SdkEventCode {
        match self {
            SdkEvent::GateEvaluated { .. } => SdkEventCode::GateEvaluated,
            SdkEvent::DynamicConfigEvaluated { .. } => SdkEventCode::DynamicConfigEvaluated,
            SdkEvent::ExperimentEvaluated { .. } => SdkEventCode::ExperimentEvaluated,
            SdkEvent::LayerEvaluated { .. } => SdkEventCode::LayerEvaluated,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self.get_code() {
            SdkEventCode::All => SdkEvent::ALL,
            SdkEventCode::GateEvaluated => SdkEvent::GATE_EVALUATED,
            SdkEventCode::DynamicConfigEvaluated => SdkEvent::DYNAMIC_CONFIG_EVALUATED,
            SdkEventCode::ExperimentEvaluated => SdkEvent::EXPERIMENT_EVALUATED,
            SdkEventCode::LayerEvaluated => SdkEvent::LAYER_EVALUATED,
            SdkEventCode::Unknown => "unknown",
        }
    }

    pub fn to_json_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let name = self.get_name();
        let mut map = serde_json::Map::new();
        map.insert("event_name".to_string(), json!(name));

        match self {
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
