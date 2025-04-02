use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::data_store_interface::DataStoreTrait;
use crate::evaluation::dynamic_value::DynamicValue;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::id_lists_adapter::IdListsAdapter;
use crate::output_logger::LogLevel;
use crate::{
    serialize_if_not_none, ObservabilityClient, OverrideAdapter, SpecAdapterConfig, SpecsAdapter,
};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Weak};

pub const DEFAULT_INIT_TIMEOUT_MS: u64 = 3000;

#[derive(Clone, Default)]
pub struct StatsigOptions {
    pub data_store: Option<Arc<dyn DataStoreTrait>>, // External DataStore
    pub enable_id_lists: Option<bool>,
    pub enable_user_agent_parsing: Option<bool>,
    pub enable_country_lookup: Option<bool>,
    pub environment: Option<String>,

    pub disable_network: Option<bool>, // Disable all out-going network including get configs, log_events...
    pub disable_all_logging: Option<bool>,

    pub event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>,
    pub event_logging_flush_interval_ms: Option<u32>,
    pub event_logging_max_queue_size: Option<u32>,

    pub fallback_to_statsig_api: Option<bool>,

    pub id_lists_adapter: Option<Arc<dyn IdListsAdapter>>,
    pub id_lists_sync_interval_ms: Option<u32>,
    pub id_lists_url: Option<String>,

    pub init_timeout_ms: Option<u64>,
    pub log_event_url: Option<String>,
    pub observability_client: Option<Weak<dyn ObservabilityClient>>,
    pub output_log_level: Option<LogLevel>,
    pub override_adapter: Option<Arc<dyn OverrideAdapter>>,

    pub spec_adapters_config: Option<Vec<SpecAdapterConfig>>, // Specs to customized spec adapter, order matters, reflecting priority of trying
    pub specs_adapter: Option<Arc<dyn SpecsAdapter>>,
    pub specs_sync_interval_ms: Option<u32>,
    pub specs_url: Option<String>,
    pub service_name: Option<String>,
    pub global_custom_fields: Option<HashMap<String, DynamicValue>>,
}

impl StatsigOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // The builder method for more complex initialization
    #[must_use]
    pub fn builder() -> StatsigOptionsBuilder {
        StatsigOptionsBuilder::default()
    }
}

#[derive(Default)]
pub struct StatsigOptionsBuilder {
    inner: StatsigOptions,
}

