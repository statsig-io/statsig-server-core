pub use net_provider_global::NetworkProviderGlobal;

use super::NetworkProvider;
use net_provider_noop::NetworkProviderNoop;
use std::sync::Arc;

#[cfg(not(feature = "custom_network_provider"))]
pub mod net_provider_curl;

mod net_provider_global;
mod net_provider_noop;

lazy_static::lazy_static! {
    static ref NOOP_NETWORK_PROVIDER: Arc<dyn NetworkProvider> = Arc::new(NetworkProviderNoop {});
}

#[allow(unused_variables)]
pub fn get_network_provider(sdk_key: &str) -> Arc<dyn NetworkProvider> {
    #[cfg(not(feature = "custom_network_provider"))]
    return Arc::new(net_provider_curl::NetworkProviderCurl::get_instance(
        sdk_key,
    ));

    #[cfg(feature = "custom_network_provider")]
    return NetworkProviderGlobal::try_get().unwrap_or_else(|| NOOP_NETWORK_PROVIDER.clone());
}
