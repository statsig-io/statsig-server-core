use crate::statsig_err::StatsigErr;
use crate::StatsigRuntime;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::{self, Debug};
use std::sync::Arc;
use tokio::time::Duration;

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecsSource {
    Uninitialized = 0,
    NoValues,
    Error,
    Loading,
    Network,
    Bootstrap,
    Adapter(String),
}

impl fmt::Display for SpecsSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            SpecsSource::Adapter(name) => {
                let type_name = format!("Adapter{}", name);
                type_name
            }
            SpecsSource::Uninitialized => "Uninitialized".to_string(),
            SpecsSource::NoValues => "NoValues".to_string(),
            SpecsSource::Error => "Error".to_string(),
            SpecsSource::Loading => "Loading".to_string(),
            SpecsSource::Network => "Network".to_string(),
            SpecsSource::Bootstrap => "Bootstrap".to_string(),
        };
        write!(f, "{}", s)
    }
}

#[async_trait]
pub trait SpecsAdapter: Send + Sync {
    fn initialize(&self, listener: Arc<dyn SpecsUpdateListener>);

    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr>;

    async fn shutdown(
        &self,
        timeout: Duration,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr>;

    async fn schedule_background_sync(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr>;

    fn get_type_name(&self) -> String;
}

pub struct SpecsUpdate {
    pub data: String,
    pub source: SpecsSource,
    pub received_at: u64,
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
pub struct SpecsInfo {
    pub lcut: Option<u64>,
    pub checksum: Option<String>,
    pub source: SpecsSource,
}

impl SpecsInfo {
    pub fn empty() -> Self {
        Self {
            lcut: None,
            checksum: None,
            source: SpecsSource::NoValues,
        }
    }

    pub fn error() -> Self {
        Self {
            lcut: None,
            checksum: None,
            source: SpecsSource::Error,
        }
    }
}

pub trait SpecsUpdateListener: Send + Sync {
    fn did_receive_specs_update(&self, update: SpecsUpdate) -> Result<(), StatsigErr>;

    fn get_current_specs_info(&self) -> SpecsInfo;
}

impl fmt::Debug for dyn SpecsAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_type_name())
    }
}
