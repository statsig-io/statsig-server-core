#![doc = include_str!("../README.md")]

mod batch;
mod identify;
mod types;

pub use identify::{Identify, IdentifyOperation};
pub use types::*;

use chrono::Utc;
use reqwest::{Client, StatusCode, Url};
use serde::Serialize;
use std::{fmt, io::Write, sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum AmplitudeError {
    InvalidInput(&'static str),
    Http(Arc<reqwest::Error>),
    Api { status: u16 },
}

impl AmplitudeError {
    /// Returns the HTTP response status, or zero if no response was received.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Api { status } => *status,
            _ => 0,
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Api { status } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::Http(error) => error.is_timeout() || error.is_connect(),
            Self::InvalidInput(_) => false,
        }
    }
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
            Self::Http(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

#[derive(Serialize)]
struct UploadRequest<'a> {
    api_key: &'a str,
    events: Vec<JsonValue>,
}

/// Server-side Amplitude client.
#[derive(Clone)]
pub struct Amplitude {
    api_key: String,
    config: Config,
    server_url: Url,
    http: Client,
    queue: Arc<batch::Queue>,
    max_retries: u32,
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
        let interval =
            Duration::try_from_secs_f64(config.flush_interval_millis.unwrap_or(0.0) / 1_000.0)
                .ok()
                .filter(|interval| std::time::Instant::now().checked_add(*interval).is_some())
                .ok_or(AmplitudeError::InvalidInput(
                    "flush_interval_millis must be a finite nonnegative duration",
                ))?;
        let capacity = config_count(config.flush_queue_size.unwrap_or(200.0), 1.0).ok_or(
            AmplitudeError::InvalidInput("flush_queue_size must be a positive whole number"),
        )?;
        let max_retries = config_count(config.flush_max_retries.unwrap_or(0.0), 0.0).ok_or(
            AmplitudeError::InvalidInput("flush_max_retries must be a nonnegative whole number"),
        )?;
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
            .map_err(http_error)?;

        Ok(Self {
            api_key: api_key.into(),
            config,
            server_url,
            http,
            queue: Arc::new(batch::Queue::new(interval, capacity as usize)),
            max_retries,
        })
    }

    /// Queues an event and resolves to its HTTP status, or zero when opted out.
    pub async fn track(&self, event: Event) -> Result<u16, AmplitudeError> {
        if self.config.opt_out == Some(true) {
            return Ok(0);
        }
        if event.event_type.trim().is_empty() {
            return Err(AmplitudeError::InvalidInput("event_type is required"));
        }
        validate_identity(&event.options)?;
        self.enqueue(event).await
    }

    /// Applies user property operations in a single identify request.
    pub async fn identify(
        &self,
        operations: Identify,
        options: EventOptions,
        event_properties: Option<JsonObject>,
    ) -> Result<u16, AmplitudeError> {
        if self.config.opt_out == Some(true) {
            return Ok(0);
        }
        self.track(Event::from_identify(operations, options, event_properties)?)
            .await
    }

    async fn enqueue(&self, mut event: Event) -> Result<u16, AmplitudeError> {
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

        self.queue
            .push(event)
            .deliver(|events| async move { self.upload(&events).await })
            .await
    }

    async fn upload(&self, events: &[Event]) -> Result<u16, AmplitudeError> {
        self.log(
            LogLevel::Debug,
            format_args!("Uploading {} event(s)", events.len()),
        );
        let request = UploadRequest {
            api_key: &self.api_key,
            events: events
                .iter()
                .map(ingestion_event)
                .collect::<Result<_, _>>()?,
        };
        let mut retries = 0;
        loop {
            let result = self
                .http
                .post(self.server_url.clone())
                .json(&request)
                .send()
                .await;
            let error = match result {
                Ok(response) if response.status() == StatusCode::OK => {
                    self.log(
                        LogLevel::Verbose,
                        format_args!("Uploaded {} event(s)", events.len()),
                    );
                    return Ok(response.status().as_u16());
                }
                Ok(response) => AmplitudeError::Api {
                    status: response.status().as_u16(),
                },
                Err(error) => http_error(error),
            };
            if retries == self.max_retries || !error.is_retryable() {
                self.log(LogLevel::Error, format_args!("{error}"));
                return Err(error);
            }
            self.log(
                LogLevel::Warn,
                format_args!("Retrying upload after {error}"),
            );
            tokio::time::sleep(Duration::from_millis(100 << retries.min(4))).await;
            retries += 1;
        }
    }

    fn log(&self, level: LogLevel, message: fmt::Arguments<'_>) {
        if self.config.log_level.unwrap_or_default() >= level {
            let name = self.config.instance_name.as_deref().unwrap_or("Amplitude");
            let _ = writeln!(std::io::stderr().lock(), "[{name}] {level:?}: {message}");
        }
    }
}

fn ingestion_event(event: &Event) -> Result<JsonValue, AmplitudeError> {
    let mut payload = serde_json::to_value(event)
        .map_err(|_| AmplitudeError::InvalidInput("Event could not be encoded"))?;
    if let Some(fields) = payload.as_object_mut() {
        if let Some(JsonValue::Object(mut options)) = fields.remove("options") {
            if let Some(product_id) = options.remove("product_id") {
                options.insert("productId".into(), product_id);
            }
            if let Some(revenue_type) = options.remove("revenue_type") {
                options.insert("revenueType".into(), revenue_type);
            }
            fields.append(&mut options);
        }
    }
    Ok(payload)
}

fn http_error(error: reqwest::Error) -> AmplitudeError {
    AmplitudeError::Http(Arc::new(error.without_url()))
}

fn config_count(value: f64, min: f64) -> Option<u32> {
    (value.fract() == 0.0 && (min..=f64::from(u32::MAX)).contains(&value)).then_some(value as u32)
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
