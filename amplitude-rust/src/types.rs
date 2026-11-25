use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

pub use serde_json::Value as JsonValue;
pub type JsonObject = serde_json::Map<String, JsonValue>;
pub type PluginId = u64;
pub type CallbackId = u64;

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Plan {
    pub branch: Option<String>,
    pub source: Option<String>,
    pub version: Option<String>,
    pub version_id: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct IngestionMetadata {
    pub source_name: Option<String>,
    pub source_version: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerZone {
    #[default]
    Us,
    Eu,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventLogLevel {
    None,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    Before,
    Enrichment,
    Destination,
    Observe,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub flush_queue_size: Option<u32>,
    pub flush_interval_millis: Option<u64>,
    pub flush_max_retries: Option<u32>,
    pub min_id_length: Option<u32>,
    pub server_zone: Option<ServerZone>,
    pub use_batch: Option<bool>,
    pub server_url: Option<String>,
    pub opt_out: Option<bool>,
    pub plan: Option<Plan>,
    pub ingestion_metadata: Option<IngestionMetadata>,
    pub event_log_level: Option<EventLogLevel>,
    pub callback: Option<CallbackId>,
    /// HTTP request timeout in milliseconds. Defaults to 10,000.
    pub timeout_ms: Option<u64>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EventOptions {
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub time: Option<i64>,
    pub app_version: Option<String>,
    pub platform: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub device_brand: Option<String>,
    pub device_manufacturer: Option<String>,
    pub device_model: Option<String>,
    pub carrier: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub dma: Option<String>,
    pub language: Option<String>,
    pub price: Option<f64>,
    pub quantity: Option<i64>,
    pub revenue: Option<f64>,
    #[serde(rename(serialize = "productId"), alias = "productId")]
    pub product_id: Option<String>,
    #[serde(rename(serialize = "revenueType"), alias = "revenueType")]
    pub revenue_type: Option<String>,
    pub currency: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lng: Option<f64>,
    pub ip: Option<String>,
    pub idfa: Option<String>,
    pub idfv: Option<String>,
    pub adid: Option<String>,
    pub android_id: Option<String>,
    pub event_id: Option<i64>,
    pub session_id: Option<i64>,
    pub insert_id: Option<String>,
    pub plan: Option<Plan>,
    pub ingestion_metadata: Option<IngestionMetadata>,
    pub partner_id: Option<String>,
    pub version_name: Option<String>,
    pub user_agent: Option<String>,
    /// Local delivery hook identifier.
    #[serde(skip_serializing)]
    pub callback: Option<CallbackId>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GroupName {
    Single(String),
    Multiple(Vec<String>),
}

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BaseEvent {
    pub event_type: String,
    #[serde(flatten)]
    pub options: EventOptions,
    pub event_properties: Option<JsonObject>,
    pub user_properties: Option<JsonObject>,
    pub groups: Option<BTreeMap<String, GroupName>>,
    pub group_properties: Option<JsonObject>,
    pub library: Option<String>,
}

impl BaseEvent {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            ..Self::default()
        }
    }
}

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Revenue {
    pub price: f64,
    pub quantity: Option<i64>,
    pub product_id: Option<String>,
    pub revenue_type: Option<String>,
    pub receipt: Option<String>,
    pub receipt_sig: Option<String>,
    pub properties: Option<JsonObject>,
    pub revenue: Option<f64>,
    pub currency: Option<String>,
}
