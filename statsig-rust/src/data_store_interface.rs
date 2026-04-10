use std::fmt::Display;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{hashing::HashUtil, StatsigErr, StatsigOptions};

pub enum RequestPath {
    RulesetsV2,
    RulesetsV1,
    IDListsV1,
    IDList,
}

impl Display for RequestPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            RequestPath::IDListsV1 => "/v1/get_id_lists",
            RequestPath::IDList => "id_list",
            RequestPath::RulesetsV2 => "/v2/download_config_specs",
            RequestPath::RulesetsV1 => "/v1/download_config_specs",
        };
        write!(f, "{value}")
    }
}

pub enum CompressFormat {
    PlainText,
    Gzip,
    StatsigBr,
}

impl Display for CompressFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            CompressFormat::PlainText => "plain_text",
            CompressFormat::Gzip => "gzip",
            CompressFormat::StatsigBr => "statsig-br",
        };
        write!(f, "{value}")
    }
}

#[derive(Deserialize, Serialize)]
pub struct DataStoreResponse {
    pub result: Option<String>,
    pub time: Option<u64>,
}

#[derive(Deserialize, Serialize)]
pub struct DataStoreBytesResponse {
    pub result: Option<Vec<u8>>,
    pub time: Option<u64>,
}

#[derive(Clone, Debug)]
pub(crate) struct DataStoreCacheKeys {
    pub plain_text: String,
    pub statsig_br: String,
}

impl DataStoreCacheKeys {
    pub(crate) fn from_selected_key(key: &str) -> Self {
        Self {
            plain_text: data_store_key_with_compress_format(key, CompressFormat::PlainText),
            statsig_br: data_store_key_with_compress_format(key, CompressFormat::StatsigBr),
        }
    }
}

#[async_trait]
pub trait DataStoreTrait: Send + Sync {
    async fn initialize(&self) -> Result<(), StatsigErr>;
    async fn shutdown(&self) -> Result<(), StatsigErr>;
    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr>;
    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr>;
    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        time: Option<u64>,
    ) -> Result<(), StatsigErr> {
        let _ = (key, value, time);
        Err(StatsigErr::BytesNotImplemented)
    }

    async fn get_bytes(&self, key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
        let _ = key;
        Err(StatsigErr::BytesNotImplemented)
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool;
}

#[derive(Clone, Debug, Default)]
pub enum DataStoreKeyVersion {
    #[default]
    V2Hashed,
    V3HumanReadable,
}

impl From<&str> for DataStoreKeyVersion {
    fn from(level: &str) -> Self {
        match level.to_lowercase().as_str() {
            "v2" | "2" => DataStoreKeyVersion::V2Hashed,
            "v3" | "3" => DataStoreKeyVersion::V3HumanReadable,
            _ => DataStoreKeyVersion::default(),
        }
    }
}

#[must_use]
pub(crate) fn get_data_store_key(
    path: RequestPath,
    sdk_key: &str,
    hashing: &HashUtil,
    options: &StatsigOptions,
) -> String {
    let key = match options
        .data_store_key_schema_version
        .clone()
        .unwrap_or_default()
    {
        DataStoreKeyVersion::V3HumanReadable => {
            let mut key = sdk_key.to_string();
            key.truncate(20);
            key
        }
        DataStoreKeyVersion::V2Hashed => hashing.hash(sdk_key, &crate::HashAlgorithm::Sha256),
    };

    format!("statsig|{path}|{}|{key}", CompressFormat::PlainText)
}

fn data_store_key_with_compress_format(key: &str, compress_format: CompressFormat) -> String {
    let target = format!("|{compress_format}|");

    [
        CompressFormat::PlainText,
        CompressFormat::Gzip,
        CompressFormat::StatsigBr,
    ]
    .into_iter()
    .map(|format| format!("|{format}|"))
    .find_map(|current| {
        key.contains(&current)
            .then(|| key.replacen(&current, &target, 1))
    })
    .unwrap_or_else(|| key.to_string())
}
