#![doc = include_str!("../README.md")]

use chrono::Utc;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{fmt, time::Duration};
use uuid::Uuid;

/// HTTP settings for an Amplitude client.
#[derive(Clone, Debug)]
pub struct AmplitudeOptions {
    pub server_url: String,
    pub timeout_ms: u64,
}

impl Default for AmplitudeOptions {
    fn default() -> Self {
        Self {
            server_url: "https://api2.amplitude.com/2/httpapi".into(),
            timeout_ms: 10_000,
        }
    }
}

/// An event with a user ID, device ID, or both.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AmplitudeEvent {
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_properties: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_properties: Option<Map<String, Value>>,
    /// Event timestamp in milliseconds since the Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<i64>,
}

impl AmplitudeEvent {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            ..Self::default()
        }
    }
}

#[derive(Debug)]
pub enum AmplitudeError {
    InvalidInput(&'static str),
    Http(reqwest::Error),
    Api { status: u16 },
}

impl fmt::Display for AmplitudeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => f.write_str(message),
            Self::Http(error) => write!(f, "Amplitude HTTP client error: {error}"),
            Self::Api { status } => write!(f, "Amplitude returned HTTP {status}"),
        }
    }
}

impl std::error::Error for AmplitudeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Serialize)]
struct TrackRequest<'a> {
    api_key: &'a str,
    events: [TrackedEvent; 1],
}

#[derive(Serialize)]
struct TrackedEvent {
    #[serde(flatten)]
    event: AmplitudeEvent,
    library: &'static str,
}

/// A reusable server-side Amplitude event client.
#[derive(Clone)]
pub struct Amplitude {
    api_key: String,
    server_url: Url,
    http: Client,
}

impl Amplitude {
    pub fn new(api_key: &str, options: Option<AmplitudeOptions>) -> Result<Self, AmplitudeError> {
        if api_key.trim().is_empty() {
            return Err(AmplitudeError::InvalidInput("api_key must not be empty"));
        }
        let options = options.unwrap_or_default();
        if options.timeout_ms == 0 {
            return Err(AmplitudeError::InvalidInput("timeout_ms must be positive"));
        }
        let server_url = Url::parse(&options.server_url)
            .ok()
            .filter(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
            .ok_or(AmplitudeError::InvalidInput(
                "server_url must be a valid HTTP(S) URL",
            ))?;
        let http = Client::builder()
            .timeout(Duration::from_millis(options.timeout_ms))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| AmplitudeError::Http(error.without_url()))?;

        Ok(Self {
            api_key: api_key.into(),
            server_url,
            http,
        })
    }

    /// Sends one event immediately. Success means Amplitude returned HTTP 200.
    pub async fn track(&self, mut event: AmplitudeEvent) -> Result<(), AmplitudeError> {
        let has_identity = [event.user_id.as_deref(), event.device_id.as_deref()]
            .into_iter()
            .flatten()
            .any(|id| !id.trim().is_empty());
        if event.event_type.trim().is_empty() || !has_identity {
            return Err(AmplitudeError::InvalidInput(
                "event_type and either user_id or device_id are required",
            ));
        }

        event
            .time
            .get_or_insert_with(|| Utc::now().timestamp_millis());
        event
            .insert_id
            .get_or_insert_with(|| Uuid::new_v4().to_string());

        let request = TrackRequest {
            api_key: &self.api_key,
            events: [TrackedEvent {
                event,
                library: concat!("amplitude-rust-core/", env!("CARGO_PKG_VERSION")),
            }],
        };
        let response = self
            .http
            .post(self.server_url.clone())
            .json(&request)
            .send()
            .await
            .map_err(|error| AmplitudeError::Http(error.without_url()))?;

        match response.status() {
            StatusCode::OK => Ok(()),
            status => Err(AmplitudeError::Api {
                status: status.as_u16(),
            }),
        }
    }
}