impl StatsigOptionsBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // Specs

    #[must_use]
    pub fn specs_url(mut self, specs_url: Option<String>) -> Self {
        self.inner.specs_url = specs_url;
        self
    }

    #[must_use]
    pub fn specs_adapter(mut self, specs_adapter: Option<Arc<dyn SpecsAdapter>>) -> Self {
        self.inner.specs_adapter = specs_adapter;
        self
    }

    #[must_use]
    pub fn specs_sync_interval_ms(mut self, specs_sync_interval_ms: Option<u32>) -> Self {
        self.inner.specs_sync_interval_ms = specs_sync_interval_ms;
        self
    }

    // Event Logging

    #[must_use]
    pub fn log_event_url(mut self, log_event_url: Option<String>) -> Self {
        self.inner.log_event_url = log_event_url;
        self
    }

    #[must_use]
    pub fn disable_all_logging(mut self, disable_all_logging: Option<bool>) -> Self {
        self.inner.disable_all_logging = disable_all_logging;
        self
    }

    #[must_use]
    pub fn event_logging_adapter(
        mut self,
        event_logging_adapter: Option<Arc<dyn EventLoggingAdapter>>,
    ) -> Self {
        self.inner.event_logging_adapter = event_logging_adapter;
        self
    }

    #[must_use]
    pub fn event_logging_flush_interval_ms(
        mut self,
        event_logging_flush_interval_ms: Option<u32>,
    ) -> Self {
        self.inner.event_logging_flush_interval_ms = event_logging_flush_interval_ms;
        self
    }

    #[must_use]
    pub fn event_logging_max_queue_size(
        mut self,
        event_logging_max_queue_size: Option<u32>,
    ) -> Self {
        self.inner.event_logging_max_queue_size = event_logging_max_queue_size;
        self
    }

    // ID Lists

    #[must_use]
    pub fn enable_id_lists(mut self, enable_id_lists: Option<bool>) -> Self {
        self.inner.enable_id_lists = enable_id_lists;
        self
    }

    #[must_use]
    pub fn id_lists_url(mut self, id_lists_url: Option<String>) -> Self {
        self.inner.id_lists_url = id_lists_url;
        self
    }

    #[must_use]
    pub fn id_lists_adapter(mut self, id_lists_adapter: Option<Arc<dyn IdListsAdapter>>) -> Self {
        self.inner.id_lists_adapter = id_lists_adapter;
        self
    }

    #[must_use]
    pub fn id_lists_sync_interval_ms(mut self, id_lists_sync_interval_ms: Option<u32>) -> Self {
        self.inner.id_lists_sync_interval_ms = id_lists_sync_interval_ms;
        self
    }

    // Other

    #[must_use]
    pub fn environment(mut self, environment: Option<String>) -> Self {
        self.inner.environment = environment;
        self
    }

    #[must_use]
    pub fn output_log_level(mut self, output_log_level: Option<u32>) -> Self {
        if let Some(level) = output_log_level {
            self.inner.output_log_level = Some(LogLevel::from(level));
        }
        self
    }

    #[must_use]
    pub fn service_name(mut self, service_name: Option<String>) -> Self {
        self.inner.service_name = service_name;
        self
    }

    #[must_use]
    pub fn fallback_to_statsig_api(mut self, fallback_to_statsig_api: Option<bool>) -> Self {
        self.inner.fallback_to_statsig_api = fallback_to_statsig_api;
        self
    }

    #[must_use]
    pub fn enable_user_agent_parsing(mut self, enable_user_agent_parsing: Option<bool>) -> Self {
        self.inner.enable_user_agent_parsing = enable_user_agent_parsing;
        self
    }

    #[must_use]
    pub fn enable_country_lookup(mut self, enable_country_lookup: Option<bool>) -> Self {
        self.inner.enable_country_lookup = enable_country_lookup;
        self
    }

    #[must_use]
    pub fn global_custom_fields(
        mut self,
        global_custom_fields: Option<HashMap<String, DynamicValue>>,
    ) -> Self {
        self.inner.global_custom_fields = global_custom_fields;
        self
    }

    pub fn disable_network(mut self, disable_network: Option<bool>) -> Self {
        self.inner.disable_network = disable_network;
        self
    }

    #[must_use]
    pub fn observability_client(mut self, client: Option<Weak<dyn ObservabilityClient>>) -> Self {
        self.inner.observability_client = client;
        self
    }

    #[must_use]
    pub fn build(self) -> StatsigOptions {
        self.inner
    }
}

impl Serialize for StatsigOptions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("StatsigOptions", 20)?;
        serialize_if_not_none!(state, "spec_url", &self.specs_url);
        serialize_if_not_none!(
            state,
            "spec_adapter",
            &get_display_name(&self.specs_adapter)
        );
        serialize_if_not_none!(state, "spec_adapter_configs", &self.spec_adapters_config);
        serialize_if_not_none!(
            state,
            "specs_sync_interval_ms",
            &self.specs_sync_interval_ms
        );
        serialize_if_not_none!(state, "init_timeout_ms", &self.init_timeout_ms);

        serialize_if_not_none!(state, "data_store", &get_if_set(&self.data_store));

        serialize_if_not_none!(state, "log_event_url", &self.log_event_url);
        serialize_if_not_none!(state, "disable_all_logging", &self.disable_all_logging);
        serialize_if_not_none!(state, "disable_network", &self.disable_network);

        serialize_if_not_none!(state, "id_lists_url", &self.id_lists_url);
        serialize_if_not_none!(state, "enable_id_lists", &self.enable_id_lists);
        serialize_if_not_none!(
            state,
            "enable_user_agent_parsing",
            &self.enable_user_agent_parsing
        );
        serialize_if_not_none!(state, "enable_country_lookup", &self.enable_country_lookup);
        serialize_if_not_none!(
            state,
            "id_lists_sync_interval",
            &self.id_lists_sync_interval_ms
        );
        serialize_if_not_none!(state, "environment", &self.environment);
        serialize_if_not_none!(
            state,
            "id_list_adapter",
            &get_display_name(&self.id_lists_adapter)
        );
        serialize_if_not_none!(
            state,
            "fallback_to_statsig_api",
            &self.fallback_to_statsig_api
        );
        serialize_if_not_none!(
            state,
            "override_adapter",
            &get_if_set(&self.override_adapter)
        );
        serialize_if_not_none!(state, "service_name", &get_if_set(&self.service_name));
        serialize_if_not_none!(state, "global_custom_fields", &self.global_custom_fields);

        state.end()
    }
}

fn get_if_set<T>(s: &Option<T>) -> Option<&str> {
    s.as_ref().map(|_| "set")
}

fn get_display_name<T: fmt::Debug>(s: &Option<T>) -> Option<String> {
    s.as_ref().map(|st| format!("{st:?}"))
}
