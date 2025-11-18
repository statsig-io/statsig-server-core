use crate::output_logger_provider_base_py::OutputLoggerProviderBasePy;
use crate::statsig_persistent_storage_override_adapter_py::{
    PersistentStorageBasePy, StatsigPersistentStorageOverrideAdapter,
};
use crate::valid_primitives_py::ValidPrimitivesPy;
use crate::{
    data_store_base_py::DataStoreBasePy, observability_client_base_py::ObservabilityClientBasePy,
};
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen::derive::*;
use statsig_rust::data_store_interface::DataStoreTrait;
use statsig_rust::networking::proxy_config::ProxyConfig;
use statsig_rust::output_logger::OutputLogProvider;
use statsig_rust::statsig_options::DEFAULT_INIT_TIMEOUT_MS;
use statsig_rust::{log_w, ConfigCompressionMode, PersistentStorage, SpecAdapterConfig};
use statsig_rust::{output_logger::LogLevel, ObservabilityClient, StatsigOptions};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

const TAG: &str = stringify!(StatsigOptionsPy);

#[gen_stub_pyclass]
#[pyclass(name = "SpecAdapterConfig", module = "statsig_python_core")]
#[derive(Clone)]
pub struct SpecAdapterConfigPy {
    #[pyo3(get, set)]
    pub adapter_type: String,

    #[pyo3(get, set)]
    pub specs_url: Option<String>,

    #[pyo3(get, set)]
    pub init_timeout_ms: Option<u64>,
}

#[pymethods]
impl SpecAdapterConfigPy {
    #[new]
    #[pyo3(signature = (adapter_type, specs_url=None, init_timeout_ms=None))]
    pub fn new(
        adapter_type: String,
        specs_url: Option<String>,
        init_timeout_ms: Option<u64>,
    ) -> Self {
        Self {
            adapter_type,
            specs_url,
            init_timeout_ms,
        }
    }
}

impl From<SpecAdapterConfigPy> for SpecAdapterConfig {
    fn from(value: SpecAdapterConfigPy) -> Self {
        Self {
            adapter_type: value.adapter_type.into(),
            init_timeout_ms: value.init_timeout_ms.unwrap_or(DEFAULT_INIT_TIMEOUT_MS),
            specs_url: value.specs_url,
            authentication_mode: None,
            ca_cert_path: None,
            client_cert_path: None,
            domain_name: None,
            client_key_path: None,
        }
    }
}

