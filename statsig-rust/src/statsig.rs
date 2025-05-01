use crate::evaluation::cmab_evaluator::{get_cmab_ranked_list, CMABRankedGroup};
use crate::evaluation::country_lookup::CountryLookup;
use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::{AnyEvaluation, BaseEvaluation, ExperimentEvaluation};
use crate::evaluation::evaluator::{Evaluator, SpecType};
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::evaluation::evaluator_result::{
    result_to_dynamic_config_eval, result_to_experiment_eval, result_to_gate_eval,
    result_to_layer_eval, EvaluatorResult,
};
use crate::evaluation::ua_parser::UserAgentParser;
use crate::event_logging::config_exposure::ConfigExposure;
use crate::event_logging::event_logger::{EventLogger, QueuedEventPayload};
use crate::event_logging::gate_exposure::GateExposure;
use crate::event_logging::layer_exposure::LayerExposure;
use crate::event_logging::statsig_event::StatsigEvent;
use crate::event_logging::statsig_event_internal::make_custom_event;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::event_logging_adapter::StatsigHttpEventLoggingAdapter;
use crate::gcir::gcir_formatter::GCIRFormatter;
use crate::hashing::HashUtil;
use crate::initialize_response::InitializeResponse;
use crate::observability::diagnostics_observer::DiagnosticsObserver;
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::{ErrorBoundaryEvent, SDKErrorsObserver};
use crate::output_logger::initialize_simple_output_logger;
use crate::persistent_storage::persistent_values_manager::PersistentValuesManager;
use crate::sdk_diagnostics::diagnostics::{ContextType, Diagnostics};
use crate::sdk_diagnostics::marker::{ActionType, KeyType, Marker};
use crate::spec_store::SpecStore;
use crate::specs_adapter::{StatsigCustomizedSpecsAdapter, StatsigHttpSpecsAdapter};
use crate::statsig_err::StatsigErr;
use crate::statsig_metadata::StatsigMetadata;
use crate::statsig_options::StatsigOptions;
use crate::statsig_runtime::StatsigRuntime;
use crate::statsig_type_factories::{
    make_dynamic_config, make_experiment, make_feature_gate, make_layer,
};
use crate::statsig_types::{DynamicConfig, Experiment, FeatureGate, Layer, ParameterStore};
use crate::user::StatsigUserInternal;
use crate::{
    dyn_value, log_d, log_e, log_w, read_lock_or_else, ClientInitResponseOptions,
    GCIRResponseFormat, IdListsAdapter, ObservabilityClient, OpsStatsEventObserver,
    OverrideAdapter, SamplingProcessor, SpecsAdapter, SpecsInfo, SpecsSource, SpecsUpdateListener,
    StatsigHttpIdListsAdapter, StatsigLocalOverrideAdapter, StatsigUser,
};
use crate::{
    log_error_to_statsig_and_console,
    statsig_core_api_options::{
        DynamicConfigEvaluationOptions, ExperimentEvaluationOptions, FeatureGateEvaluationOptions,
        LayerEvaluationOptions, ParameterStoreEvaluationOptions,
    },
};
use serde::de::DeserializeOwned;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::try_join;

const TAG: &str = stringify!(Statsig);
const ERROR_SDK_KEY: &str = "__STATSIG_ERROR_SDK_KEY__";
const INIT_IP_TAG: &str = "INIT_COUNTRY_LOOKUP";
const INIT_UA_TAG: &str = "INIT_UA";

lazy_static::lazy_static! {
    static ref SHARED_INSTANCE: Mutex<Option<Arc<Statsig>>> = Mutex::new(None);
}

pub struct Statsig {
    pub statsig_runtime: Arc<StatsigRuntime>,

    sdk_key: String,
    options: Arc<StatsigOptions>,
    event_logger: Arc<EventLogger>,
    specs_adapter: Arc<dyn SpecsAdapter>,
    event_logging_adapter: Arc<dyn EventLoggingAdapter>,
    id_lists_adapter: Option<Arc<dyn IdListsAdapter>>,
    override_adapter: Option<Arc<dyn OverrideAdapter>>,
    spec_store: Arc<SpecStore>,
    hashing: Arc<HashUtil>,
    gcir_formatter: Arc<GCIRFormatter>,
    statsig_environment: Option<HashMap<String, DynamicValue>>,
    fallback_environment: Mutex<Option<HashMap<String, DynamicValue>>>,
    ops_stats: Arc<OpsStatsForInstance>,
    error_observer: Arc<dyn OpsStatsEventObserver>,
    diagnostics_observer: Arc<dyn OpsStatsEventObserver>,
    sampling_processor: Arc<SamplingProcessor>,
    background_tasks_started: Arc<AtomicBool>,
    persistent_values_manager: Option<Arc<PersistentValuesManager>>,
    initialize_details: Mutex<InitializeDetails>,
}

pub struct StatsigContext {
    pub sdk_key: String,
    pub options: Arc<StatsigOptions>,
    pub local_override_adapter: Option<Arc<dyn OverrideAdapter>>,
    pub error_observer: Arc<dyn OpsStatsEventObserver>,
    pub diagnostics_observer: Arc<dyn OpsStatsEventObserver>,
    pub spec_store: Arc<SpecStore>,
}

#[derive(Debug, Clone)]
pub struct FailureDetails {
    pub reason: String,
    pub error: Option<StatsigErr>,
}

#[derive(Debug, Clone)]
pub struct InitializeDetails {
    pub duration: f64,
    pub init_success: bool,
    pub is_config_spec_ready: bool,
    pub is_id_list_ready: Option<bool>,
    pub source: SpecsSource,
    pub failure_details: Option<FailureDetails>,
}

impl Default for InitializeDetails {
    fn default() -> Self {
        InitializeDetails {
            duration: 0.0,
            init_success: false,
            is_config_spec_ready: false,
            is_id_list_ready: None,
            source: SpecsSource::Uninitialized,
            failure_details: None,
        }
    }
}

impl InitializeDetails {
    pub fn from_error(reason: &str, error: Option<StatsigErr>) -> Self {
        InitializeDetails {
            duration: 0.0,
            init_success: false,
            is_config_spec_ready: false,
            is_id_list_ready: None,
            source: SpecsSource::Uninitialized,
            failure_details: Some(FailureDetails {
                reason: reason.to_string(),
                error,
            }),
        }
    }
}

