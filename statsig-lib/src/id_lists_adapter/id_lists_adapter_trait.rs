use crate::{StatsigErr, StatsigRuntime};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IdListMetadata {
    pub name: String,
    pub url: String,

    #[serde(rename = "fileID")]
    pub file_id: Option<String>,

    pub size: u64,
    pub creation_time: i64,
}

pub struct IdListUpdate {
    pub raw_changeset: Option<String>,
    pub new_metadata: IdListMetadata,
}

#[async_trait]
pub trait IdListsAdapter: Send + Sync {
    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn IdListsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr>;

    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr>;

    async fn sync_id_lists(&self) -> Result<(), StatsigErr>;
}

pub trait IdListsUpdateListener: Send + Sync {
    fn get_current_id_list_metadata(&self) -> HashMap<String, IdListMetadata>;

    fn did_receive_id_list_updates(&self, updates: HashMap<String, IdListUpdate>);
}
