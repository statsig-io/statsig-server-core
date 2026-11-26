use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

pub use serde_json::Value as JsonValue;
pub type JsonObject = serde_json::Map<String, JsonValue>;

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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "u8", into = "u8")]
#[repr(u8)]
pub enum LogLevel {
    #[default]
    None = 0,
    Error = 1,
    Warn = 2,
    Verbose = 3,
    Debug = 4,
}

impl TryFrom<u8> for LogLevel {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, &'static str> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Error),
            2 => Ok(Self::Warn),
            3 => Ok(Self::Verbose),
            4 => Ok(Self::Debug),
            _ => Err("log_level must be between 0 and 4"),
        }
    }
}

impl From<LogLevel> for u8 {
    fn from(level: LogLevel) -> Self {
        level as Self
    }
}

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub flush_interval_millis: Option<f64>,
    pub flush_max_retries: Option<f64>,
    pub flush_queue_size: Option<f64>,
    pub instance_name: Option<String>,
    pub log_level: Option<LogLevel>,
    pub server_zone: Option<ServerZone>,
    pub use_batch: Option<bool>,
    pub server_url: Option<String>,
    pub opt_out: Option<bool>,
    pub plan: Option<Plan>,
    pub ingestion_metadata: Option<IngestionMetadata>,
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
    #[serde(alias = "productId")]
    pub product_id: Option<String>,
    #[serde(alias = "revenueType")]
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GroupName {
    Single(String),
    Multiple(Vec<String>),
}

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Event {
    pub event_type: String,
    pub options: EventOptions,
    pub event_properties: Option<JsonObject>,
    pub user_properties: Option<JsonObject>,
    pub groups: Option<BTreeMap<String, GroupName>>,
    pub group_properties: Option<JsonObject>,
    pub library: Option<String>,
}

impl Event {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            ..Self::default()
        }
    }
}

pub type BaseEvent = Event;
