use super::{AmplitudeError, AmplitudeEvent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AmplitudeStatus {
    Unknown,
    Skipped,
    Success,
    Ratelimit,
    PayloadTooLarge,
    Invalid,
    Failed,
    Timeout,
}

impl AmplitudeStatus {
    pub fn from_code(code: u16) -> Self {
        match code {
            200..=299 => Self::Success,
            429 => Self::Ratelimit,
            413 => Self::PayloadTooLarge,
            408 => Self::Timeout,
            400..=499 => Self::Invalid,
            500.. => Self::Failed,
            _ => Self::Unknown,
        }
    }
    pub fn should_retry(self) -> bool {
        matches!(
            self,
            Self::Invalid | Self::PayloadTooLarge | Self::Ratelimit | Self::Timeout | Self::Failed
        )
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmplitudeResponse {
    pub code: u16,
    pub status: AmplitudeStatus,
    pub error: String,
    pub body: Value,
}

impl AmplitudeResponse {
    pub fn from_http_status(code: u16, error: impl Into<String>) -> Self {
        Self {
            code,
            status: AmplitudeStatus::from_code(code),
            error: error.into(),
            body: json!({}),
        }
    }

    pub fn parse(http_code: u16, body: &str) -> Result<Self, AmplitudeError> {
        let fallback = || {
            Self::from_http_status(
                http_code,
                format!("HTTP {http_code}: malformed or incomplete response body."),
            )
        };
        if body.trim().is_empty() {
            return Ok(Self::from_http_status(
                http_code,
                format!("HTTP {http_code}: empty response body."),
            ));
        }
        let Ok(body) = serde_json::from_str::<Value>(body) else {
            return Ok(fallback());
        };
        let Some(code) = body
            .get("code")
            .and_then(Value::as_u64)
            .and_then(|v| u16::try_from(v).ok())
        else {
            return Ok(fallback());
        };
        let status = AmplitudeStatus::from_code(code);
        let error = body
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if status == AmplitudeStatus::Invalid && error.starts_with("Invalid API key: ") {
            return Err(AmplitudeError::InvalidApiKey);
        }
        if status == AmplitudeStatus::Success
            && [
                "events_ingested",
                "payload_size_bytes",
                "server_upload_time",
            ]
            .iter()
            .any(|key| body.get(key).and_then(Value::as_i64).is_none())
        {
            return Ok(fallback());
        }
        if status == AmplitudeStatus::Ratelimit
            && body.get("eps_threshold").and_then(Value::as_i64).is_none()
        {
            return Ok(fallback());
        }
        Ok(Self {
            code,
            status,
            error,
            body,
        })
    }

    pub fn invalid_event_indices(&self) -> BTreeSet<usize> {
        let mut indices = BTreeSet::new();
        for key in ["events_with_invalid_fields", "events_with_missing_fields"] {
            if let Some(fields) = self.body.get(key).and_then(Value::as_object) {
                for values in fields.values().filter_map(Value::as_array) {
                    indices.extend(
                        values
                            .iter()
                            .filter_map(Value::as_u64)
                            .filter_map(|v| usize::try_from(v).ok()),
                    );
                }
            }
        }
        indices
    }

    pub fn exceeds_quota(&self, event: &AmplitudeEvent) -> bool {
        [
            ("exceeded_daily_quota_users", &event.user_id),
            ("exceeded_daily_quota_devices", &event.device_id),
        ]
        .iter()
        .any(|(key, id)| {
            id.as_ref().filter(|id| !id.is_empty()).is_some_and(|id| {
                self.body
                    .get(key)
                    .and_then(Value::as_object)
                    .is_some_and(|ids| ids.contains_key(id))
            })
        })
    }
}
