use napi_derive::napi;
use parking_lot::Mutex;
use serde_json::Value;
use statsig_rust::{Amplitude as RustAmplitude, AmplitudeEvent as RustAmplitudeEvent};
use std::collections::HashMap;

#[napi(object)]
pub struct AmplitudeOptions {
    pub server_url: Option<String>,
    pub timeout_ms: Option<u32>,
}

#[napi(object)]
pub struct AmplitudeEvent {
    pub event_type: String,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub event_properties: Option<HashMap<String, Value>>,
    pub user_properties: Option<HashMap<String, Value>>,
    pub time: Option<i64>,
    pub insert_id: Option<String>,
    pub session_id: Option<i64>,
}

#[napi]
pub struct Amplitude {
    inner: Mutex<RustAmplitude>,
}

#[napi]
impl Amplitude {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RustAmplitude::new()),
        }
    }

    #[napi]
    pub fn init(&self, api_key: String, options: Option<AmplitudeOptions>) -> napi::Result<()> {
        let mut config = statsig_rust::AmplitudeOptions::default();
        if let Some(options) = options {
            if let Some(server_url) = options.server_url {
                config.server_url = server_url;
            }
            if let Some(timeout_ms) = options.timeout_ms {
                config.timeout_ms = u64::from(timeout_ms);
            }
        }
        self.inner
            .lock()
            .init(&api_key, Some(config))
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn track(&self, event: AmplitudeEvent) -> napi::Result<()> {
        let client = self.inner.lock().clone();
        client
            .track(RustAmplitudeEvent {
                event_type: event.event_type,
                user_id: event.user_id,
                device_id: event.device_id,
                event_properties: event.event_properties.map(|map| map.into_iter().collect()),
                user_properties: event.user_properties.map(|map| map.into_iter().collect()),
                time: event.time,
                insert_id: event.insert_id,
                session_id: event.session_id,
                library: Some(format!("amplitude-node-core/{}", env!("CARGO_PKG_VERSION"))),
            })
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }
}
