use std::sync::{Arc, Weak};

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

impl StatsigOptions {
    /**
     * There is some reference capture issue around ObservabilityClient and StatsigOptions.
     * By storing the ObservabilityClient in a Weak reference and having the strong
     * Arc reference be owned by StatsigNapi, we can avoid the issue.
     */
    pub fn safe_convert_to_inner(
        mut self,
    ) -> (
        Option<Arc<sigstat::StatsigOptions>>,
        Option<Arc<ObservabilityClient>>,
    ) {
        let obs_client = self.observability_client.take().map(Arc::new);

        let mut weak_obs_client: Option<Weak<dyn ObservabilityClientTrait>> = None;
        if let Some(obs_client) = &obs_client {
            weak_obs_client =
                Some(Arc::downgrade(obs_client) as Weak<dyn ObservabilityClientTrait>);
        }

        self.observability_client = None;

        let inner = sigstat::StatsigOptions {
            specs_url: self.specs_url,
            log_event_url: self.log_event_url,
            output_log_level: self.output_log_level.map(|s| s.as_str().into()),
            observability_client: weak_obs_client,
            data_store: self
                .data_store
                .map(|store| Arc::new(store) as Arc<dyn DataStoreTrait>),
            specs_sync_interval_ms: self.specs_sync_interval_ms,
            ..Default::default()
        };

        (Some(Arc::new(inner)), obs_client)
    }
}
