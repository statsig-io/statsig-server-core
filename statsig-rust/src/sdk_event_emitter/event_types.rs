use serde::Serialize;

use crate::{specs_response::spec_types::SpecsResponseFull, SpecsSource};

const TAG: &str = "SdkEvent";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SdkEventCode {
    Unknown = 0,
    All = 1,
    GateEvaluated = 2,
    DynamicConfigEvaluated = 3,
    ExperimentEvaluated = 4,
    LayerEvaluated = 5,
    SpecsUpdated = 6,
}

impl SdkEventCode {
    pub fn from_name(name: &str) -> Self {
        match name {
            SdkEvent::ALL => SdkEventCode::All,
            SdkEvent::GATE_EVALUATED => SdkEventCode::GateEvaluated,
            SdkEvent::DYNAMIC_CONFIG_EVALUATED => SdkEventCode::DynamicConfigEvaluated,
            SdkEvent::EXPERIMENT_EVALUATED => SdkEventCode::ExperimentEvaluated,
            SdkEvent::LAYER_EVALUATED => SdkEventCode::LayerEvaluated,
            SdkEvent::SPECS_UPDATED => SdkEventCode::SpecsUpdated,
            _ => SdkEventCode::Unknown,
        }
    }

    pub fn as_raw(&self) -> u8 {
        *self as u8
    }
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum SdkEvent<'a> {
    SpecsUpdated {
        source: &'a SpecsSource,
        source_api: &'a Option<String>,
        values: &'a SpecsResponseFull,
    },
    GateEvaluated {
        gate_name: &'a str,
        rule_id: &'a str,
        value: bool,
        reason: &'a str,
    },
    DynamicConfigEvaluated {
        config_name: &'a str,
        reason: &'a str,
        rule_id: Option<&'a str>,
        value: Option<&'a crate::DynamicReturnable>,
    },
    ExperimentEvaluated {
        experiment_name: &'a str,
        reason: &'a str,
        rule_id: Option<&'a str>,
        value: Option<&'a crate::DynamicReturnable>,
        group_name: Option<&'a str>,
    },
    LayerEvaluated {
        layer_name: &'a str,
        reason: &'a str,
        rule_id: Option<&'a str>,
        // not giving value as it wouldn't trigger an exposure
    },
}

impl SdkEvent<'_> {
    pub const ALL: &'static str = "*";
    pub const GATE_EVALUATED: &'static str = "gate_evaluated";
    pub const DYNAMIC_CONFIG_EVALUATED: &'static str = "dynamic_config_evaluated";
    pub const EXPERIMENT_EVALUATED: &'static str = "experiment_evaluated";
    pub const LAYER_EVALUATED: &'static str = "layer_evaluated";
    pub const SPECS_UPDATED: &'static str = "specs_updated";

    pub fn get_code(&self) -> SdkEventCode {
        match self {
            SdkEvent::GateEvaluated { .. } => SdkEventCode::GateEvaluated,
            SdkEvent::DynamicConfigEvaluated { .. } => SdkEventCode::DynamicConfigEvaluated,
            SdkEvent::ExperimentEvaluated { .. } => SdkEventCode::ExperimentEvaluated,
            SdkEvent::LayerEvaluated { .. } => SdkEventCode::LayerEvaluated,
            SdkEvent::SpecsUpdated { .. } => SdkEventCode::SpecsUpdated,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self.get_code() {
            SdkEventCode::All => SdkEvent::ALL,
            SdkEventCode::GateEvaluated => SdkEvent::GATE_EVALUATED,
            SdkEventCode::DynamicConfigEvaluated => SdkEvent::DYNAMIC_CONFIG_EVALUATED,
            SdkEventCode::ExperimentEvaluated => SdkEvent::EXPERIMENT_EVALUATED,
            SdkEventCode::LayerEvaluated => SdkEvent::LAYER_EVALUATED,
            SdkEventCode::SpecsUpdated => SdkEvent::SPECS_UPDATED,
            SdkEventCode::Unknown => "unknown",
        }
    }

    pub fn to_raw_json_string(&self) -> Option<String> {
        let raw = SdkEventRaw {
            event_name: self.get_name(),
            data: self,
        };

        match serde_json::to_string(&raw) {
            Ok(json) => Some(json),
            Err(e) => {
                crate::log_e!(TAG, "Failed to convert SdkEvent to raw JSON string: {}", e);
                None
            }
        }
    }
}

#[derive(Serialize, Clone)]
pub struct SdkEventRaw<'a> {
    event_name: &'static str,
    data: &'a SdkEvent<'a>,
}
