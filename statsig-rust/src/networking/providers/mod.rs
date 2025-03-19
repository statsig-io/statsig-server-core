pub use net_provider_global::NetworkProviderGlobal;

use super::NetworkProvider;
use net_provider_noop::NetworkProviderNoop;
use std::sync::{Arc, Weak};

#[cfg(not(feature = "custom_network_provider"))]
pub mod net_provider_reqwest;

mod net_provider_global;
mod net_provider_noop;

lazy_static::lazy_static! {
    static ref NOOP_NETWORK_PROVIDER: Arc<dyn NetworkProvider> = Arc::new(NetworkProviderNoop {});
}

#[cfg(not(feature = "custom_network_provider"))]
lazy_static::lazy_static! {
    static ref REQWEST_NETWORK_PROVIDER: Arc<dyn NetworkProvider> = Arc::new(net_provider_reqwest::NetworkProviderReqwest {});
}

#[cfg(feature = "custom_network_provider")]
pub fn get_network_provider() -> Weak<dyn NetworkProvider> {
    NetworkProviderGlobal::try_get().unwrap_or_else(|| Arc::downgrade(&NOOP_NETWORK_PROVIDER))
}

#[cfg(not(feature = "custom_network_provider"))]
pub fn get_network_provider() -> Weak<dyn NetworkProvider> {
    Arc::downgrade(&REQWEST_NETWORK_PROVIDER)
}
