use crate::networking::{
    providers::get_network_provider, HttpMethod, NetworkProvider, RequestArgs,
};
use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::{collections::HashMap, fmt, sync::Arc};
use uuid::Uuid;

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

#[derive(Clone, Debug, Default, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library: Option<String>,
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
    NotInitialized,
    InvalidInput(String),
    Network(String),
    Api { status: u16, body: String },
}

impl fmt::Display for AmplitudeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "Call Amplitude.init before track"),
            Self::InvalidInput(message) | Self::Network(message) => f.write_str(message),
            Self::Api { status, body } => write!(f, "Amplitude returned HTTP {status}: {body}"),
        }
    }
}

impl std::error::Error for AmplitudeError {}

struct AmplitudeConfig {
    api_key: String,
    options: AmplitudeOptions,
    network: Arc<dyn NetworkProvider>,
}

#[derive(Clone, Default)]
pub struct Amplitude {
    config: Option<Arc<AmplitudeConfig>>,
}

impl Amplitude {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(
        &mut self,
        api_key: &str,
        options: Option<AmplitudeOptions>,
    ) -> Result<(), AmplitudeError> {
        if api_key.trim().is_empty() {
            return Err(AmplitudeError::InvalidInput(
                "api_key must not be empty".into(),
            ));
        }
        let options = options.unwrap_or_default();
        if !(options.server_url.starts_with("https://")
            || options.server_url.starts_with("http://"))
            || options.timeout_ms == 0
        {
            return Err(AmplitudeError::InvalidInput(
                "server_url must use HTTP(S) and timeout_ms must be positive".into(),
            ));
        }
        let network = get_network_provider()
            .upgrade()
            .ok_or_else(|| AmplitudeError::Network("No network provider available".into()))?;
        self.config = Some(Arc::new(AmplitudeConfig {
            api_key: api_key.into(),
            options,
            network,
        }));
        Ok(())
    }

    /// Sends one event immediately; success means the API accepted the request.
    pub async fn track(&self, mut event: AmplitudeEvent) -> Result<(), AmplitudeError> {
        let config = self.config.as_ref().ok_or(AmplitudeError::NotInitialized)?;
        let has_identity = [&event.user_id, &event.device_id]
            .iter()
            .any(|id| id.as_ref().is_some_and(|id| !id.trim().is_empty()));
        if event.event_type.trim().is_empty() || !has_identity {
            return Err(AmplitudeError::InvalidInput(
                "event_type and either user_id or device_id are required".into(),
            ));
        }
        event
            .time
            .get_or_insert_with(|| Utc::now().timestamp_millis());
        event
            .insert_id
            .get_or_insert_with(|| Uuid::new_v4().to_string());
        event
            .library
            .get_or_insert_with(|| format!("amplitude-rust-core/{}", env!("CARGO_PKG_VERSION")));
        let body = serde_json::to_vec(&json!({
            "api_key": config.api_key,
            "events": [event],
        }))
        .map_err(|e| AmplitudeError::InvalidInput(e.to_string()))?;
        let args = RequestArgs {
            url: config.options.server_url.clone(),
            timeout_ms: config.options.timeout_ms,
            headers: Some(HashMap::from([
                ("Content-Type".into(), "application/json".into()),
                ("Accept".into(), "application/json".into()),
            ])),
            body: Some(body),
            disable_file_streaming: Some(true),
            ..RequestArgs::new()
        };
        let mut response = config.network.send(&HttpMethod::POST, &args).await;
        if let Some(error) = response.error {
            return Err(AmplitudeError::Network(error));
        }
        match response.status_code {
            Some(200) => Ok(()),
            Some(status) => Err(AmplitudeError::Api {
                status,
                body: response
                    .data
                    .as_mut()
                    .and_then(|data| data.read_to_string().ok())
                    .unwrap_or_default(),
            }),
            None => Err(AmplitudeError::Network("No HTTP response received".into())),
        }
    }
}