impl Statsig {
    pub fn new(sdk_key: &str, options: Option<Arc<StatsigOptions>>) -> Self {
        let statsig_runtime = StatsigRuntime::get_runtime();
        let options = options.unwrap_or_default();

        let hashing = Arc::new(HashUtil::new());

        let specs_adapter = initialize_specs_adapter(sdk_key, &options, &hashing);
        let id_lists_adapter = initialize_id_lists_adapter(sdk_key, &options);
        let event_logging_adapter = initialize_event_logging_adapter(sdk_key, &options);
        let override_adapter = match options.override_adapter.as_ref() {
            Some(adapter) => Some(Arc::clone(adapter)),
            None => Some(Arc::new(StatsigLocalOverrideAdapter::new()) as Arc<dyn OverrideAdapter>),
        };

        let event_logger = Arc::new(EventLogger::new(
            sdk_key,
            event_logging_adapter.clone(),
            &options,
            &statsig_runtime,
        ));

        let diagnostics = Arc::new(Diagnostics::new(event_logger.clone(), sdk_key));
        let diagnostics_observer: Arc<dyn OpsStatsEventObserver> =
            Arc::new(DiagnosticsObserver::new(diagnostics));
        let error_observer: Arc<dyn OpsStatsEventObserver> =
            Arc::new(SDKErrorsObserver::new(sdk_key, &options));

        let ops_stats = setup_ops_stats(
            sdk_key,
            &options,
            statsig_runtime.clone(),
            &error_observer,
            &diagnostics_observer,
            &options.observability_client,
        );

        let spec_store = Arc::new(SpecStore::new(
            sdk_key,
            hashing.sha256(sdk_key),
            statsig_runtime.clone(),
            options.data_store.clone(),
        ));

        let environment = options
            .environment
            .as_ref()
            .map(|env| HashMap::from([("tier".into(), dyn_value!(env.as_str()))]));

        let sampling_processor = Arc::new(SamplingProcessor::new(
            &statsig_runtime,
            hashing.clone(),
            sdk_key,
        ));

        let persistent_values_manager = options.persistent_storage.clone().map(|storage| {
            Arc::new(PersistentValuesManager {
                persistent_storage: storage,
            })
        });

        StatsigMetadata::update_service_name(options.service_name.clone());

        Statsig {
            sdk_key: sdk_key.to_string(),
            options,
            gcir_formatter: Arc::new(GCIRFormatter::new(&spec_store, &override_adapter)),
            event_logger,
            hashing,
            statsig_environment: environment,
            fallback_environment: Mutex::new(None),
            override_adapter,
            spec_store,
            specs_adapter,
            event_logging_adapter,
            id_lists_adapter,
            statsig_runtime,
            ops_stats,
            error_observer,
            sampling_processor,
            diagnostics_observer,
            background_tasks_started: Arc::new(AtomicBool::new(false)),
            persistent_values_manager,
            initialize_details: Mutex::new(InitializeDetails::default()),
        }
    }

    /***
     *  Initializes the Statsig client and returns an error if initialization fails.
     *
     *  This method performs the client initialization and returns `Ok(())` if successful.
     *  If the initialization completes with failure details, it returns a [`StatsigErr`]
     *  describing the failure.
     *
     *  For detailed information about the initialization process—regardless of success or failure—
     *  use [`initialize_with_details`] instead.
     *
     *  # Errors
     *
     *  Returns a [`StatsigErr`] if the client fails to initialize successfully.
     */
    pub async fn initialize(&self) -> Result<(), StatsigErr> {
        let details = self.initialize_with_details().await?;

        if let Some(failure_details) = details.failure_details {
            Err(failure_details
                .error
                .unwrap_or(StatsigErr::InitializationError(failure_details.reason)))
        } else {
            Ok(())
        }
    }

    /***
     *  Initializes the Statsig client and returns detailed information about the process.
     *
     *  This method returns a [`StatsigInitializeDetails`] struct, which includes metadata such as
     *  the success status, initialization source, and any failure details. Even if initialization
     *  fails, this method does not return an error; instead, the `init_success` field will be `false`
     *  and `failure_details` may be populated.
     ***/
    pub async fn initialize_with_details(&self) -> Result<InitializeDetails, StatsigErr> {
        self.ops_stats.add_marker(
            Marker::new(KeyType::Overall, ActionType::Start, None),
            Some(ContextType::Initialize),
        );

        let init_details = if let Some(timeout_ms) = self.options.init_timeout_ms {
            self.apply_timeout_to_init(timeout_ms).await
        } else {
            self.initialize_impl_with_details().await
        };
        self.log_init_details(&init_details);
        if let Ok(details) = &init_details {
            if let Ok(mut curr_init_details) = self.initialize_details.try_lock() {
                *curr_init_details = details.clone();
            }
        }
        init_details
    }

    pub fn get_initialize_details(&self) -> InitializeDetails {
        match self.initialize_details.lock() {
            Ok(details) => details.clone(),
            Err(poison_error) => InitializeDetails::from_error(
                "Failed to lock initialize_details",
                Some(StatsigErr::LockFailure(poison_error.to_string())),
            ),
        }
    }

    pub fn is_initialized(&self) -> bool {
        match self.initialize_details.lock() {
            Ok(details) => details.init_success,
            Err(_) => false,
        }
    }

    pub async fn shutdown(&self) -> Result<(), StatsigErr> {
        self.shutdown_with_timeout(Duration::from_secs(3)).await
    }

    pub async fn shutdown_with_timeout(&self, timeout: Duration) -> Result<(), StatsigErr> {
        log_d!(
            TAG,
            "Shutting down Statsig with timeout {}ms",
            timeout.as_millis()
        );

        let start = Instant::now();
        let shutdown_result = tokio::select! {
            () = tokio::time::sleep(timeout) => {
                log_w!(TAG, "Statsig shutdown timed out. {}", start.elapsed().as_millis());
                Err(StatsigErr::ShutdownFailure(
                    "Shutdown timed out".to_string()
                ))
            }
            sub_result = async {
                let id_list_shutdown: Pin<Box<_>> = if let Some(adapter) = &self.id_lists_adapter {
                    adapter.shutdown(timeout)
                } else {
                    Box::pin(async { Ok(()) })
                };

                try_join!(
                    id_list_shutdown,
                    self.event_logger.shutdown(timeout),
                    self.specs_adapter.shutdown(timeout, &self.statsig_runtime),
                )
            } => {
                match sub_result {
                    Ok(_) => {
                        log_d!(TAG, "All Statsig tasks shutdown successfully");
                        Ok(())
                    }
                    Err(e) => {
                        log_w!(TAG, "Error during shutdown: {:?}", e);
                        Err(e)
                    }
                }
            }
        };

        self.statsig_runtime.shutdown();
        shutdown_result
    }

    async fn start_background_tasks(
        event_logger: Arc<EventLogger>,
        statsig_runtime: Arc<StatsigRuntime>,
        id_lists_adapter: Option<Arc<dyn IdListsAdapter>>,
        specs_adapter: Arc<dyn SpecsAdapter>,
        ops_stats: Arc<OpsStatsForInstance>,
        bg_tasks_started: Arc<AtomicBool>,
    ) -> bool {
        if bg_tasks_started.load(Ordering::SeqCst) {
            return true;
        }

        let mut success = true;
        event_logger.clone().start_background_task(&statsig_runtime);

        if let Some(adapter) = &id_lists_adapter {
            if let Err(e) = adapter
                .clone()
                .schedule_background_sync(&statsig_runtime)
                .await
            {
                success = false;
                log_w!(TAG, "Failed to schedule idlist background job {}", e);
            }
        }

        if let Err(e) = specs_adapter
            .clone()
            .schedule_background_sync(&statsig_runtime)
            .await
        {
            success = false;
            log_error_to_statsig_and_console!(
                ops_stats,
                TAG,
                StatsigErr::SpecsAdapterSkipPoll(format!(
                    "Failed to schedule specs adapter background job: {}",
                    e
                ))
            );
        }

        bg_tasks_started.store(true, Ordering::SeqCst);

        success
    }

