use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::evaluation::evaluation_details::EvaluationDetails;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum KeyType {
    #[serde(rename = "initialize")]
    Initialize,
    #[serde(rename = "overall")]
    Overall,
    #[serde(rename = "download_config_specs")]
    DownloadConfigSpecs,
    #[serde(rename = "get_id_list")]
    GetIDList,
    #[serde(rename = "get_id_list_sources")]
    GetIDListSources,
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
    pub key: KeyType,

    action: ActionType,

    timestamp: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    attempt: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    config_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    id_list_count: Option<u32>,

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

    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    evaluation_details: Option<EvaluationDetails>,

    #[serde(skip_serializing_if = "Option::is_none")]
    config_spec_ready: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
}

impl Marker {
    #[must_use]
    pub fn new(key: KeyType, action: ActionType, step: Option<StepType>) -> Self {
        Self {
            key,
            action,
            step,
            success: None,
            timestamp: Utc::now().timestamp_millis() as u64,
            status_code: None,
            url: None,
            id_list_count: None,
            sdk_region: None,
            marker_id: None,
            attempt: None,
            config_name: None,
            message: None,
            error: None,
            evaluation_details: None,
            config_spec_ready: None,
            source: None,
        }
    }

    #[must_use]
    pub fn with_is_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }

    #[must_use]
    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = Some(status_code);
        self
    }

    #[must_use]
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = Some(attempt);
        self
    }

    #[must_use]
    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    #[must_use]
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    #[must_use]
    pub fn with_eval_details(mut self, details: EvaluationDetails) -> Self {
        self.evaluation_details = Some(details);
        self
    }

    #[must_use]
    pub fn with_config_spec_ready(mut self, ready: bool) -> Self {
        self.config_spec_ready = Some(ready);
        self
    }

    #[must_use]
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    #[must_use]
    pub fn with_sdk_region(mut self, region: Option<String>) -> Self {
        self.sdk_region = region;
        self
    }

    #[must_use]
    pub fn with_id_list_count(mut self, count: usize) -> Self {
        if count <= u32::MAX as usize {
            self.id_list_count = Some(count as u32);
        }
        self
    }

    #[must_use]
    pub fn with_error(mut self, error: HashMap<String, String>) -> Self {
        self.error = Some(error);
        self
    }

    // TODO add more as needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_new() {
        let timestamp: u64 = Utc::now().timestamp_millis() as u64;
        let marker = Marker::new(
            KeyType::Initialize,
            ActionType::Start,
            Some(StepType::Process),
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
        let timestamp: u64 = Utc::now().timestamp_millis() as u64;
        let marker = Marker::new(
            KeyType::Initialize,
            ActionType::Start,
            Some(StepType::NetworkRequest),
        )
        .with_is_success(true)
        .with_status_code(200)
        .with_attempt(1);

        let serialized = serde_json::to_string(&marker).expect("Failed to serialize Marker");
        let expected_json = format!(
            r#"{{"key":"initialize","action":"start","timestamp":{timestamp},"attempt":1,"statusCode":200,"step":"network_request","success":true}}"#
        );
        assert_eq!(serialized, expected_json);
    }
}