#[gen_stub_pyclass]
#[pyclass(name = "ProxyConfig", module = "statsig_python_core")]
#[derive(Clone)]
pub struct ProxyConfigPy {
    #[pyo3(get, set)]
    pub proxy_host: Option<String>,
    #[pyo3(get, set)]
    pub proxy_port: Option<u16>,
    #[pyo3(get, set)]
    pub proxy_auth: Option<String>,
    #[pyo3(get, set)]
    pub proxy_protocol: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ProxyConfigPy {
    #[new]
    #[pyo3(signature = (proxy_host=None, proxy_port=None, proxy_auth=None, proxy_protocol=None))]
    fn new(
        proxy_host: Option<String>,
        proxy_port: Option<u16>,
        proxy_auth: Option<String>,
        proxy_protocol: Option<String>,
    ) -> Self {
        ProxyConfigPy {
            proxy_host,
            proxy_port,
            proxy_auth,
            proxy_protocol,
        }
    }
}

#[gen_stub_pyclass]
#[pyclass(name = "StatsigOptions", module = "statsig_python_core")]
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
    pub wait_for_user_agent_init: Option<bool>,
    #[pyo3(get, set)]
    pub wait_for_country_lookup_init: Option<bool>,
    #[pyo3(get, set)]
    pub disable_country_lookup: Option<bool>,
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
    pub global_custom_fields: Option<HashMap<String, ValidPrimitivesPy>>,
    #[pyo3(get, set)]
    pub observability_client: Option<Py<ObservabilityClientBasePy>>,
    #[pyo3(get, set)]
    pub output_logger_provider: Option<Py<OutputLoggerProviderBasePy>>,
    #[pyo3(get, set)]
    pub data_store: Option<Py<DataStoreBasePy>>,
    #[pyo3(get, set)]
    pub persistent_storage: Option<Py<PersistentStorageBasePy>>,
    #[pyo3(get, set)]
    pub config_compression_mode: Option<String>,
    #[pyo3(get, set)]
    pub proxy_config: Option<Py<ProxyConfigPy>>,
    #[pyo3(get, set)]
    pub spec_adapter_configs: Option<Py<PyList>>,
    #[pyo3(get, set)]
    pub use_third_party_ua_parser: Option<bool>,
    #[pyo3(get, set)]
    pub disable_disk_access: Option<bool>,
    #[pyo3(get, set)]
    pub experimental_flags: Option<HashSet<String>>,
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
        wait_for_user_agent_init=None,
        wait_for_country_lookup_init=None,
        disable_country_lookup=None,
        id_lists_url=None,
        id_lists_sync_interval_ms=None,
        fallback_to_statsig_api=None,
        environment=None,
        output_log_level=None,
        global_custom_fields=None,
        observability_client=None,
        data_store=None,
        persistent_storage=None,
        config_compression_mode=None,
        proxy_config=None,
        output_logger_provider=None,
        spec_adapter_configs=None,
        use_third_party_ua_parser=None,
        disable_disk_access=None,
        experimental_flags=None,
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
        wait_for_user_agent_init: Option<bool>,
        wait_for_country_lookup_init: Option<bool>,
        disable_country_lookup: Option<bool>,
        id_lists_url: Option<String>,
        id_lists_sync_interval_ms: Option<u32>,
        fallback_to_statsig_api: Option<bool>,
        environment: Option<String>,
        output_log_level: Option<String>,
        global_custom_fields: Option<HashMap<String, ValidPrimitivesPy>>,
        observability_client: Option<Py<ObservabilityClientBasePy>>,
        data_store: Option<Py<DataStoreBasePy>>,
        persistent_storage: Option<Py<PersistentStorageBasePy>>,
        config_compression_mode: Option<String>,
        proxy_config: Option<Py<ProxyConfigPy>>,
        output_logger_provider: Option<Py<OutputLoggerProviderBasePy>>,
        spec_adapter_configs: Option<Py<PyList>>,
        use_third_party_ua_parser: Option<bool>,
        disable_disk_access: Option<bool>,
        experimental_flags: Option<HashSet<String>>,
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
            wait_for_user_agent_init,
            wait_for_country_lookup_init,
            disable_country_lookup,
            id_lists_url,
            id_lists_sync_interval_ms,
            fallback_to_statsig_api,
            environment,
            output_log_level,
            global_custom_fields,
            observability_client,
            data_store,
            disable_network,
            persistent_storage,
            config_compression_mode,
            proxy_config,
            output_logger_provider,
            spec_adapter_configs,
            use_third_party_ua_parser,
            disable_disk_access,
            experimental_flags,
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

#[allow(deprecated)]
fn create_inner_statsig_options(
    py: Python,
    opts: StatsigOptionsPy,
    ob_client_weak: Option<Weak<dyn ObservabilityClient>>,
) -> StatsigOptions {
    let mut global_custom_fields = None;
    if let Some(fields) = opts.global_custom_fields {
        let converted = fields
            .into_iter()
            .map(|(k, v)| (k, v.into_dynamic_value()))
            .collect();

        global_custom_fields = Some(converted);
    }

    StatsigOptions {
        specs_url: opts.specs_url.clone(),
        specs_adapter: None,
        specs_sync_interval_ms: opts.specs_sync_interval_ms,
        init_timeout_ms: opts.init_timeout_ms,
        data_store: opts
            .data_store
            .and_then(|store| match store.extract::<DataStoreBasePy>(py) {
                Ok(store_unwrapped) => {
                    let store_actual = Arc::new(store_unwrapped) as Arc<dyn DataStoreTrait>;
                    Some(store_actual)
                }
                Err(_) => {
                    log_w!(TAG, "Failed to convert data store");
                    None
                }
            }),
        log_event_url: opts.log_event_url.clone(),
        disable_all_logging: opts.disable_all_logging,
        event_logging_adapter: None,
        event_logging_flush_interval_ms: None,
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
        wait_for_user_agent_init: opts.wait_for_user_agent_init,
        wait_for_country_lookup_init: opts.wait_for_user_agent_init,
        global_custom_fields,
        disable_network: opts.disable_network,
        disable_country_lookup: opts.disable_country_lookup,
        persistent_storage: opts.persistent_storage.as_ref().map(|s| {
            Arc::new(StatsigPersistentStorageOverrideAdapter::new(
                s.extract(py).unwrap_or_default(),
            )) as Arc<dyn PersistentStorage>
        }),
        config_compression_mode: opts
            .config_compression_mode
            .as_ref()
            .map(|mode| ConfigCompressionMode::from(mode.as_str())),
        proxy_config: opts.proxy_config.and_then(|py_val| {
            match py_val.extract::<ProxyConfigPy>(py) {
                Ok(cfg) => Some(ProxyConfig {
                    proxy_host: cfg.proxy_host,
                    proxy_port: cfg.proxy_port,
                    proxy_auth: cfg.proxy_auth,
                    proxy_protocol: cfg.proxy_protocol,
                }),
                Err(_) => {
                    log_w!(TAG, "Failed to convert proxy config");
                    None
                }
            }
        }),
        spec_adapters_config: opts.spec_adapter_configs.and_then(|configs| {
            match configs.extract::<Vec<SpecAdapterConfigPy>>(py) {
                Ok(configs) => Some(configs.into_iter().map(|config| config.into()).collect()),
                Err(_) => None,
            }
        }),
        output_logger_provider: opts.output_logger_provider.as_ref().map(|provider| {
            Arc::new(
                provider
                    .extract::<OutputLoggerProviderBasePy>(py)
                    .unwrap_or_default(),
            ) as Arc<dyn OutputLogProvider>
        }),
        console_capture_options: None,
        use_third_party_ua_parser: opts.use_third_party_ua_parser,
        disable_disk_access: opts.disable_disk_access,
        experimental_flags: opts.experimental_flags,
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
