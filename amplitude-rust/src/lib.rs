#![doc = include_str!("../README.md")]

mod identify;
mod types;

pub use identify::{Identify, IdentifyOperation};
pub use types::*;

use chrono::Utc;
use reqwest::{Client, StatusCode, Url};
use serde::Serialize;
use std::{fmt, time::Duration};
use uuid::Uuid;

#[derive(Debug)]
pub enum AmplitudeError {
    NotImplemented,
    InvalidInput(&'static str),
    Http(reqwest::Error),
    Api { status: u16 },
}

impl fmt::Display for AmplitudeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented => f.write_str("Not implemented"),
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
struct UploadRequest<'a> {
    api_key: &'a str,
    events: [BaseEvent; 1],
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<UploadOptions>,
}

#[derive(Serialize)]
struct UploadOptions {
    min_id_length: u32,
}

pub struct FlushTask;

impl FlushTask {
    pub async fn wait(self) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }
}

/// Server-side Amplitude client.
#[derive(Clone)]
pub struct Amplitude {
    api_key: String,
    config: Config,
    server_url: Url,
    http: Client,
}

impl Amplitude {
    pub fn new(api_key: &str, configuration: Option<Config>) -> Result<Self, AmplitudeError> {
        if api_key.trim().is_empty() {
            return Err(AmplitudeError::InvalidInput("api_key must not be empty"));
        }
        let config = configuration.unwrap_or_default();
        let timeout_ms = config.timeout_ms.unwrap_or(10_000);
        if timeout_ms == 0 {
            return Err(AmplitudeError::InvalidInput("timeout_ms must be positive"));
        }
        let default_url = match (
            config.server_zone.unwrap_or_default(),
            config.use_batch.unwrap_or(false),
        ) {
            (ServerZone::Us, false) => "https://api2.amplitude.com/2/httpapi",
            (ServerZone::Eu, false) => "https://api.eu.amplitude.com/2/httpapi",
            (ServerZone::Us, true) => "https://api2.amplitude.com/batch",
            (ServerZone::Eu, true) => "https://api.eu.amplitude.com/batch",
        };
        let server_url = Url::parse(config.server_url.as_deref().unwrap_or(default_url))
            .ok()
            .filter(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
            .ok_or(AmplitudeError::InvalidInput(
                "server_url must be a valid HTTP(S) URL",
            ))?;
        let http = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| AmplitudeError::Http(error.without_url()))?;

        Ok(Self {
            api_key: api_key.into(),
            config,
            server_url,
            http,
        })
    }

    pub fn configuration(&self) -> Result<Config, AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub fn set_configuration(&mut self, _configuration: Config) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub fn api_key(&self) -> Result<&str, AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub fn set_api_key(&mut self, _api_key: &str) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    /// Sends one event immediately.
    pub async fn track(&self, event: BaseEvent) -> Result<(), AmplitudeError> {
        if self.config.opt_out == Some(true) {
            return Ok(());
        }
        if event.event_type.trim().is_empty() {
            return Err(AmplitudeError::InvalidInput("event_type is required"));
        }
        validate_identity(&event.options)?;
        self.upload(event).await
    }

    /// Applies user property operations in a single identify request.
    pub async fn identify(
        &self,
        operations: Identify,
        options: EventOptions,
        event_properties: Option<JsonObject>,
    ) -> Result<(), AmplitudeError> {
        if self.config.opt_out == Some(true) {
            return Ok(());
        }
        validate_identity(&options)?;
        self.upload(BaseEvent {
            options,
            event_properties,
            user_properties: Some(identify::user_properties(operations)?),
            ..BaseEvent::new("$identify")
        })
        .await
    }

    pub async fn group_identify(
        &self,
        _group_type: &str,
        _group: &str,
        _operations: Identify,
        _options: Option<EventOptions>,
        _event_properties: Option<JsonObject>,
        _user_properties: Option<JsonObject>,
    ) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub async fn revenue(
        &self,
        _transaction: Revenue,
        _options: EventOptions,
    ) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub async fn set_group(
        &self,
        _group_type: &str,
        _group: GroupName,
        _options: EventOptions,
    ) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub fn flush(&self) -> Result<Vec<FlushTask>, AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub fn add(&mut self, _plugin: PluginId, _stage: PluginType) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub fn remove(&mut self, _plugin: PluginId) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    pub async fn shutdown(&self) -> Result<(), AmplitudeError> {
        Err(AmplitudeError::NotImplemented)
    }

    async fn upload(&self, mut event: BaseEvent) -> Result<(), AmplitudeError> {
        let options = &mut event.options;
        options
            .time
            .get_or_insert_with(|| Utc::now().timestamp_millis());
        options
            .insert_id
            .get_or_insert_with(|| Uuid::new_v4().to_string());
        options.plan = options.plan.take().or_else(|| self.config.plan.clone());
        options.ingestion_metadata = options
            .ingestion_metadata
            .take()
            .or_else(|| self.config.ingestion_metadata.clone());
        event.library.get_or_insert_with(|| {
            concat!("amplitude-rust-core/", env!("CARGO_PKG_VERSION")).into()
        });

        let request = UploadRequest {
            api_key: &self.api_key,
            events: [event],
            options: self
                .config
                .min_id_length
                .map(|min_id_length| UploadOptions { min_id_length }),
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

fn validate_identity(options: &EventOptions) -> Result<(), AmplitudeError> {
    let has_identity = [options.user_id.as_deref(), options.device_id.as_deref()]
        .into_iter()
        .flatten()
        .any(|id| !id.trim().is_empty());
    if has_identity {
        Ok(())
    } else {
        Err(AmplitudeError::InvalidInput(
            "user_id or device_id is required",
        ))
    }
}
