pub use specs_adapter_trait::*;
mod specs_adapter_trait;
pub mod statsig_http_specs_adapter;
pub mod statsig_customized_specs_adapter;
#[cfg(feature = "with_grpc")]
pub mod statsig_grpc_specs_adapter;

pub const DEFAULT_INIT_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, Clone)]
pub struct SpecAdapterConfig {
  pub adapter_type: AdapterType,
  pub specs_url: String,
  pub init_timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub enum AdapterType {
  NetworkGrpcWebsocket,
  NetworkHttp,
}