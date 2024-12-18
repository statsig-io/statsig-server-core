use std::collections::HashMap;

use lazy_static::lazy_static;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

lazy_static! {
    pub static ref STATSIG_METADATA: StatsigMetadata = StatsigMetadata::new();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsigMetadata {
    pub sdk_type: String,
    pub sdk_version: String,
    pub session_id: String,
}

impl StatsigMetadata {
    fn new() -> Self {
        Self {
            sdk_version: "0.0.1-beta.126".to_string(),
            sdk_type: "statsig-server-core".to_string(),
            session_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn get_constant_request_headers(sdk_key: &str) -> HashMap<String, String> {
        let meta = &STATSIG_METADATA;

        HashMap::from([
            ("STATSIG-API-KEY".to_string(), sdk_key.to_string()),
            ("STATSIG-SDK-TYPE".to_string(), meta.sdk_type.clone()),
            ("STATSIG-SDK-VERSION".to_string(), meta.sdk_version.clone()),
            ("STATSIG-SERVER-SESSION-ID".to_string(), meta.session_id.clone()),
        ])
    }

    pub fn get_metadata() -> StatsigMetadata {
        let meta = &STATSIG_METADATA;
        StatsigMetadata {
           sdk_version: meta.sdk_version.clone(),
           sdk_type: meta.sdk_type.clone(),
           session_id: meta.session_id.clone(),
       }
    }

    pub fn get_as_json() -> Value {
        json!(StatsigMetadata::get_metadata())
    }
}