    async fn apply_timeout_to_init(
        &self,
        timeout_ms: u64,
    ) -> Result<InitializeDetails, StatsigErr> {
        let timeout = Duration::from_millis(timeout_ms);

        let init_future = self.initialize_impl_with_details();
        let timeout_future = sleep(timeout);

        let event_logger = self.event_logger.clone();
        let statsig_runtime = self.statsig_runtime.clone();
        let id_lists_adapter = self.id_lists_adapter.clone();
        let specs_adapter = self.specs_adapter.clone();
        let ops_stats = self.ops_stats.clone();
        let background_tasks_started = self.background_tasks_started.clone();
        // Create another clone specifically for the closure
        let statsig_runtime_for_closure = statsig_runtime.clone();

        tokio::select! {
            result = init_future => {
                result
            },
            _ = timeout_future => {
                statsig_runtime.spawn(
                    "start_background_tasks",
                    |_shutdown_notify| async move {
                        Self::start_background_tasks(
                            event_logger,
                            statsig_runtime_for_closure,
                            id_lists_adapter,
                            specs_adapter,
                            ops_stats,
                            background_tasks_started,
                        ).await;
                    }
                );
                Ok(self.timeout_failure(timeout_ms))
            },
        }
    }

    async fn initialize_impl_with_details(&self) -> Result<InitializeDetails, StatsigErr> {
        let start_time = Instant::now();
        self.spec_store.set_source(SpecsSource::Loading);
        self.specs_adapter.initialize(self.spec_store.clone());

        let mut error_message = None;
        let mut id_list_ready = None;

        let init_country_lookup = if !self.options.disable_country_lookup.unwrap_or_default() {
            Some(self.statsig_runtime.spawn(INIT_IP_TAG, |_| async {
                CountryLookup::load_country_lookup();
            }))
        } else {
            None
        };

        let init_ua = if !self.options.disable_user_agent_parsing.unwrap_or_default() {
            Some(self.statsig_runtime.spawn(INIT_UA_TAG, |_| async {
                UserAgentParser::load_parser();
            }))
        } else {
            None
        };

        let init_res = match self
            .specs_adapter
            .clone()
            .start(&self.statsig_runtime)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                self.spec_store.set_source(SpecsSource::NoValues);
                error_message = Some(format!("Failed to start specs adapter: {e}"));
                Err(e)
            }
        };

        if let Some(adapter) = &self.id_lists_adapter {
            match adapter
                .clone()
                .start(&self.statsig_runtime, self.spec_store.clone())
                .await
            {
                Ok(()) => {
                    id_list_ready = Some(true);
                }
                Err(e) => {
                    id_list_ready = Some(false);
                    error_message.get_or_insert_with(|| format!("Failed to sync ID lists: {e}"));
                }
            }
            if let Err(e) = adapter
                .clone()
                .schedule_background_sync(&self.statsig_runtime)
                .await
            {
                log_w!(TAG, "Failed to schedule id_list background job {}", e);
            }
        }

        if let Err(e) = self
            .event_logging_adapter
            .clone()
            .start(&self.statsig_runtime)
            .await
        {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::UnstartedAdapter(format!(
                    "Failed to start event logging adapter: {}",
                    e
                ))
            );
        }

        let spec_info = self.spec_store.get_current_specs_info();
        let duration = start_time.elapsed().as_millis() as f64;

        self.set_default_environment_from_server();

        if self.options.wait_for_country_lookup_init.unwrap_or(false) {
            if let Some(task_id) = init_country_lookup {
                let _ = self
                    .statsig_runtime
                    .await_join_handle(INIT_IP_TAG, &task_id)
                    .await;
            }
        }
        if self.options.wait_for_user_agent_init.unwrap_or(false) {
            if let Some(task_id) = init_ua {
                let _ = self
                    .statsig_runtime
                    .await_join_handle(INIT_UA_TAG, &task_id)
                    .await;
            };
        }

        let error = init_res.clone().err();

        let success = Self::start_background_tasks(
            self.event_logger.clone(),
            self.statsig_runtime.clone(),
            self.id_lists_adapter.clone(),
            self.specs_adapter.clone(),
            self.ops_stats.clone(),
            self.background_tasks_started.clone(),
        )
        .await;

        Ok(InitializeDetails {
            init_success: success,
            is_config_spec_ready: matches!(spec_info.lcut, Some(v) if v != 0),
            is_id_list_ready: id_list_ready,
            source: spec_info.source,
            failure_details: error.as_ref().map(|e| FailureDetails {
                reason: e.to_string(),
                error: Some(e.clone()),
            }),
            duration,
        })
    }

    fn timeout_failure(&self, timeout_ms: u64) -> InitializeDetails {
        InitializeDetails {
            init_success: false,
            is_config_spec_ready: false,
            is_id_list_ready: None,
            source: SpecsSource::Uninitialized,
            failure_details: Some(FailureDetails {
                reason: "Initialization timed out".to_string(),
                error: None,
            }),
            duration: timeout_ms as f64,
        }
    }

    fn log_init_details(&self, init_details: &Result<InitializeDetails, StatsigErr>) {
        match init_details {
            Ok(details) => {
                self.log_init_finish(
                    details.init_success,
                    &None,
                    &details.duration,
                    &self.spec_store.get_current_specs_info(),
                );
                if let Some(failure) = &details.failure_details {
                    log_error_to_statsig_and_console!(
                        self.ops_stats,
                        TAG,
                        StatsigErr::InitializationError(failure.reason.clone())
                    );
                }
            }
            Err(err) => {
                log_w!(TAG, "Initialization error: {:?}", err);
            }
        }
    }

    pub fn get_context(&self) -> StatsigContext {
        StatsigContext {
            sdk_key: self.sdk_key.clone(),
            options: self.options.clone(),
            local_override_adapter: self.override_adapter.clone(),
            error_observer: self.error_observer.clone(),
            diagnostics_observer: self.diagnostics_observer.clone(),
            spec_store: self.spec_store.clone(),
        }
    }

    pub fn log_event(
        &self,
        user: &StatsigUser,
        event_name: &str,
        value: Option<String>,
        metadata: Option<HashMap<String, String>>,
    ) {
        let user_internal = self.internalize_user(user);

        self.event_logger
            .enqueue(QueuedEventPayload::CustomEvent(make_custom_event(
                user_internal.to_loggable(),
                StatsigEvent {
                    event_name: event_name.to_string(),
                    value: value.map(|v| json!(v)),
                    metadata,
                    statsig_metadata: None,
                },
            )));
    }

    pub fn log_event_with_number(
        &self,
        user: &StatsigUser,
        event_name: &str,
        value: Option<f64>,
        metadata: Option<HashMap<String, String>>,
    ) {
        let user_internal = self.internalize_user(user);

        self.event_logger
            .enqueue(QueuedEventPayload::CustomEvent(make_custom_event(
                user_internal.to_loggable(),
                StatsigEvent {
                    event_name: event_name.to_string(),
                    value: value.map(|v| json!(v)),
                    metadata,
                    statsig_metadata: None,
                },
            )));
    }

    pub fn log_layer_param_exposure_with_layer_json(
        &self,
        layer_json: String,
        parameter_name: String,
    ) {
        let layer = match serde_json::from_str::<Layer>(&layer_json) {
            Ok(layer) => layer,
            Err(e) => {
                log_error_to_statsig_and_console!(
                    self.ops_stats.clone(),
                    TAG,
                    StatsigErr::ShutdownFailure(e.to_string())
                );
                return;
            }
        };

        self.log_layer_param_exposure_with_layer(layer, parameter_name);
    }

    pub fn log_layer_param_exposure_with_layer(&self, layer: Layer, parameter_name: String) {
        if layer.__disable_exposure {
            self.event_logger
                .increment_non_exposure_checks_count(layer.name.clone());
            return;
        }

        let layer_eval = layer.__evaluation.as_ref();

        let sampling_details = self.sampling_processor.get_sampling_decision_and_details(
            &layer.__user.get_sampling_key(),
            layer_eval.map(AnyEvaluation::from).as_ref(),
            Some(&parameter_name),
        );

        if !sampling_details.should_send_exposure {
            return;
        }

        self.event_logger
            .enqueue(QueuedEventPayload::LayerExposure(LayerExposure {
                user: layer.__user,
                parameter_name,
                evaluation: layer.__evaluation,
                layer_name: layer.name,
                evaluation_details: layer.details,
                version: layer.__version,
                is_manual_exposure: false,
                sampling_details,
                override_config_name: layer.__override_config_name.clone(),
            }));
    }

    pub async fn flush_events(&self) {
        self.event_logger.flush_all_blocking().await;
    }

    pub fn get_client_init_response(&self, user: &StatsigUser) -> InitializeResponse {
        self.get_client_init_response_with_options(user, self.gcir_formatter.get_default_options())
    }

    pub fn get_client_init_response_with_options(
        &self,
        user: &StatsigUser,
        options: &ClientInitResponseOptions,
    ) -> InitializeResponse {
        let user_internal = self.internalize_user(user);
        self.gcir_formatter
            .get_as_v1_format(user_internal, &self.hashing, options)
    }

    pub fn get_client_init_response_as_string(&self, user: &StatsigUser) -> String {
        serde_json::to_string(&self.get_client_init_response(user)).unwrap_or_default()
    }

    pub fn get_client_init_response_with_options_as_string(
        &self,
        user: &StatsigUser,
        options: &ClientInitResponseOptions,
    ) -> String {
        let user_internal = self.internalize_user(user);
        let response = match options.response_format {
            Some(GCIRResponseFormat::InitializeWithSecondaryExposureMapping) => {
                json!(self
                    .gcir_formatter
                    .get_as_v2_format(user_internal, &self.hashing, options))
            }
            _ => json!(self
                .gcir_formatter
                .get_as_v1_format(user_internal, &self.hashing, options)),
        };

        json!(response).to_string()
    }

    pub fn get_string_parameter_from_store(
        &self,
        user: &StatsigUser,
        parameter_store_name: &str,
        parameter_name: &str,
        fallback: Option<String>,
    ) -> Option<String> {
        self.get_parameter_from_store(user, parameter_store_name, parameter_name, fallback)
    }

    pub fn get_boolean_parameter_from_store(
        &self,
        user: &StatsigUser,
        parameter_store_name: &str,
        parameter_name: &str,
        fallback: Option<bool>,
    ) -> Option<bool> {
        self.get_parameter_from_store(user, parameter_store_name, parameter_name, fallback)
    }

    pub fn get_float_parameter_from_store(
        &self,
        user: &StatsigUser,
        parameter_store_name: &str,
        parameter_name: &str,
        fallback: Option<f64>,
    ) -> Option<f64> {
        self.get_parameter_from_store(user, parameter_store_name, parameter_name, fallback)
    }

    pub fn get_integer_parameter_from_store(
        &self,
        user: &StatsigUser,
        parameter_store_name: &str,
        parameter_name: &str,
        fallback: Option<i64>,
    ) -> Option<i64> {
        self.get_parameter_from_store(user, parameter_store_name, parameter_name, fallback)
    }

    pub fn get_array_parameter_from_store(
        &self,
        user: &StatsigUser,
        parameter_store_name: &str,
        parameter_name: &str,
        fallback: Option<Vec<Value>>,
    ) -> Option<Vec<Value>> {
        self.get_parameter_from_store(user, parameter_store_name, parameter_name, fallback)
    }

    pub fn get_object_parameter_from_store(
        &self,
        user: &StatsigUser,
        parameter_store_name: &str,
        parameter_name: &str,
        fallback: Option<HashMap<String, Value>>,
    ) -> Option<HashMap<String, Value>> {
        self.get_parameter_from_store(user, parameter_store_name, parameter_name, fallback)
    }

    pub fn get_parameter_from_store<T: DeserializeOwned>(
        &self,
        user: &StatsigUser,
        parameter_store_name: &str,
        parameter_name: &str,
        fallback: Option<T>,
    ) -> Option<T> {
        let store = self.get_parameter_store(parameter_store_name);
        match fallback {
            Some(fallback) => Some(store.get(user, parameter_name, fallback)),
            None => store.get_opt(user, parameter_name),
        }
    }

    pub fn get_parameter_store(&self, parameter_store_name: &str) -> ParameterStore {
        self.get_parameter_store_with_options(
            parameter_store_name,
            ParameterStoreEvaluationOptions::default(),
        )
    }

    pub fn get_parameter_store_with_options(
        &self,
        parameter_store_name: &str,
        options: ParameterStoreEvaluationOptions,
    ) -> ParameterStore {
        self.event_logger
            .increment_non_exposure_checks_count(parameter_store_name.to_string());
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return ParameterStore {
                name: parameter_store_name.to_string(),
                parameters: HashMap::new(),
                details: EvaluationDetails::unrecognized_no_data(),
                options,
                _statsig_ref: self,
            };
        });

        let stores = &data.values.param_stores;
        let store = match stores {
            Some(stores) => stores.get(parameter_store_name),
            None => {
                return ParameterStore {
                    name: parameter_store_name.to_string(),
                    parameters: HashMap::new(),
                    details: EvaluationDetails::unrecognized(&data),
                    options,
                    _statsig_ref: self,
                };
            }
        };
        match store {
            Some(store) => ParameterStore {
                name: parameter_store_name.to_string(),
                parameters: store.parameters.clone(),
                details: EvaluationDetails::recognized(&data, &EvaluatorResult::default()),
                options,
                _statsig_ref: self,
            },
            None => ParameterStore {
                name: parameter_store_name.to_string(),
                parameters: HashMap::new(),
                details: EvaluationDetails::unrecognized(&data),
                options,
                _statsig_ref: self,
            },
        }
    }
}

