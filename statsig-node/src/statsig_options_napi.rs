use std::sync::Arc;

use napi_derive::napi;

use crate::{data_store_napi::DataStore, observability_client_napi::ObservabilityClient};
use sigstat::{
    data_store_interface::DataStoreTrait, ObservabilityClient as ObservabilityClientTrait,
};

#[napi(object, object_to_js = false)]
pub struct StatsigOptions {
    pub specs_url: Option<String>,
    pub log_event_url: Option<String>,
    pub output_log_level: Option<String>,
    pub observability_client: Option<ObservabilityClient>,
    pub data_store: Option<DataStore>,
    pub specs_sync_interval_ms: Option<u32>,
}

impl From<StatsigOptions> for sigstat::StatsigOptions {
    fn from(options: StatsigOptions) -> Self {
        sigstat::StatsigOptions {
            output_log_level: options.output_log_level.map(|s| s.as_str().into()),
            specs_url: options.specs_url,
            log_event_url: options.log_event_url,
            data_store: options
                .data_store
                .map(|store| Arc::new(store) as Arc<dyn DataStoreTrait>),
            observability_client: options
                .observability_client
                .map(|client| Arc::new(client) as Arc<dyn ObservabilityClientTrait>),
            specs_sync_interval_ms: options.specs_sync_interval_ms,
            ..Default::default()
        }
    }
}
