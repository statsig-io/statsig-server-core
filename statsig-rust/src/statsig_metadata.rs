use crate::log_e;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

lazy_static! {
    static ref STATSIG_METADATA: RwLock<StatsigMetadata> = RwLock::new(StatsigMetadata::new());
}

pub const SDK_VERSION: &str = "0.15.2-rc.2602030137";

const TAG: &str = stringify!(StatsigMetadata);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatsigMetadata {
    pub sdk_type: String,
    pub sdk_version: String,

    #[serde(rename = "sessionID")]
    pub session_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "service_name")]
    pub service_name: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatsigMetadataWithLogEventExtras {
    #[serde(flatten)]
    pub base: StatsigMetadata,

    pub flushing_interval_ms: u64,
    pub batch_size: usize,
    pub max_pending_batches: usize,
    pub flush_type: String,
}

impl StatsigMetadata {
    fn new() -> Self {
        Self {
            sdk_version: SDK_VERSION.to_string(),
            sdk_type: "statsig-server-core".to_string(),
            session_id: Uuid::new_v4().to_string(),
            os: None,
            arch: None,
            language_version: None,
            service_name: None,
        }
    }

    pub fn update_values(sdk_type: String, os: String, arch: String, language_version: String) {
        match STATSIG_METADATA.try_write_for(std::time::Duration::from_secs(5)) {
            Some(mut metadata) => {
                metadata.sdk_type = sdk_type;
                metadata.os = Some(os);
                metadata.arch = Some(arch);
                metadata.language_version = Some(language_version);
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to clone StatsigMetadata: Failed to lock STATSIG_METADATA"
                );
            }
        }
    }

    pub fn update_service_name(service_name: Option<String>) {
        match STATSIG_METADATA.try_write_for(std::time::Duration::from_secs(5)) {
            Some(mut metadata) => {
                metadata.service_name = service_name;
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to clone StatsigMetadata: Failed to lock STATSIG_METADATA"
                );
            }
        }
    }

    #[must_use]
    pub fn get_constant_request_headers(sdk_key: &str) -> HashMap<String, String> {
        let meta = Self::get_metadata();

        HashMap::from([
            ("STATSIG-API-KEY".to_string(), sdk_key.to_string()),
            ("STATSIG-SDK-TYPE".to_string(), meta.sdk_type),
            ("STATSIG-SDK-VERSION".to_string(), meta.sdk_version),
            ("STATSIG-SERVER-SESSION-ID".to_string(), meta.session_id),
        ])
    }

    #[must_use]
    pub fn get_metadata() -> StatsigMetadata {
        match STATSIG_METADATA.try_read_for(std::time::Duration::from_secs(5)) {
            Some(metadata) => metadata.clone(),
            None => {
                log_e!(
                    TAG,
                    "Failed to clone StatsigMetadata: Failed to lock STATSIG_METADATA"
                );
                StatsigMetadata::new()
            }
        }
    }

    #[must_use]
    pub fn get_as_json() -> Value {
        json!(StatsigMetadata::get_metadata())
    }

    #[must_use]
    pub fn get_with_log_event_extras(
        flushing_interval_ms: u64,
        batch_size: usize,
        max_pending_batches: usize,
        flush_type: String,
    ) -> StatsigMetadataWithLogEventExtras {
        StatsigMetadataWithLogEventExtras {
            base: StatsigMetadata::get_metadata(),
            flushing_interval_ms,
            batch_size,
            max_pending_batches,
            flush_type,
        }
    }
}
