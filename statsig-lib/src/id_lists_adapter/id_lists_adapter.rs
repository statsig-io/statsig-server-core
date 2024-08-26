use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use crate::{StatsigErr};

pub type IdListsResponse = HashMap<String, IdListEntry>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IdListEntry {
    pub name: String,
    pub url: Option<String>,

    #[serde(rename = "fileID")]
    pub file_id: Option<String>,

    pub size: u64,
    pub creation_time: i64,

    #[serde(skip)]
    pub loaded_ids: HashSet<String>
}


#[async_trait]
pub trait IdListsAdapter: Send + Sync {
    async fn start(
        self: Arc<Self>,
        runtime_handle: &Handle
    ) -> Result<(), StatsigErr>;

    async fn sync_id_lists(&self) -> Result<(), StatsigErr>;

    fn does_list_contain_id(&self, list_name: &str, id: &str) -> bool;

    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr>;
}