// -------------------------
//   User Store Functions
// -------------------------
impl Statsig {
    pub fn identify(&self, user: &StatsigUser) {
        let user_internal = self.internalize_user(user);

        self.event_logger
            .enqueue(QueuedEventPayload::CustomEvent(make_custom_event(
                user_internal.to_loggable(),
                StatsigEvent {
                    event_name: "statsig::identify".to_string(),
                    value: None,
                    metadata: None,
                    statsig_metadata: None,
                },
            )));
    }
}

// -------------------------
//   CMAB Functions
// -------------------------

impl Statsig {
    pub fn get_cmab_ranked_groups(
        &self,
        user: &StatsigUser,
        cmab_name: &str,
    ) -> Vec<CMABRankedGroup> {
        self.event_logger
            .increment_non_exposure_checks_count(cmab_name.to_string());
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });
        let user_internal = self.internalize_user(user);
        get_cmab_ranked_list(
            &mut EvaluatorContext::new(
                &user_internal,
                &data,
                &self.hashing,
                data.values.app_id.as_ref(),
                self.override_adapter.as_ref(),
            ),
            cmab_name,
        )
    }

    pub fn log_cmab_exposure_for_group(
        &self,
        user: &StatsigUser,
        cmab_name: &str,
        group_id: String,
    ) {
        let user_internal = self.internalize_user(user);
        let experiment = self.get_experiment_impl(&user_internal, cmab_name);
        let sampling_info = match experiment.__evaluation {
            Some(ref eval) => eval.base.sampling_info.clone(),
            None => None,
        };
        let base_eval = BaseEvaluation {
            name: cmab_name.to_string(),
            rule_id: group_id.clone(),
            secondary_exposures: match experiment.__evaluation {
                Some(ref eval) => eval.base.secondary_exposures.clone(),
                None => vec![],
            },
            sampling_info,
        };
        let experiment_eval = ExperimentEvaluation {
            base: base_eval.clone(),
            id_type: experiment.id_type.clone(),
            value: HashMap::new(),
            group: group_id,
            is_device_based: false,
            is_in_layer: false,
            explicit_parameters: None,
            group_name: None,
            is_experiment_active: Some(true),
            is_user_in_experiment: Some(true),
            undelegated_secondary_exposures: None,
        };

        let sampling_details = self.sampling_processor.get_sampling_decision_and_details(
            &user_internal.get_sampling_key(),
            Some(&AnyEvaluation::from(&experiment_eval)),
            None,
        );

        if !sampling_details.should_send_exposure {
            return;
        }

        self.event_logger
            .enqueue(QueuedEventPayload::ConfigExposure(ConfigExposure {
                user: user_internal.to_loggable(),
                evaluation: Some(base_eval),
                evaluation_details: experiment.details.clone(),
                config_name: cmab_name.to_string(),
                rule_passed: None,
                version: experiment.__version,
                is_manual_exposure: true,
                sampling_details,
                override_config_name: experiment.__override_config_name.clone(),
            }));
    }
}

