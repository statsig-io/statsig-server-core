use crate::statsig_err::StatsigErr;
use crate::StatsigRuntime;
use async_trait::async_trait;
use serde::Serialize;
use std::fmt::{self, Debug};
use std::sync::Arc;
use tokio::time::Duration;

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
            SpecsSource::Uninitialized => "Uninitialized",
            SpecsSource::NoValues => "NoValues",
            SpecsSource::Error => "Error",
            SpecsSource::Loading => "Loading",
            SpecsSource::Network => "Network",
            SpecsSource::Bootstrap => "Bootstrap",
            SpecsSource::Adapter(name) => &format!("Adapter{}", name),
        };
        write!(f, "{}", s)
    }
}

#[async_trait]
pub trait SpecsAdapter: Send + Sync {
    async fn start(
        self: Arc<Self>,
        statsig_runtime: &Arc<StatsigRuntime>,
        listener: Arc<dyn SpecsUpdateListener + Send + Sync>,
    ) -> Result<(), StatsigErr>;

    async fn shutdown(
        &self,
        timeout: Duration,
        statsig_runtime: &Arc<StatsigRuntime>,
    ) -> Result<(), StatsigErr>;

    fn schedule_background_sync(
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

pub struct SpecsInfo {
    pub lcut: Option<u64>,
    pub source: SpecsSource,
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
