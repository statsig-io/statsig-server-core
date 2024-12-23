use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::evaluation::evaluation_details::EvaluationDetails;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum KeyType {
    #[serde(rename = "initialize")]
    Initialize,
    #[serde(rename = "overall")]
    Overall,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StepType {
    #[serde(rename = "process")]
    Process,
    #[serde(rename = "network_request")]
    NetworkRequest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "end")]
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    action: ActionType,

    #[serde(skip_serializing_if = "Option::is_none")]
    attempt: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    config_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    id_list_count: Option<u32>,

    key: KeyType,

    #[serde(rename = "markerID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    marker_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    sdk_region: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    status_code: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    step: Option<StepType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    success: Option<bool>,

    timestamp: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    evaluation_details: Option<EvaluationDetails>,
}

impl Marker {
    pub fn new(key: KeyType, action: ActionType, step: Option<StepType>, timestamp: u64) -> Self {
        Self {
            key,
            action,
            step,
            success: None,
            timestamp,
            status_code: None,
            url: None,
            id_list_count: None,
            sdk_region: None,
            marker_id: None,
            attempt: None,
            config_name: None,
            message: None,
            error: None,
            evaluation_details: None
        }
    }

    pub fn with_is_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }

    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = Some(status_code);
        self
    }

    pub fn with_attempt(mut self, attempt: u16) -> Self {
        self.attempt = Some(attempt);
        self
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    pub fn with_eval_details(mut self, details: EvaluationDetails) -> Self {
        self.evaluation_details = Some(details);
        self
    }

    // TODO add more as needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_new() {
        let timestamp: u64 = 1_640_995_200_000;
        let marker = Marker::new(
            KeyType::Initialize,
            ActionType::Start,
            Some(StepType::Process),
            timestamp,
        );

        assert_eq!(marker.key, KeyType::Initialize);
        assert_eq!(marker.action, ActionType::Start);
        assert_eq!(marker.step, Some(StepType::Process));
        assert_eq!(marker.success, None);
        assert_eq!(marker.status_code, None);
        assert_eq!(marker.timestamp, timestamp);
    }

    #[test]
    fn test_marker_serialization() {
        let timestamp: u64 = 1_640_995_200_000;
        let marker = Marker::new(
            KeyType::Initialize,
            ActionType::Start,
            Some(StepType::NetworkRequest),
            timestamp,
        )
        .with_is_success(true)
        .with_status_code(200)
        .with_attempt(1);

        let serialized = serde_json::to_string(&marker).expect("Failed to serialize Marker");
        let expected_json = r#"{"action":"start","attempt":1,"key":"initialize","statusCode":200,"step":"network_request","success":true,"timestamp":1640995200000}"#;

        assert_eq!(serialized, expected_json);
    }
}