// -------------------------
//   Shared Instance Functions
// -------------------------

impl Statsig {
    pub fn shared() -> Arc<Statsig> {
        let lock = match SHARED_INSTANCE.lock() {
            Ok(lock) => lock,
            Err(e) => {
                log_e!(TAG, "Statsig::shared() mutex error: {}", e);
                return Arc::new(Statsig::new(ERROR_SDK_KEY, None));
            }
        };

        match lock.as_ref() {
            Some(statsig) => statsig.clone(),
            None => {
                log_e!(
                    TAG,
                    "Statsig::shared() called, but no instance has been set with Statsig::new_shared(...)"
                );
                Arc::new(Statsig::new(ERROR_SDK_KEY, None))
            }
        }
    }

    pub fn new_shared(
        sdk_key: &str,
        options: Option<Arc<StatsigOptions>>,
    ) -> Result<Arc<Statsig>, StatsigErr> {
        match SHARED_INSTANCE.lock() {
            Ok(mut lock) => {
                if lock.is_some() {
                    let message = "Statsig shared instance already exists. Call Statsig::remove_shared() before creating a new instance.";
                    log_e!(TAG, "{}", message);
                    return Err(StatsigErr::SharedInstanceFailure(message.to_string()));
                }

                let statsig = Arc::new(Statsig::new(sdk_key, options));
                *lock = Some(statsig.clone());
                Ok(statsig)
            }
            Err(e) => {
                let message = format!("Statsig::new_shared() mutex error: {}", e);
                log_e!(TAG, "{}", message);
                Err(StatsigErr::SharedInstanceFailure(message))
            }
        }
    }

    pub fn remove_shared() {
        match SHARED_INSTANCE.lock() {
            Ok(mut lock) => {
                *lock = None;
            }
            Err(e) => {
                log_e!(TAG, "Statsig::remove_shared() mutex error: {}", e);
            }
        }
    }

    pub fn has_shared_instance() -> bool {
        match SHARED_INSTANCE.lock() {
            Ok(lock) => lock.is_some(),
            Err(_) => false,
        }
    }
}

// -------------------------
//   Feature Gate Functions
// -------------------------

impl Statsig {
    pub fn check_gate(&self, user: &StatsigUser, gate_name: &str) -> bool {
        self.check_gate_with_options(user, gate_name, FeatureGateEvaluationOptions::default())
    }

    pub fn check_gate_with_options(
        &self,
        user: &StatsigUser,
        gate_name: &str,
        options: FeatureGateEvaluationOptions,
    ) -> bool {
        self.get_feature_gate_with_options(user, gate_name, options)
            .value
    }

    pub fn get_feature_gate(&self, user: &StatsigUser, gate_name: &str) -> FeatureGate {
        self.get_feature_gate_with_options(user, gate_name, FeatureGateEvaluationOptions::default())
    }

    pub fn get_feature_gate_with_options(
        &self,
        user: &StatsigUser,
        gate_name: &str,
        options: FeatureGateEvaluationOptions,
    ) -> FeatureGate {
        let user_internal = self.internalize_user(user);
        let disable_exposure_logging = options.disable_exposure_logging;
        let gate = self.get_feature_gate_impl(&user_internal, gate_name);

        if disable_exposure_logging {
            log_d!(TAG, "Exposure logging is disabled for gate {}", gate_name);
            self.event_logger
                .increment_non_exposure_checks_count(gate_name.to_string());
        } else {
            self.log_gate_exposure(user_internal, gate_name, &gate, false);
        }

        gate
    }

    pub fn manually_log_gate_exposure(&self, user: &StatsigUser, gate_name: &str) {
        let user_internal = self.internalize_user(user);
        let gate = self.get_feature_gate_impl(&user_internal, gate_name);
        self.log_gate_exposure(user_internal, gate_name, &gate, true);
    }

    pub fn get_fields_needed_for_gate(&self, gate_name: &str) -> Vec<String> {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        let gate = data.values.feature_gates.get(gate_name);
        match gate {
            Some(gate) => match &gate.fields_used {
                Some(fields) => fields.clone(),
                None => vec![],
            },
            None => vec![],
        }
    }
}

// -------------------------
//   Override Functions
// -------------------------

impl Statsig {
    pub fn override_gate(&self, gate_name: &str, value: bool, id: Option<&str>) {
        if let Some(adapter) = &self.override_adapter {
            adapter.override_gate(gate_name, value, id);
        }
    }

    pub fn override_dynamic_config(
        &self,
        config_name: &str,
        value: HashMap<String, serde_json::Value>,
        id: Option<&str>,
    ) {
        if let Some(adapter) = &self.override_adapter {
            adapter.override_dynamic_config(config_name, value, id);
        }
    }

    pub fn override_layer(
        &self,
        layer_name: &str,
        value: HashMap<String, serde_json::Value>,
        id: Option<&str>,
    ) {
        if let Some(adapter) = &self.override_adapter {
            adapter.override_layer(layer_name, value, id);
        }
    }

    pub fn override_experiment(
        &self,
        experiment_name: &str,
        value: HashMap<String, serde_json::Value>,
        id: Option<&str>,
    ) {
        if let Some(adapter) = &self.override_adapter {
            adapter.override_experiment(experiment_name, value, id);
        }
    }

    pub fn override_experiment_by_group_name(
        &self,
        experiment_name: &str,
        group_name: &str,
        id: Option<&str>,
    ) {
        if let Some(adapter) = &self.override_adapter {
            adapter.override_experiment_by_group_name(experiment_name, group_name, id);
        }
    }

