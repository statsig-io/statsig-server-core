#![doc = include_str!("../README.md")]

mod batch;
mod destination;
mod identify;
mod timeline;
mod types;

pub use destination::AmplitudeDestinationPlugin;
pub use identify::{Identify, IdentifyOperation};
pub use timeline::{
    Delivery, DeliveryResult, DestinationPlugin, EventPlugin, FlushResults, Plugin, PluginFuture,
    PluginId, Timeline,
};
pub use types::*;

use std::{fmt, sync::Arc};

#[derive(Clone, Debug)]
pub enum AmplitudeError {
    InvalidInput(&'static str),
    Http(Arc<reqwest::Error>),
    Api { status: u16 },
    Plugin { code: u16, message: String },
    Closed,
    WorkerStopped,
}

impl AmplitudeError {
    /// Returns the HTTP or plugin status when available, or zero.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Api { status } => *status,
            Self::Plugin { code, .. } => *code,
            _ => 0,
        }
    }
}

impl fmt::Display for AmplitudeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => f.write_str(message),
            Self::Http(error) => write!(f, "Amplitude HTTP client error: {error}"),
            Self::Api { status } => write!(f, "Amplitude returned HTTP {status}"),
            Self::Plugin { message, .. } => f.write_str(message),
            Self::Closed => f.write_str("Amplitude client has been shut down"),
            Self::WorkerStopped => {
                f.write_str("Amplitude delivery worker stopped before completing the batch")
            }
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

/// Server-side Amplitude client. Clones share plugins and pending events.
#[derive(Clone)]
pub struct Amplitude {
    timeline: Arc<Timeline>,
    config: Arc<Config>,
}

impl Amplitude {
    pub fn new(api_key: &str, configuration: Option<Config>) -> Result<Self, AmplitudeError> {
        let config = configuration.unwrap_or_default();
        let destination = AmplitudeDestinationPlugin::new(api_key, config.clone())?;
        let client = Self {
            timeline: Arc::new(Timeline::new(config.opt_out.unwrap_or(false))),
            config: Arc::new(config),
        };
        client.add(Plugin::Destination(Arc::new(destination)))?;
        Ok(client)
    }

    pub fn configuration(&self) -> &Config {
        &self.config
    }

    /// Initializes and registers a plugin. Use the returned ID to remove it.
    pub fn add(&self, plugin: Plugin) -> Result<PluginId, AmplitudeError> {
        plugin.setup(self)?;
        self.timeline.add(plugin)
    }

    pub fn remove(&self, plugin: PluginId) -> bool {
        self.timeline.remove(plugin)
    }

    /// Flushes queued work and returns each destination's results.
    pub async fn flush(&self) -> Vec<FlushResults> {
        self.timeline.flush().await
    }

    /// Stops accepting events and drains pending deliveries.
    pub async fn shutdown(&self) -> Vec<FlushResults> {
        self.timeline.shutdown().await
    }

    /// Returns zero for a queued or filtered event. With the default flush
    /// interval of zero, waits for delivery and returns the HTTP status.
    pub async fn track(&self, event: Event) -> Result<u16, AmplitudeError> {
        let delivery = self.timeline.apply(event)?;
        if self.config.flush_interval_millis.unwrap_or_default() > 0.0 && delivery.is_pending() {
            return Ok(0);
        }
        delivery.wait().await?.status()
    }

    /// Waits for the first destination's delivery outcome at any flush interval.
    pub async fn track_with_result(&self, event: Event) -> Result<DeliveryResult, AmplitudeError> {
        self.timeline.apply(event)?.wait().await
    }

    pub async fn identify(
        &self,
        operations: Identify,
        options: EventOptions,
        event_properties: Option<JsonObject>,
    ) -> Result<u16, AmplitudeError> {
        self.track(self.identify_event(operations, options, event_properties)?)
            .await
    }

    /// Applies identify operations and waits for the delivery outcome.
    pub async fn identify_with_result(
        &self,
        operations: Identify,
        options: EventOptions,
        event_properties: Option<JsonObject>,
    ) -> Result<DeliveryResult, AmplitudeError> {
        self.track_with_result(self.identify_event(operations, options, event_properties)?)
            .await
    }

    fn identify_event(
        &self,
        operations: Identify,
        options: EventOptions,
        event_properties: Option<JsonObject>,
    ) -> Result<Event, AmplitudeError> {
        if self.config.opt_out == Some(true) {
            return Ok(Event {
                options,
                event_properties,
                ..Event::new("$identify")
            });
        }
        Event::from_identify(operations, options, event_properties)
    }
}
