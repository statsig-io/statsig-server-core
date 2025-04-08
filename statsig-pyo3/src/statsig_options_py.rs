use crate::pyo_utils::py_dict_to_map;
use crate::{
    data_store_base_py::DataStoreBasePy, observability_client_base_py::ObservabilityClientBasePy,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::*;
use statsig_rust::data_store_interface::DataStoreTrait;
use statsig_rust::{output_logger::LogLevel, ObservabilityClient, StatsigOptions};
use std::sync::{Arc, Weak};

#[gen_stub_pyclass]
#[pyclass(name = "StatsigOptions")]
#[derive(FromPyObject, Default)]
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
    pub disable_network: Option<bool>,
    #[pyo3(get, set)]
    pub event_logging_flush_interval_ms: Option<u32>,
    #[pyo3(get, set)]
    pub event_logging_max_queue_size: Option<u32>,
    #[pyo3(get, set)]
    pub event_logging_max_pending_batch_queue_size: Option<u32>,
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
    pub observability_client: Option<Py<ObservabilityClientBasePy>>,
    #[pyo3(get, set)]
    pub data_store: Option<Py<DataStoreBasePy>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl StatsigOptionsPy {
    #[new]
    #[pyo3(signature = (
        specs_url=None,
        specs_sync_interval_ms=None,
        init_timeout_ms=None,
        log_event_url=None,
        disable_all_logging=None,
        disable_network=None,
        event_logging_flush_interval_ms=None,
        event_logging_max_queue_size=None,
        event_logging_max_pending_batch_queue_size=None,
        enable_id_lists=None,
        enable_user_agent_parsing=None,
        enable_country_lookup=None,
        id_lists_url=None,
        id_lists_sync_interval_ms=None,
        fallback_to_statsig_api=None,
        environment=None,
        output_log_level=None,
        global_custom_fields=None,
        observability_client=None,
        data_store=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        specs_url: Option<String>,
        specs_sync_interval_ms: Option<u32>,
        init_timeout_ms: Option<u64>,
        log_event_url: Option<String>,
        disable_all_logging: Option<bool>,
        disable_network: Option<bool>,
        event_logging_flush_interval_ms: Option<u32>,
        event_logging_max_queue_size: Option<u32>,
        event_logging_max_pending_batch_queue_size: Option<u32>,
        enable_id_lists: Option<bool>,
        enable_user_agent_parsing: Option<bool>,
        enable_country_lookup: Option<bool>,
        id_lists_url: Option<String>,
        id_lists_sync_interval_ms: Option<u32>,
        fallback_to_statsig_api: Option<bool>,
        environment: Option<String>,
        output_log_level: Option<String>,
        global_custom_fields: Option<Py<PyDict>>,
        observability_client: Option<Py<ObservabilityClientBasePy>>,
        data_store: Option<Py<DataStoreBasePy>>,
    ) -> Self {
        Self {
            specs_url,
            specs_sync_interval_ms,
            init_timeout_ms,
            log_event_url,
            disable_all_logging,
            event_logging_flush_interval_ms,
            event_logging_max_queue_size,
            event_logging_max_pending_batch_queue_size,
            enable_id_lists,
            enable_user_agent_parsing,
            enable_country_lookup,
            id_lists_url,
            id_lists_sync_interval_ms,
            fallback_to_statsig_api,
            environment,
            output_log_level,
            global_custom_fields,
            observability_client,
            data_store,
            disable_network,
        }
    }
}

pub(crate) fn safe_convert_to_statsig_options(
    py: Python,
    opts: Option<StatsigOptionsPy>,
) -> (Option<StatsigOptions>, Option<Arc<dyn ObservabilityClient>>) {
    let opts = match opts {
        Some(opts) => opts,
        None => return (None, None),
    };

    let (ob_client_weak, ob_client_strong) =
        extract_observability_client(py, &opts.observability_client);

    let opts = create_inner_statsig_options(py, opts, ob_client_weak);

    (Some(opts), ob_client_strong)
}

fn create_inner_statsig_options(
    py: Python,
    opts: StatsigOptionsPy,
    ob_client_weak: Option<Weak<dyn ObservabilityClient>>,
) -> StatsigOptions {
    StatsigOptions {
        specs_url: opts.specs_url.clone(),
        specs_adapter: None,
        specs_sync_interval_ms: opts.specs_sync_interval_ms,
        init_timeout_ms: opts.init_timeout_ms,
        data_store: opts.data_store.map(|store| {
            Arc::new(store.extract::<DataStoreBasePy>(py).unwrap()) as Arc<dyn DataStoreTrait>
        }),
        spec_adapters_config: None,
        log_event_url: opts.log_event_url.clone(),
        disable_all_logging: opts.disable_all_logging,
        event_logging_adapter: None,
        event_logging_flush_interval_ms: opts.event_logging_flush_interval_ms,
        event_logging_max_queue_size: opts.event_logging_max_queue_size,
        event_logging_max_pending_batch_queue_size: opts.event_logging_max_pending_batch_queue_size,
        enable_id_lists: opts.enable_id_lists,
        id_lists_url: opts.id_lists_url.clone(),
        id_lists_sync_interval_ms: opts.id_lists_sync_interval_ms,
        fallback_to_statsig_api: opts.fallback_to_statsig_api,
        environment: opts.environment.clone(),
        id_lists_adapter: None,
        override_adapter: None,
        output_log_level: opts
            .output_log_level
            .as_ref()
            .map(|level| LogLevel::from(level.as_str())),
        observability_client: ob_client_weak,
        service_name: None,
        enable_user_agent_parsing: opts.enable_user_agent_parsing,
        enable_country_lookup: opts.enable_country_lookup,
        global_custom_fields: opts
            .global_custom_fields
            .as_ref()
            .map(|dict| py_dict_to_map(dict.bind(py))),
        disable_network: opts.disable_network,
        persistent_storage: None,
        // persistent_storage: opts.persistent_storage.as_ref().map(|s| {
        //     Arc::new(StatsigPersistentStorageOverrideAdapter::new(
        //         s.clone_ref(py),
        //     )) as Arc<dyn PersistentStorage>
        // }),
    }
}

type ExtractObsClientResult = (
    Option<Weak<dyn ObservabilityClient>>,
    Option<Arc<dyn ObservabilityClient>>,
);

fn extract_observability_client(
    py: Python,
    ob_client: &Option<Py<ObservabilityClientBasePy>>,
) -> ExtractObsClientResult {
    let extracted = match ob_client {
        Some(ob_client) => ob_client.extract::<ObservabilityClientBasePy>(py),
        None => return (None, None),
    };

    let ob_client_strong: Option<Arc<dyn ObservabilityClient>>;
    let ob_client_weak: Option<Weak<dyn ObservabilityClient>>;

    match extracted {
        Ok(ob_client) => {
            let ob_client_arc: Arc<dyn ObservabilityClient> = Arc::new(ob_client);
            ob_client_weak = Some(Arc::downgrade(&ob_client_arc));
            ob_client_strong = Some(ob_client_arc);
        }
        Err(_) => return (None, None),
    }

    (ob_client_weak, ob_client_strong)
}