    pub fn remove_gate_override(&self, gate_name: &str, id: Option<&str>) {
        if let Some(adapter) = &self.override_adapter {
            adapter.remove_gate_override(gate_name, id);
        }
    }

    pub fn remove_dynamic_config_override(&self, config_name: &str, id: Option<&str>) {
        if let Some(adapter) = &self.override_adapter {
            adapter.remove_dynamic_config_override(config_name, id);
        }
    }

    pub fn remove_experiment_override(&self, experiment_name: &str, id: Option<&str>) {
        if let Some(adapter) = &self.override_adapter {
            adapter.remove_experiment_override(experiment_name, id);
        }
    }

    pub fn remove_layer_override(&self, layer_name: &str, id: Option<&str>) {
        if let Some(adapter) = &self.override_adapter {
            adapter.remove_layer_override(layer_name, id);
        }
    }

    pub fn remove_all_overrides(&self) {
        if let Some(adapter) = &self.override_adapter {
            adapter.remove_all_overrides();
        }
    }
}

// -------------------------
//   Debugging Functions
// -------------------------

impl Statsig {
    pub fn get_feature_gate_list(&self) -> Vec<String> {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        data.values.feature_gates.keys().cloned().collect()
    }

    pub fn get_dynamic_config_list(&self) -> Vec<String> {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        data.values
            .dynamic_configs
            .iter()
            .filter(|(_, config)| config.entity == "dynamic_config")
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn get_experiment_list(&self) -> Vec<String> {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        data.values
            .dynamic_configs
            .iter()
            .filter(|(_, config)| config.entity == "experiment")
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn get_parameter_store_list(&self) -> Vec<String> {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        match &data.values.param_stores {
            Some(param_stores) => param_stores.keys().cloned().collect(),
            None => vec![],
        }
    }
}

// -------------------------
//   Dynamic Config Functions
// -------------------------

impl Statsig {
    pub fn get_dynamic_config(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
    ) -> DynamicConfig {
        self.get_dynamic_config_with_options(
            user,
            dynamic_config_name,
            DynamicConfigEvaluationOptions::default(),
        )
    }

    pub fn get_dynamic_config_with_options(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
        options: DynamicConfigEvaluationOptions,
    ) -> DynamicConfig {
        let user_internal = self.internalize_user(user);
        let disable_exposure_logging = options.disable_exposure_logging;
        let config = self.get_dynamic_config_impl(&user_internal, dynamic_config_name);

        if disable_exposure_logging {
            log_d!(
                TAG,
                "Exposure logging is disabled for Dynamic Config {}",
                dynamic_config_name
            );
            self.event_logger
                .increment_non_exposure_checks_count(dynamic_config_name.to_string());
        } else {
            self.log_dynamic_config_exposure(user_internal, dynamic_config_name, &config, false);
        }

        config
    }

    pub fn manually_log_dynamic_config_exposure(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
    ) {
        let user_internal = self.internalize_user(user);
        let dynamic_config = self.get_dynamic_config_impl(&user_internal, dynamic_config_name);
        self.log_dynamic_config_exposure(user_internal, dynamic_config_name, &dynamic_config, true);
    }

    pub fn get_fields_needed_for_dynamic_config(&self, config_name: &str) -> Vec<String> {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        let config = data.values.dynamic_configs.get(config_name);
        match config {
            Some(config) => match &config.fields_used {
                Some(fields) => fields.clone(),
                None => vec![],
            },
            None => vec![],
        }
    }
}

// -------------------------
//   Experiment Functions
// -------------------------

impl Statsig {
    pub fn get_experiment(&self, user: &StatsigUser, experiment_name: &str) -> Experiment {
        self.get_experiment_with_options(
            user,
            experiment_name,
            ExperimentEvaluationOptions::default(),
        )
    }

    pub fn get_experiment_with_options(
        &self,
        user: &StatsigUser,
        experiment_name: &str,
        options: ExperimentEvaluationOptions,
    ) -> Experiment {
        let user_internal = self.internalize_user(user);
        let disable_exposure_logging = options.disable_exposure_logging;
        let mut experiment = self.get_experiment_impl(&user_internal, experiment_name);
        if let Some(persisted_experiment) = self.persistent_values_manager.as_ref().and_then(|m| {
            m.try_apply_sticky_value_to_experiment(&user_internal, &options, &experiment)
        }) {
            experiment = persisted_experiment
        }

        if disable_exposure_logging {
            log_d!(
                TAG,
                "Exposure logging is disabled for experiment {}",
                experiment_name
            );
            self.event_logger
                .increment_non_exposure_checks_count(experiment_name.to_string());
        } else {
            self.log_experiment_exposure(user_internal, experiment_name, &experiment, false);
        }

        experiment
    }

    pub fn manually_log_experiment_exposure(&self, user: &StatsigUser, experiment_name: &str) {
        let user_internal = self.internalize_user(user);
        let experiment = self.get_experiment_impl(&user_internal, experiment_name);
        self.log_experiment_exposure(user_internal, experiment_name, &experiment, true);
    }

    pub fn get_fields_needed_for_experiment(&self, experiment_name: &str) -> Vec<String> {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        let config = data.values.dynamic_configs.get(experiment_name);
        match config {
            Some(config) => match &config.fields_used {
                Some(fields) => fields.clone(),
                None => vec![],
            },
            None => vec![],
        }
    }
}

// -------------------------
//   Layer Functions
// -------------------------

impl Statsig {
    pub fn get_layer(&self, user: &StatsigUser, layer_name: &str) -> Layer {
        self.get_layer_with_options(user, layer_name, LayerEvaluationOptions::default())
    }

    pub fn get_layer_with_options(
        &self,
        user: &StatsigUser,
        layer_name: &str,
        options: LayerEvaluationOptions,
    ) -> Layer {
        let user_internal = self.internalize_user(user);
        self.get_layer_impl(user_internal, layer_name, options)
    }

    pub fn manually_log_layer_parameter_exposure(
        &self,
        user: &StatsigUser,
        layer_name: &str,
        parameter_name: String,
    ) {
        let user_internal = self.internalize_user(user);
        let layer =
            self.get_layer_impl(user_internal, layer_name, LayerEvaluationOptions::default());

        let layer_eval = layer.__evaluation.as_ref();

        let sampling_details = self.sampling_processor.get_sampling_decision_and_details(
            &layer.__user.get_sampling_key(),
            layer_eval.map(AnyEvaluation::from).as_ref(),
            Some(parameter_name.as_str()),
        );

        if !sampling_details.should_send_exposure {
            return;
        }

        self.event_logger
            .enqueue(QueuedEventPayload::LayerExposure(LayerExposure {
                user: layer.__user,
                parameter_name,
                evaluation: layer.__evaluation,
                layer_name: layer.name,
                evaluation_details: layer.details,
                version: layer.__version,
                is_manual_exposure: true,
                sampling_details,
                override_config_name: layer.__override_config_name.clone(),
            }));
    }

    pub fn get_fields_needed_for_layer(&self, layer_name: &str) -> Vec<String> {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return vec![];
        });

        let layer = data.values.layer_configs.get(layer_name);
        match layer {
            Some(layer) => match &layer.fields_used {
                Some(fields) => fields.clone(),
                None => vec![],
            },
            None => vec![],
        }
    }
}

