use crate::statsig_err::StatsigErr;
use async_trait::async_trait;
use serde::Serialize;
use std::fmt;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SpecsSource {
    Uninitialized = 0,
    NoValues,
    Error,
    Loading,
    Network,
    Bootstrap,
}

impl fmt::Display for SpecsSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            SpecsSource::Uninitialized => "Uninitialized",
            SpecsSource::NoValues => "NoValues",
            SpecsSource::Error => "Error",
            SpecsSource::Loading => "Loading",
            SpecsSource::Network => "Network",
            SpecsSource::Bootstrap => "Bootstrap",
        };
        write!(f, "{}", s)
    }
}

#[async_trait]
pub trait SpecsAdapter: Send + Sync {
    async fn start(
        self: Arc<Self>,
        runtime_handle: &Handle,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr>;

    async fn manually_sync_specs(&self, current_store_lcut: Option<u64>) -> Result<(), StatsigErr>;

    async fn shutdown(&self, timeout: Duration) -> Result<(), StatsigErr>;
}

pub struct SpecsUpdate {
    pub data: String,
    pub source: SpecsSource,
    pub received_at: u64,
}

pub struct SpecsInfo {
    pub lcut: Option<u64>,
    pub source: SpecsSource,
}

pub trait SpecsUpdateListener: Send + Sync {
    fn did_receive_specs_update(&self, update: SpecsUpdate);

    fn get_current_specs_info(&self) -> SpecsInfo;
}
