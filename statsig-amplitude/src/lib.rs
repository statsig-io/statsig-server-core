//! Opt-in forwarding alongside an existing Statsig event logging adapter.
//! Amplitude failures are best effort: they never cause successful Statsig uploads
//! to be retried. Configure an Amplitude callback to observe delivery failures.

#![doc = include_str!("../README.md")]

use amplitude_rust::{Amplitude, Event};
use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use statsig_rust::{
    log_event_payload::LogEventRequest, EventLoggingAdapter, StatsigErr, StatsigRuntime,
};
use std::sync::Arc;

pub struct StatsigAmplitudeAdapter {
    primary: Arc<dyn EventLoggingAdapter>,
    amplitude: Arc<Amplitude>,
}

impl StatsigAmplitudeAdapter {
    pub fn new(primary: Arc<dyn EventLoggingAdapter>, amplitude: Arc<Amplitude>) -> Self {
        Self { primary, amplitude }
    }

    /// Map identifiable custom and exposure events. Internal events without a user
    /// ID are skipped. No user attributes, custom attributes, IPs, or emails are copied.
    pub fn map_events(request: &LogEventRequest) -> Result<Vec<Event>, StatsigErr> {
        let events = request.payload.events.as_array().ok_or_else(|| {
            StatsigErr::SerializationError("Statsig events must be an array".into())
        })?;
        let mut mapped = Vec::new();
        for source in events {
            let Some(user_id) = source
                .pointer("/user/userID")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
            else {
                continue;
            };
            let name = source
                .get("eventName")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| {
                    StatsigErr::SerializationError("Statsig eventName is required".into())
                })?;
            let time = source.get("time").and_then(Value::as_u64).ok_or_else(|| {
                StatsigErr::SerializationError("Statsig event time is required".into())
            })?;
            let mut event = Event::new(name);
            event.user_id = Some(user_id.into());
            event.time = Some(time);
            event.event_properties.insert(
                "statsig".into(),
                json!({
                    "value": source.get("value"),
                    "metadata": source.get("metadata"),
                    "secondary_exposures": source.get("secondaryExposures"),
                    "statsig_metadata": source.get("statsigMetadata"),
                }),
            );
            // Only mapped fields participate: request retry counters, SDK metadata,
            // and private user attributes cannot change an event's deduplication ID.
            let identity = json!([name, user_id, time, event.event_properties]);
            let mut encoded = String::new();
            canonical_json(&identity, &mut encoded);
            event.insert_id = format!("statsig-{:x}", Sha256::digest(encoded.as_bytes()));
            mapped.push(event);
        }
        Ok(mapped)
    }
}

// serde_json's preserve_order feature is enabled by statsig-rust. Sort recursively
// so HashMap insertion order cannot change IDs across Statsig retry attempts.
fn canonical_json(value: &Value, output: &mut String) {
    match value {
        Value::Object(object) => {
            output.push('{');
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort_unstable();
            for key in keys {
                output.push_str(&serde_json::to_string(key).unwrap());
                output.push(':');
                canonical_json(&object[key], output);
                output.push(',');
            }
            output.push('}');
        }
        Value::Array(array) => {
            output.push('[');
            for value in array {
                canonical_json(value, output);
                output.push(',');
            }
            output.push(']');
        }
        _ => output.push_str(&value.to_string()),
    }
}

#[allow(clippy::double_must_use)]
#[async_trait]
impl EventLoggingAdapter for StatsigAmplitudeAdapter {
    async fn start(&self, runtime: &Arc<StatsigRuntime>) -> Result<(), StatsigErr> {
        self.primary.start(runtime).await
    }

    async fn log_events(&self, request: LogEventRequest) -> Result<bool, StatsigErr> {
        // The primary result owns Statsig retry semantics. Forward only after it
        // succeeds, and never turn a successful Statsig upload into a failure.
        let success = self.primary.log_events(request.clone()).await?;
        if !success {
            return Ok(false);
        }
        match Self::map_events(&request) {
            Ok(events) => {
                if let Err(error) = self.amplitude.send_events(events).await {
                    statsig_rust::log_w!(
                        "StatsigAmplitudeAdapter",
                        "Amplitude forwarding failed: {error}"
                    );
                }
            }
            Err(_) => {
                statsig_rust::log_w!(
                    "StatsigAmplitudeAdapter",
                    "Unable to map Statsig events for Amplitude"
                );
            }
        }
        Ok(true)
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        let primary = self.primary.shutdown().await;
        let amplitude = self
            .amplitude
            .shutdown()
            .await
            .map_err(|error| StatsigErr::ShutdownFailure(error.to_string()));
        primary.and(amplitude)
    }

    fn should_schedule_background_flush(&self) -> bool {
        self.primary.should_schedule_background_flush()
    }
}