// -------------------------
//   Internal Functions
// -------------------------

impl Statsig {
    pub(crate) fn get_from_statsig_env(&self, key: &str) -> Option<DynamicValue> {
        if let Some(env) = &self.statsig_environment {
            return env.get(key).cloned();
        }

        if let Ok(fallback_env) = self.fallback_environment.lock() {
            if let Some(env) = &*fallback_env {
                return env.get(key).cloned();
            }
        }

        None
    }

    pub(crate) fn get_value_from_global_custom_fields(&self, key: &str) -> Option<&DynamicValue> {
        if let Some(env) = &self.options.global_custom_fields {
            return env.get(key);
        }

        None
    }

    pub(crate) fn use_global_custom_fields<T>(
        &self,
        f: impl FnOnce(Option<&HashMap<String, DynamicValue>>) -> Result<(), T>,
    ) -> Result<(), T> {
        f(self.options.global_custom_fields.as_ref())
    }

    pub(crate) fn use_statsig_env<T>(
        &self,
        f: impl FnOnce(Option<&HashMap<String, DynamicValue>>) -> T,
    ) -> T {
        if let Some(env) = &self.statsig_environment {
            return f(Some(env));
        }

        if let Ok(fallback_env) = self.fallback_environment.lock() {
            if let Some(env) = &*fallback_env {
                return f(Some(env));
            }
        }

        f(None)
    }
}

// -------------------------
//   Private Functions
// -------------------------

impl Statsig {
    fn evaluate_spec<T>(
        &self,
        user_internal: &StatsigUserInternal,
        spec_name: &str,
        make_empty_result: impl FnOnce(EvaluationDetails) -> T,
        make_result: impl FnOnce(EvaluatorResult, EvaluationDetails) -> T,
        spec_type: &SpecType,
    ) -> T {
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                &self.ops_stats,
                TAG,
                StatsigErr::LockFailure(
                    "Failed to acquire read lock for spec store data".to_string()
                )
            );
            return make_empty_result(EvaluationDetails::unrecognized_no_data());
        });
        let app_id = data.values.app_id.as_ref();
        let mut context = EvaluatorContext::new(
            user_internal,
            &data,
            &self.hashing,
            app_id,
            self.override_adapter.as_ref(),
        );

        match Evaluator::evaluate_with_details(&mut context, spec_name, spec_type) {
            Ok(eval_details) => make_result(context.result, eval_details),
            Err(e) => {
                log_error_to_statsig_and_console!(
                    &self.ops_stats,
                    TAG,
                    StatsigErr::EvaluationError(e.to_string())
                );
                make_empty_result(EvaluationDetails::error(&e.to_string()))
            }
        }
    }

    fn get_feature_gate_impl(
        &self,
        user_internal: &StatsigUserInternal,
        gate_name: &str,
    ) -> FeatureGate {
        self.evaluate_spec(
            user_internal,
            gate_name,
            |eval_details| make_feature_gate(gate_name, None, eval_details, None, None),
            |mut result, eval_details| {
                let evaluation = result_to_gate_eval(gate_name, &mut result);
                make_feature_gate(
                    gate_name,
                    Some(evaluation),
                    eval_details,
                    result.version,
                    result.override_config_name,
                )
            },
            &SpecType::Gate,
        )
    }

    fn get_dynamic_config_impl(
        &self,
        user_internal: &StatsigUserInternal,
        config_name: &str,
    ) -> DynamicConfig {
        self.evaluate_spec(
            user_internal,
            config_name,
            |eval_details| make_dynamic_config(config_name, None, eval_details, None, None),
            |mut result, eval_details| {
                let evaluation = result_to_dynamic_config_eval(config_name, &mut result);
                make_dynamic_config(
                    config_name,
                    Some(evaluation),
                    eval_details,
                    result.version,
                    result.override_config_name,
                )
            },
            &SpecType::DynamicConfig,
        )
    }

    fn get_experiment_impl(
        &self,
        user_internal: &StatsigUserInternal,
        experiment_name: &str,
    ) -> Experiment {
        self.evaluate_spec(
            user_internal,
            experiment_name,
            |eval_details| make_experiment(experiment_name, None, eval_details, None, None),
            |mut result, eval_details| {
                let data = read_lock_or_else!(self.spec_store.data, {
                    let evaluation = result_to_experiment_eval(experiment_name, None, &mut result);
                    return make_experiment(
                        experiment_name,
                        Some(evaluation),
                        eval_details,
                        result.version,
                        result.override_config_name,
                    );
                });
                let evaluation = result_to_experiment_eval(
                    experiment_name,
                    data.values.dynamic_configs.get(experiment_name),
                    &mut result,
                );
                make_experiment(
                    experiment_name,
                    Some(evaluation),
                    eval_details,
                    result.version,
                    result.override_config_name,
                )
            },
            &SpecType::Experiment,
        )
    }

    fn get_layer_impl(
        &self,
        user_internal: StatsigUserInternal,
        layer_name: &str,
        evaluation_options: LayerEvaluationOptions,
    ) -> Layer {
        let disable_exposure_logging = evaluation_options.disable_exposure_logging;
        let event_logger_ptr = Arc::downgrade(&self.event_logger);
        let sampling_processor_ptr = Arc::downgrade(&self.sampling_processor);

        if disable_exposure_logging {
            self.event_logger
                .increment_non_exposure_checks_count(layer_name.to_string());
        }

        let mut layer = self.evaluate_spec(
            &user_internal,
            layer_name,
            |eval_details| {
                make_layer(
                    user_internal.to_loggable(),
                    layer_name,
                    None,
                    eval_details,
                    None,
                    None,
                    disable_exposure_logging,
                    None,
                    None,
                )
            },
            |mut result, eval_details| {
                let evaluation = result_to_layer_eval(layer_name, &mut result);
                let event_logger_ptr = Arc::downgrade(&self.event_logger);
                let sampling_processor_ptr = Arc::downgrade(&self.sampling_processor);

                make_layer(
                    user_internal.to_loggable(),
                    layer_name,
                    Some(evaluation),
                    eval_details,
                    Some(event_logger_ptr),
                    result.version,
                    disable_exposure_logging,
                    Some(sampling_processor_ptr),
                    result.override_config_name,
                )
            },
            &SpecType::Layer,
        );
        if let Some(persisted_layer) = self.persistent_values_manager.as_ref().and_then(|p| {
            p.try_apply_sticky_value_to_layer(
                &user_internal,
                &evaluation_options,
                &layer,
                Some(event_logger_ptr),
                Some(sampling_processor_ptr),
                disable_exposure_logging,
            )
        }) {
            layer = persisted_layer
        }
        layer
    }

    fn log_gate_exposure(
        &self,
        user: StatsigUserInternal,
        gate_name: &str,
        gate: &FeatureGate,
        is_manual: bool,
    ) {
        let gate_eval = gate.__evaluation.as_ref();
        let secondary_exposures = gate_eval.map(|eval| &eval.base.secondary_exposures);
        let sampling_details = self.sampling_processor.get_sampling_decision_and_details(
            &user.get_sampling_key(),
            gate_eval.map(AnyEvaluation::from).as_ref(),
            None,
        );

        if !sampling_details.should_send_exposure {
            return;
        }

        self.event_logger
            .enqueue(QueuedEventPayload::GateExposure(GateExposure {
                user: user.to_loggable(),
                gate_name: gate_name.to_string(),
                value: gate.value,
                rule_id: Some(gate.rule_id.clone()),
                secondary_exposures: secondary_exposures.cloned(),
                evaluation_details: gate.details.clone(),
                version: gate.__version,
                is_manual_exposure: is_manual,
                sampling_details,
                override_config_name: gate.__override_config_name.clone(),
            }));
    }

    fn log_dynamic_config_exposure(
        &self,
        user_internal: StatsigUserInternal,
        dynamic_config_name: &str,
        dynamic_config: &DynamicConfig,
        is_manual: bool,
    ) {
        let config_eval = dynamic_config.__evaluation.as_ref();
        let base_eval = config_eval.map(|eval| eval.base.clone());
        let loggable_user = user_internal.to_loggable();

        let sampling_details = self.sampling_processor.get_sampling_decision_and_details(
            &loggable_user.get_sampling_key(),
            config_eval.map(AnyEvaluation::from).as_ref(),
            None,
        );

        if !sampling_details.should_send_exposure {
            return;
        }

        self.event_logger
            .enqueue(QueuedEventPayload::ConfigExposure(ConfigExposure {
                user: loggable_user,
                evaluation: base_eval,
                evaluation_details: dynamic_config.details.clone(),
                config_name: dynamic_config_name.to_string(),
                rule_passed: dynamic_config.__evaluation.as_ref().map(|eval| eval.passed),
                version: dynamic_config.__version,
                is_manual_exposure: is_manual,
                sampling_details,
                override_config_name: dynamic_config.__override_config_name.clone(),
            }));
    }

    fn log_experiment_exposure(
        &self,
        user_internal: StatsigUserInternal,
        experiment_name: &str,
        experiment: &Experiment,
        is_manual: bool,
    ) {
        let experiment_eval = experiment.__evaluation.as_ref();
        let base_eval = experiment_eval.map(|eval| eval.base.clone());

        let sampling_details = self.sampling_processor.get_sampling_decision_and_details(
            &user_internal.get_sampling_key(),
            experiment_eval.map(AnyEvaluation::from).as_ref(),
            None,
        );

        if !sampling_details.should_send_exposure {
            return;
        }

        let loggable_user = user_internal.to_loggable();

        self.event_logger
            .enqueue(QueuedEventPayload::ConfigExposure(ConfigExposure {
                user: loggable_user,
                evaluation: base_eval,
                evaluation_details: experiment.details.clone(),
                config_name: experiment_name.to_string(),
                rule_passed: None,
                version: experiment.__version,
                is_manual_exposure: is_manual,
                sampling_details,
                override_config_name: experiment.__override_config_name.clone(),
            }));
    }

    fn internalize_user<'s, 'u>(&'s self, user: &'u StatsigUser) -> StatsigUserInternal<'s, 'u> {
        StatsigUserInternal::new(user, Some(self))
    }

    fn set_default_environment_from_server(&self) {
        let data = read_lock_or_else!(self.spec_store.data, {
            return;
        });

        if let Some(default_env) = data.values.default_environment.as_ref() {
            let env_map = HashMap::from([("tier".to_string(), dyn_value!(default_env.as_str()))]);

            if let Ok(mut fallback_env) = self.fallback_environment.lock() {
                *fallback_env = Some(env_map);
            }
        }
    }

    fn log_init_finish(
        &self,
        success: bool,
        error_message: &Option<String>,
        duration: &f64,
        specs_info: &SpecsInfo,
    ) {
        let is_store_populated = specs_info.source != SpecsSource::NoValues;
        let source_str = specs_info.source.to_string();
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Dist,
            "initialization".to_string(),
            *duration,
            Some(HashMap::from([
                ("success".to_owned(), success.to_string()),
                ("source".to_owned(), source_str.clone()),
                ("store_populated".to_owned(), is_store_populated.to_string()),
            ])),
        ));
        self.ops_stats.add_marker(
            {
                let marker = Marker::new(KeyType::Overall, ActionType::End, None)
                    .with_is_success(success)
                    .with_config_spec_ready(specs_info.source != SpecsSource::NoValues)
                    .with_source(source_str);

                if let Some(msg) = &error_message {
                    marker.with_message(msg.to_string())
                } else {
                    marker
                }
            },
            Some(ContextType::Initialize),
        );
        self.ops_stats
            .enqueue_diagnostics_event(None, Some(ContextType::Initialize));
    }
}

