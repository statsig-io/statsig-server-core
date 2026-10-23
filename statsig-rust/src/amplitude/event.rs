use super::{AmplitudeCallback, AmplitudeError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

pub const MAX_PROPERTY_KEYS: usize = 1024;
pub const MAX_STRING_LENGTH: usize = 1000;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmplitudePlan {
    pub branch: Option<String>,
    pub source: Option<String>,
    pub version: Option<String>,
    pub version_id: Option<String>,
}

impl AmplitudePlan {
    pub fn to_json_object(&self) -> Value {
        let mut value = serde_json::to_value(self).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .retain(|_, v| v.as_str().is_some_and(|s| !s.is_empty()));
        value
    }
    pub fn set_branch(&mut self, value: impl Into<String>) -> &mut Self {
        self.branch = Some(value.into());
        self
    }
    pub fn set_source(&mut self, value: impl Into<String>) -> &mut Self {
        self.source = Some(value.into());
        self
    }
    pub fn set_version(&mut self, value: impl Into<String>) -> &mut Self {
        self.version = Some(value.into());
        self
    }
    pub fn set_version_id(&mut self, value: impl Into<String>) -> &mut Self {
        self.version_id = Some(value.into());
        self
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmplitudeIngestionMetadata {
    pub source_name: Option<String>,
    pub source_version: Option<String>,
}
impl AmplitudeIngestionMetadata {
    pub fn to_json_object(&self) -> Value {
        let mut object = Map::new();
        for (key, value) in [
            ("source_name", &self.source_name),
            ("source_version", &self.source_version),
        ] {
            if let Some(value) = value.as_ref().filter(|s| !s.is_empty()) {
                object.insert(key.into(), json!(value));
            }
        }
        Value::Object(object)
    }
    pub fn set_source_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.source_name = Some(value.into());
        self
    }
    pub fn set_source_version(&mut self, value: impl Into<String>) -> &mut Self {
        self.source_version = Some(value.into());
        self
    }
}

/// All fields of Amplitude-Java Event. `time` remains an alias for the demo API;
/// when supplied, `timestamp` (the Java field name) takes precedence.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AmplitudeEvent {
    pub event_type: String,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub time: Option<i64>,
    pub timestamp: Option<i64>,
    pub location_lat: f64,
    pub location_lng: f64,
    pub app_version: Option<String>,
    pub version_name: Option<String>,
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
    pub idfa: Option<String>,
    pub idfv: Option<String>,
    pub adid: Option<String>,
    pub android_id: Option<String>,
    pub language: Option<String>,
    pub partner_id: Option<String>,
    pub ip: Option<String>,
    pub product_id: Option<String>,
    pub revenue_type: Option<String>,
    pub currency: Option<String>,
    pub event_properties: Option<Map<String, Value>>,
    pub user_properties: Option<Map<String, Value>>,
    pub groups: Option<Map<String, Value>>,
    pub group_properties: Option<Map<String, Value>>,
    pub price: Option<f64>,
    pub quantity: i32,
    pub revenue: Option<f64>,
    pub event_id: i32,
    pub session_id: Option<i64>,
    pub insert_id: Option<String>,
    pub library: Option<String>,
    pub plan: Option<AmplitudePlan>,
    pub ingestion_metadata: Option<AmplitudeIngestionMetadata>,
    #[serde(skip)]
    pub callback: Option<AmplitudeCallback>,
}

impl std::fmt::Debug for AmplitudeEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AmplitudeEvent")
            .field("event_type", &self.event_type)
            .finish_non_exhaustive()
    }
}

impl Default for AmplitudeEvent {
    fn default() -> Self {
        Self {
            event_type: String::new(),
            user_id: None,
            device_id: None,
            time: Some(Utc::now().timestamp_millis()),
            timestamp: None,
            location_lat: 0.0,
            location_lng: 0.0,
            app_version: None,
            version_name: None,
            platform: None,
            os_name: None,
            os_version: None,
            device_brand: None,
            device_manufacturer: None,
            device_model: None,
            carrier: None,
            country: None,
            region: None,
            city: None,
            dma: None,
            idfa: None,
            idfv: None,
            adid: None,
            android_id: None,
            language: None,
            partner_id: None,
            ip: None,
            product_id: None,
            revenue_type: None,
            currency: None,
            event_properties: None,
            user_properties: None,
            groups: None,
            group_properties: None,
            price: None,
            quantity: 0,
            revenue: None,
            event_id: 0,
            session_id: Some(-1),
            insert_id: Some(uuid::Uuid::new_v4().to_string()),
            library: None,
            plan: None,
            ingestion_metadata: None,
            callback: None,
        }
    }
}

