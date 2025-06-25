use crate::{StatsigErr, StatsigRuntime};
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, fmt};

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
pub trait IdListsAdapter: Any + Send + Sync {
    /// Called during Statsig::initialize. Mostly to attach the listener.
    /// Scheduling background threads should be done in the IdListsAdapter::schedule_background_sync function.
    ///
    /// # Arguments
    ///
    /// * `statsig_runtime` - The Statsig runtime instance.
    /// * `listener` - The listener to push updates to.
    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn IdListsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr>;

    /// Called during Statsig::shutdown or Statsig::shutdown_with_timeout.
    /// This function gives some grace period to the adapter to finish its work.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The timeout for the shutdown.
    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr>;

    /// Called during Statsig::initialize.
    /// This function is used to schedule the background sync thread and is called just after IdListsAdapter::start
    ///
    /// # Arguments
    ///
    /// * `statsig_runtime` - The Statsig runtime instance.
    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr>;

    /// Returns the type name of the adapter. Used for logging and error messages.
    fn get_type_name(&self) -> String;
}

pub trait IdListsUpdateListener: Send + Sync {
    fn get_current_id_list_metadata(&self) -> HashMap<String, IdListMetadata>;

    fn did_receive_id_list_updates(&self, updates: HashMap<String, IdListUpdate>);
}

impl fmt::Debug for dyn IdListsAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_type_name())
    }
}