fn initialize_event_logging_adapter(
    sdk_key: &str,
    options: &StatsigOptions,
) -> Arc<dyn EventLoggingAdapter> {
    options
        .event_logging_adapter
        .clone()
        .unwrap_or_else(|| Arc::new(StatsigHttpEventLoggingAdapter::new(sdk_key, Some(options))))
}

fn initialize_specs_adapter(
    sdk_key: &str,
    options: &StatsigOptions,
    hashing: &HashUtil,
) -> Arc<dyn SpecsAdapter> {
    if let Some(adapter) = options.specs_adapter.clone() {
        log_d!(TAG, "Using provided SpecsAdapter: {}", sdk_key);
        return adapter;
    }

    if let Some(adapter_config) = options.spec_adapters_config.clone() {
        return Arc::new(StatsigCustomizedSpecsAdapter::new_from_config(
            sdk_key,
            adapter_config,
            options,
            hashing,
        ));
    }

    if let Some(data_adapter) = options.data_store.clone() {
        return Arc::new(StatsigCustomizedSpecsAdapter::new_from_data_store(
            sdk_key,
            data_adapter,
            options,
            hashing,
        ));
    }

    Arc::new(StatsigHttpSpecsAdapter::new(sdk_key, Some(options)))
}

fn initialize_id_lists_adapter(
    sdk_key: &str,
    options: &StatsigOptions,
) -> Option<Arc<dyn IdListsAdapter>> {
    if let Some(id_lists_adapter) = options.id_lists_adapter.clone() {
        return Some(id_lists_adapter);
    }

    if options.enable_id_lists.unwrap_or(false) {
        return Some(Arc::new(StatsigHttpIdListsAdapter::new(sdk_key, options)));
    }

    None
}

fn setup_ops_stats(
    sdk_key: &str,
    options: &StatsigOptions,
    statsig_runtime: Arc<StatsigRuntime>,
    error_observer: &Arc<dyn OpsStatsEventObserver>,
    diagnostics_observer: &Arc<dyn OpsStatsEventObserver>,
    external_observer: &Option<Weak<dyn ObservabilityClient>>,
) -> Arc<OpsStatsForInstance> {
    // TODO migrate output logger to use ops_stats
    initialize_simple_output_logger(&options.output_log_level);

    let ops_stat = OPS_STATS.get_for_instance(sdk_key);
    ops_stat.subscribe(statsig_runtime.clone(), Arc::downgrade(error_observer));
    ops_stat.subscribe(
        statsig_runtime.clone(),
        Arc::downgrade(diagnostics_observer),
    );

    if let Some(ob_client) = external_observer {
        if let Some(client) = ob_client.upgrade() {
            client.init();
            let as_observer = client.to_ops_stats_event_observer();
            ops_stat.subscribe(statsig_runtime, Arc::downgrade(&as_observer));
        }
    }

    ops_stat
}
