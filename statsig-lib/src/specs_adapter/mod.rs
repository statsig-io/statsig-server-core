pub use specs_adapter_trait::*;

pub use statsig_bootstrap_specs_adapter::StatsigBootstrapSpecsAdapter;
pub use statsig_customized_specs_adapter::StatsigCustomizedSpecsAdapter;
#[cfg(feature = "with_grpc")]
pub use statsig_grpc_specs_adapter::StatsigGrpcSpecsAdapter;
pub use statsig_http_specs_adapter::StatsigHttpSpecsAdapter;

mod statsig_bootstrap_specs_adapter;
mod statsig_customized_specs_adapter;
#[cfg(feature = "with_grpc")]
mod statsig_grpc_specs_adapter;
mod statsig_http_specs_adapter;

mod specs_adapter_trait;

pub const DEFAULT_INIT_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, Clone)]
pub struct SpecAdapterConfig {
    pub adapter_type: SpecsAdapterType,
    pub specs_url: String,
    pub init_timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub enum SpecsAdapterType {
    NetworkGrpcWebsocket,
    NetworkHttp,
}
