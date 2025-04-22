use crate::statsig_err::StatsigErr;
use crate::StatsigRuntime;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::{self, Debug};
use std::sync::Arc;
use std::time::Duration;

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

impl SpecsSource {
    pub fn new_from_string(s: &str) -> Self {
        if s.starts_with("Adapter(") {
            let name = s
                .strip_prefix("Adapter(")
                .and_then(|s| s.strip_suffix(")"))
                .unwrap_or("");
            return SpecsSource::Adapter(name.to_string());
        }

        match s {
            "Uninitialized" => SpecsSource::Uninitialized,
            "NoValues" => SpecsSource::NoValues,
            "Error" => SpecsSource::Error,
            "Loading" => SpecsSource::Loading,
            "Network" => SpecsSource::Network,
            "Bootstrap" => SpecsSource::Bootstrap,
            _ => SpecsSource::Error,
        }
    }
}

impl fmt::Display for SpecsSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            SpecsSource::Adapter(name) => {
                let type_name = format!("Adapter({name})");
                type_name
            }
            SpecsSource::Uninitialized => "Uninitialized".to_string(),
            SpecsSource::NoValues => "NoValues".to_string(),
            SpecsSource::Error => "Error".to_string(),
            SpecsSource::Loading => "Loading".to_string(),
            SpecsSource::Network => "Network".to_string(),
            SpecsSource::Bootstrap => "Bootstrap".to_string(),
        };
        write!(f, "{s}")
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpecsInfo {
    pub lcut: Option<u64>,
    pub checksum: Option<String>,
    pub zstd_dict_id: Option<String>,
    pub source: SpecsSource,
}

impl SpecsInfo {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            lcut: None,
            checksum: None,
            zstd_dict_id: None,
            source: SpecsSource::NoValues,
        }
    }

    #[must_use]
    pub fn error() -> Self {
        Self {
            lcut: None,
            checksum: None,
            zstd_dict_id: None,
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
