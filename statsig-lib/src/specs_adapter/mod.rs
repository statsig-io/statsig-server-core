pub use specs_adapter_trait::*;

pub use statsig_bootstrap_specs_adapter::StatsigBootstrapSpecsAdapter;
pub use statsig_customized_specs_adapter::StatsigCustomizedSpecsAdapter;
#[cfg(feature = "with_grpc")]
pub use statsig_grpc_specs_adapter::StatsigGrpcSpecsAdapter;
pub use statsig_http_specs_adapter::StatsigHttpSpecsAdapter;
pub use statsig_local_file_specs_adapter::StatsigLocalFileSpecsAdapter;

mod statsig_bootstrap_specs_adapter;
mod statsig_customized_specs_adapter;
mod statsig_data_store_specs_adapter;
#[cfg(feature = "with_grpc")]
mod statsig_grpc_specs_adapter;
mod statsig_http_specs_adapter;
mod statsig_local_file_specs_adapter;

mod specs_adapter_trait;

pub const DEFAULT_INIT_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, Clone)]
pub struct SpecAdapterConfig {
    pub adapter_type: SpecsAdapterType,
    pub init_timeout_ms: u64,

    pub specs_url: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SpecsAdapterType {
    NetworkGrpcWebsocket,
    NetworkHttp,
    DataStore,
}
