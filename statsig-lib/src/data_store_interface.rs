use std::fmt::Display;

use async_trait::async_trait;

use crate::StatsigErr;

pub enum RequestPath {
    RulesetsV2,
    IDListsV1,
    IDList
}

impl Display for RequestPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            RequestPath::IDListsV1 => "/v1/get_id_lists",
            RequestPath::RulesetsV2 => "/v2/download_config_specs",
            RequestPath::IDList => "id_list",
        };
        write!(f, "{}", value)
    }
}

pub enum CompressFormat {
    PlainText,
    Gzip
}

impl Display for CompressFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            CompressFormat::PlainText => "plain_text",
            CompressFormat::Gzip => "gzip",
        };
        write!(f, "{}", value)
    }
}

pub struct DataAdapterResponse {
    pub result: Option<String>,
    pub time: Option<u64>,
}

#[async_trait]
pub trait DataStoreTrait: Send + Sync {
    async fn initialize(&self) -> Result<(), StatsigErr>;
    async fn shutdown(&self) -> Result<(), StatsigErr>;
    async fn get(&self, key: &str) -> Result<DataAdapterResponse, StatsigErr>;
    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr>;
    async fn support_polling_updates_for(&self, path: RequestPath) -> bool;
}

pub fn get_data_adapter_dcs_key(hashed_key: &str) -> String {
    get_data_adapter_key(RequestPath::RulesetsV2, CompressFormat::PlainText, hashed_key)
}

pub fn get_data_adapter_key(path: RequestPath, compress: CompressFormat, hashed_key: &str) -> String {
    format!("statsig|{}|{}|{}",path, compress, hashed_key)
}