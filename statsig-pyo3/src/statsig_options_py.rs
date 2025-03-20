use crate::observability_client_py::ObservabilityClientPy;
use crate::pyo_utils::py_dict_to_map;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use statsig_rust::{output_logger::LogLevel, ObservabilityClient, StatsigOptions};
use std::sync::Arc;
use std::sync::Weak;

#[pyclass(name = "StatsigOptions")]
pub struct StatsigOptionsPy {
    #[pyo3(get, set)]
    pub specs_url: Option<String>,
    #[pyo3(get, set)]
    pub specs_sync_interval_ms: Option<u32>,
    #[pyo3(get, set)]
    pub init_timeout_ms: Option<u64>,
    #[pyo3(get, set)]
    pub log_event_url: Option<String>,
    #[pyo3(get, set)]
    pub disable_all_logging: Option<bool>,
    #[pyo3(get, set)]
    pub event_logging_flush_interval_ms: Option<u32>,
    #[pyo3(get, set)]
    pub event_logging_max_queue_size: Option<u32>,
    #[pyo3(get, set)]
    pub enable_id_lists: Option<bool>,
    #[pyo3(get, set)]
    pub enable_user_agent_parsing: Option<bool>,
    #[pyo3(get, set)]
    pub enable_country_lookup: Option<bool>,
    #[pyo3(get, set)]
    pub id_lists_url: Option<String>,
    #[pyo3(get, set)]
    pub id_lists_sync_interval_ms: Option<u32>,
    #[pyo3(get, set)]
    pub fallback_to_statsig_api: Option<bool>,
    #[pyo3(get, set)]
    pub environment: Option<String>,
    #[pyo3(get, set)]
    pub output_log_level: Option<String>,
    #[pyo3(get, set)]
    pub global_custom_fields: Option<Py<PyDict>>,
    #[pyo3(get, set)]
    pub observability_client: Option<Py<ObservabilityClientPy>>,
}

#[pymethods]
impl StatsigOptionsPy {
    #[new]
    pub fn new() -> Self {
        Self {
            specs_url: None,
            specs_sync_interval_ms: None,
            init_timeout_ms: None,
            log_event_url: None,
            disable_all_logging: None,
            event_logging_flush_interval_ms: None,
            event_logging_max_queue_size: None,
            enable_id_lists: None,
            enable_user_agent_parsing: None,
            id_lists_url: None,
            id_lists_sync_interval_ms: None,
            fallback_to_statsig_api: None,
            environment: None,
            output_log_level: None,
            enable_country_lookup: None,
            global_custom_fields: None,
            observability_client: None,
        }
    }
}

impl StatsigOptionsPy {
    pub fn to_statsig_options(
        &self,
        py: Python,
    ) -> (StatsigOptions, Option<Arc<ObservabilityClientPy>>) {
        let ob_client_strong =
            self.observability_client
                .as_ref()
                .and_then(
                    |py_ob_client| match py_ob_client.extract::<ObservabilityClientPy>(py) {
                        Ok(ob_client) => Some(Arc::new(ob_client.clone())),
                        Err(_e) => None,
                    },
                );

        let ob_client_weak = ob_client_strong
            .as_ref()
            .map(|arc| Arc::downgrade(arc) as Weak<dyn ObservabilityClient>);

        (
            StatsigOptions {
                specs_url: self.specs_url.clone(),
                specs_adapter: None,
                specs_sync_interval_ms: self.specs_sync_interval_ms,
                init_timeout_ms: self.init_timeout_ms,
                data_store: None,
                spec_adapters_config: None,
                log_event_url: self.log_event_url.clone(),
                disable_all_logging: self.disable_all_logging,
                event_logging_adapter: None,
                event_logging_flush_interval_ms: self.event_logging_flush_interval_ms,
                event_logging_max_queue_size: self.event_logging_max_queue_size,
                enable_id_lists: self.enable_id_lists,
                id_lists_url: self.id_lists_url.clone(),
                id_lists_sync_interval_ms: self.id_lists_sync_interval_ms,
                fallback_to_statsig_api: self.fallback_to_statsig_api,
                environment: self.environment.clone(),
                id_lists_adapter: None,
                override_adapter: None,
                output_log_level: self
                    .output_log_level
                    .as_ref()
                    .map(|level| LogLevel::from(level.as_str())),
                observability_client: ob_client_weak,
                service_name: None,
                enable_user_agent_parsing: self.enable_user_agent_parsing,
                enable_country_lookup: self.enable_country_lookup,
                global_custom_fields: self
                    .global_custom_fields
                    .as_ref()
                    .map(|dict| py_dict_to_map(dict.bind(py))),
            },
            ob_client_strong,
        )
    }
}

impl Default for StatsigOptionsPy {
    fn default() -> Self {
        Self::new()
    }
}