impl AmplitudeEvent {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            ..Self::default()
        }
    }
    pub fn with_identity(
        event_type: impl Into<String>,
        user_id: Option<String>,
        device_id: Option<String>,
    ) -> Result<Self, AmplitudeError> {
        if user_id.is_none() && device_id.is_none() {
            return Err(AmplitudeError::InvalidInput(
                "Event must have one defined userId and/or deviceId".into(),
            ));
        }
        Ok(Self {
            user_id,
            device_id,
            ..Self::new(event_type)
        })
    }
    pub fn to_json_object(&self) -> Result<Value, AmplitudeError> {
        let mut event = json!({
            "event_type": self.event_type, "user_id": self.user_id, "device_id": self.device_id,
            "time": self.timestamp.or(self.time).unwrap_or_else(|| Utc::now().timestamp_millis()),
            "location_lat": self.location_lat, "location_lng": self.location_lng,
            "library": self.library.clone().unwrap_or_else(|| format!("amplitude-rust-core/{}", env!("CARGO_PKG_VERSION"))),
            "event_id": self.event_id, "session_id": self.session_id.unwrap_or(-1),
        });
        event["version_name"] = json!(self.version_name);
        event["platform"] = json!(self.platform);
        event["os_name"] = json!(self.os_name);
        event["os_version"] = json!(self.os_version);
        event["device_brand"] = json!(self.device_brand);
        event["device_manufacturer"] = json!(self.device_manufacturer);
        event["device_model"] = json!(self.device_model);
        event["carrier"] = json!(self.carrier);
        event["country"] = json!(self.country);
        event["region"] = json!(self.region);
        event["city"] = json!(self.city);
        event["dma"] = json!(self.dma);
        event["idfa"] = json!(self.idfa);
        event["idfv"] = json!(self.idfv);
        event["adid"] = json!(self.adid);
        event["android_id"] = json!(self.android_id);
        event["language"] = json!(self.language);
        event["partner_id"] = json!(self.partner_id);
        event["ip"] = json!(self.ip);
        if let Some(value) = &self.app_version {
            event["app_version"] = json!(value);
        }
        if let Some(value) = &self.insert_id {
            event["insert_id"] = json!(value);
        }
        for (name, value) in [
            ("event_properties", &self.event_properties),
            ("user_properties", &self.user_properties),
            ("groups", &self.groups),
            ("group_properties", &self.group_properties),
        ] {
            event[name] = truncate_properties(&Value::Object(value.clone().unwrap_or_default()))?;
        }
        if self.revenue.is_some() || self.price.is_some() {
            event["quantity"] = json!(if self.quantity > 0 { self.quantity } else { 1 });
            if let Some(value) = self.price {
                event["price"] = json!(value);
            }
            if let Some(value) = self.revenue {
                event["revenue"] = json!(value);
            }
            for (key, value) in [
                ("productId", &self.product_id),
                ("revenueType", &self.revenue_type),
                ("currency", &self.currency),
            ] {
                if let Some(value) = value {
                    event[key] = json!(value);
                }
            }
        }
        if let Some(plan) = &self.plan {
            event["plan"] = plan.to_json_object();
        }
        if let Some(metadata) = &self.ingestion_metadata {
            event["ingestion_metadata"] = metadata.to_json_object();
        }
        Ok(event)
    }
    pub fn set_event_properties(&mut self, value: Option<Map<String, Value>>) -> &mut Self {
        self.event_properties = value;
        self
    }
    pub fn add_event_property(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        let map = self.event_properties.get_or_insert_with(Map::new);
        let key = key.into();
        if value.is_null() {
            map.remove(&key);
        } else {
            map.insert(key, value);
        }
        self
    }
    pub fn set_user_properties(&mut self, value: Option<Map<String, Value>>) -> &mut Self {
        self.user_properties = value;
        self
    }
    pub fn add_user_property(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        let map = self.user_properties.get_or_insert_with(Map::new);
        let key = key.into();
        if value.is_null() {
            map.remove(&key);
        } else {
            map.insert(key, value);
        }
        self
    }
    pub fn set_groups(&mut self, value: Option<Map<String, Value>>) -> &mut Self {
        self.groups = value;
        self
    }
    pub fn add_group(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        let map = self.groups.get_or_insert_with(Map::new);
        let key = key.into();
        if value.is_null() {
            map.remove(&key);
        } else {
            map.insert(key, value);
        }
        self
    }
    pub fn set_group_properties(&mut self, value: Option<Map<String, Value>>) -> &mut Self {
        self.group_properties = value;
        self
    }
    pub fn add_group_property(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        let map = self.group_properties.get_or_insert_with(Map::new);
        let key = key.into();
        if value.is_null() {
            map.remove(&key);
        } else {
            map.insert(key, value);
        }
        self
    }
}

pub fn truncate_properties(value: &Value) -> Result<Value, AmplitudeError> {
    match value {
        Value::Object(object) => {
            if object.len() > MAX_PROPERTY_KEYS {
                return Err(AmplitudeError::InvalidInput(
                    "Too many properties (more than 1024) in JSON".into(),
                ));
            }
            Ok(Value::Object(
                object
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), truncate_properties(value)?)))
                    .collect::<Result<_, AmplitudeError>>()?,
            ))
        }
        Value::Array(array) => Ok(Value::Array(
            array
                .iter()
                .map(truncate_properties)
                .collect::<Result<_, _>>()?,
        )),
        Value::String(value) => Ok(Value::String(String::from_utf16_lossy(
            &value
                .encode_utf16()
                .take(MAX_STRING_LENGTH)
                .collect::<Vec<_>>(),
        ))),
        _ => Ok(value.clone()),
    }
}
