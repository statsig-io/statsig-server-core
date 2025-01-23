use crate::client_init_response_formatter::{
    ClientInitResponseFormatter, ClientInitResponseOptions,
};
use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluator::{Evaluator, SpecType};
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::evaluation::evaluator_result::{
    result_to_dynamic_config_eval, result_to_experiment_eval, result_to_gate_eval,
    result_to_layer_eval, EvaluatorResult,
};
use crate::event_logging::config_exposure::ConfigExposure;
use crate::event_logging::event_logger::{EventLogger, QueuedEventPayload};
use crate::event_logging::gate_exposure::GateExposure;
use crate::event_logging::layer_exposure::LayerExposure;
use crate::event_logging::statsig_event::StatsigEvent;
use crate::event_logging::statsig_event_internal::make_custom_event;
use crate::event_logging_adapter::EventLoggingAdapter;
use crate::event_logging_adapter::StatsigHttpEventLoggingAdapter;
use crate::hashing::HashUtil;
use crate::initialize_response::InitializeResponse;
use crate::observability::observability_client_adapter::{MetricType, ObservabilityEvent};
use crate::observability::ops_stats::{OpsStatsForInstance, OPS_STATS};
use crate::observability::sdk_errors_observer::{ErrorBoundaryEvent, SDKErrorsObserver};
use crate::output_logger::initialize_simple_output_logger;
use crate::sdk_diagnostics::diagnostics::Diagnostics;
use crate::spec_store::{SpecStore, SpecStoreData};
use crate::specs_adapter::{StatsigCustomizedSpecsAdapter, StatsigHttpSpecsAdapter};
use crate::statsig_err::StatsigErr;
use crate::statsig_options::StatsigOptions;
use crate::statsig_runtime::StatsigRuntime;
use crate::statsig_type_factories::{
    make_dynamic_config, make_experiment, make_feature_gate, make_layer,
};
use crate::statsig_types::{DynamicConfig, Experiment, FeatureGate, Layer, ParameterStore};
use crate::statsig_user_internal::StatsigUserInternal;
use crate::{
    dyn_value, log_d, log_e, log_w, read_lock_or_else, IdListsAdapter, OverrideAdapter,
    SpecsAdapter, SpecsInfo, SpecsSource, SpecsUpdateListener, StatsigHttpIdListsAdapter,
    StatsigUser,
};
use crate::{log_error_to_statsig_and_console, statsig_core_api_options::*};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::try_join;

const TAG: &str = stringify!(Statsig);

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
    hashing: HashUtil,
    gcir_formatter: Arc<ClientInitResponseFormatter>,
    statsig_environment: Option<HashMap<String, DynamicValue>>,
    fallback_environment: Mutex<Option<HashMap<String, DynamicValue>>>,
    diagnostics: Diagnostics,
    ops_stats: Arc<OpsStatsForInstance>,
}

pub struct StatsigContext {
    pub sdk_key: String,
    pub options: Arc<StatsigOptions>,
    pub local_override_adapter: Option<Arc<dyn OverrideAdapter>>,
    pub spec_store_data: Option<SpecStoreData>,
}

impl Statsig {
    pub fn new(sdk_key: &str, options: Option<Arc<StatsigOptions>>) -> Self {
        let statsig_runtime = StatsigRuntime::get_runtime();
        let options = options.unwrap_or_default();

        let ops_stats = setup_ops_stats(sdk_key, &options, statsig_runtime.clone());

        let spec_store = Arc::new(SpecStore::new(
            sdk_key,
            options.data_store.clone(),
            Some(statsig_runtime.clone()),
            ops_stats.clone(),
        ));

        let hashing = HashUtil::new();
        initialize_simple_output_logger(&options.output_log_level);

        let specs_adapter = initialize_specs_adapter(sdk_key, &options, &hashing);
        let id_lists_adapter = initialize_id_lists_adapter(sdk_key, &options);
        let event_logging_adapter = initialize_event_logging_adapter(sdk_key, &options);
        let override_adapter = options.override_adapter.as_ref().map(Arc::clone);

        let environment = options
            .environment
            .as_ref()
            .map(|env| HashMap::from([("tier".into(), dyn_value!(env.as_str()))]));

        let event_logger = Arc::new(EventLogger::new(
            sdk_key,
            event_logging_adapter.clone(),
            &options,
            &statsig_runtime,
        ));
        let diagnostics = Diagnostics::new(event_logger.clone(), spec_store.clone());

        Statsig {
            sdk_key: sdk_key.to_string(),
            options,
            gcir_formatter: Arc::new(ClientInitResponseFormatter::new(
                &spec_store,
                &override_adapter,
            )),
            event_logger,
            hashing,
            statsig_environment: environment,
            fallback_environment: Mutex::new(None),
            override_adapter,
            spec_store,
            specs_adapter,
            event_logging_adapter,
            id_lists_adapter,
            diagnostics,
            statsig_runtime,
            ops_stats,
        }
    }

    pub async fn initialize(&self) -> Result<(), StatsigErr> {
        let start_time = Instant::now();
        self.diagnostics.mark_init_overall_start();
        self.spec_store.set_source(SpecsSource::Loading);
        self.event_logger
            .clone()
            .start_background_task(&self.statsig_runtime);

        let mut success = true;
        let mut error_message: Option<String> = None;
        self.specs_adapter.initialize(self.spec_store.clone());

        let init_res = match self
            .specs_adapter
            .clone()
            .start(&self.statsig_runtime)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                self.spec_store.set_source(SpecsSource::NoValues);
                success = false;
                error_message = Some(format!("Failed to start specs adapter: {}", e));
                Err(e)
            }
        };

        if let Some(adapter) = &self.id_lists_adapter {
            match adapter
                .clone()
                .start(&self.statsig_runtime, self.spec_store.clone())
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    success = false;
                    error_message.get_or_insert_with(|| format!("Failed to sync ID lists: {}", e));
                }
            }
            if let Err(e) = adapter
                .clone()
                .schedule_background_sync(&self.statsig_runtime)
            {
                log_w!(TAG, "Failed to schedule idlist background job {}", e)
            }
        }

        self.event_logging_adapter
            .clone()
            .start(&self.statsig_runtime)
            .await
            .map_err(|e| {
                log_error_to_statsig_and_console!(
                    self.ops_stats.clone(),
                    TAG,
                    "Failed to start event logging adaper {}",
                    e
                );
                e
            })?;

        let _ = self
            .specs_adapter
            .clone()
            .schedule_background_sync(&self.statsig_runtime);

        self.set_default_environment_from_server();
        self.log_init_finish(
            success,
            &error_message,
            start_time,
            self.spec_store.get_current_specs_info(),
        );
        if let Err(e) = init_res.clone() {
            log_error_to_statsig_and_console!(
                self.ops_stats,
                TAG,
                "{}",
                error_message.unwrap_or(e.to_string())
            );
        }

        init_res
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
        let final_result = self.__shutdown_internal(timeout).await;
        self.finalize_shutdown(timeout.saturating_sub(start.elapsed()));
        final_result
    }

    pub async fn __shutdown_internal(&self, timeout: Duration) -> Result<(), StatsigErr> {
        log_d!(
            TAG,
            "Shutting down Statsig with timeout {}ms",
            timeout.as_millis()
        );

        let start = Instant::now();
        tokio::select! {
            _ = tokio::time::sleep(timeout) => {
                log_w!(TAG, "Statsig shutdown timed out. {}", start.elapsed().as_millis());
                Err(StatsigErr::ShutdownTimeout)
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
        }
    }

    pub fn sequenced_shutdown_prepare<F>(&self, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let event_logger = self.event_logger.clone();
        let specs_adapter = self.specs_adapter.clone();
        let runtime: Arc<StatsigRuntime> = self.statsig_runtime.clone();

        self.statsig_runtime
            .spawn("sequenced_shutdown_prep", |_shutdown_notify| async move {
                let timeout = Duration::from_millis(1000);

                let result = try_join!(
                    event_logger.shutdown(timeout),
                    specs_adapter.shutdown(timeout, &runtime)
                );

                match result {
                    Ok(_) => {
                        log_d!(TAG, "Shutdown successfully");
                        callback();
                    }
                    Err(e) => {
                        log_e!(TAG, "Shutdown failed: {:?}", e);
                        callback();
                    }
                }
            });
    }

    pub fn finalize_shutdown(&self, timeout: Duration) {
        self.statsig_runtime.shutdown(timeout);
    }

    pub fn get_context(&self) -> StatsigContext {
        StatsigContext {
            sdk_key: self.sdk_key.clone(),
            options: self.options.clone(),
            local_override_adapter: self.override_adapter.clone(),
            spec_store_data: self.get_current_values(),
        }
    }

    // todo: merge into get_context
    pub fn get_current_values(&self) -> Option<SpecStoreData> {
        // TODO better error handling here
        Some(self.spec_store.data.read().ok()?.clone())
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
                user_internal,
                StatsigEvent {
                    event_name: event_name.to_string(),
                    value: value.map(|v| json!(v)),
                    metadata,
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
                user_internal,
                StatsigEvent {
                    event_name: event_name.to_string(),
                    value: value.map(|v| json!(v)),
                    metadata,
                },
            )));
    }

    pub fn log_layer_param_exposure(&self, layer_json: String, parameter_name: String) {
        let layer = match serde_json::from_str::<Layer>(&layer_json) {
            Ok(layer) => layer,
            Err(e) => {
                log_error_to_statsig_and_console!(
                    self.ops_stats.clone(),
                    TAG,
                    "Shutdown failed: {:?}",
                    e
                );
                return;
            }
        };

        if layer.__disable_exposure {
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
            }));
    }

    pub async fn flush_events(&self) {
        self.event_logger.flush_blocking().await;
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
            .get(user_internal, &self.hashing, options)
    }

    pub fn get_client_init_response_with_options_as_string(
        &self,
        user: &StatsigUser,
        options: &ClientInitResponseOptions,
    ) -> String {
        json!(self.get_client_init_response_with_options(user, options)).to_string()
    }

    pub fn get_parameter_store(&self, parameter_store_name: &str) -> ParameterStore {
        self.event_logger
        .increment_non_exposure_checks_count(parameter_store_name.to_string());
        let data = read_lock_or_else!(self.spec_store.data, {
            log_error_to_statsig_and_console!(
                self.ops_stats.clone(),
                TAG,
                "Failed to acquire read lock for spec store data"
            );
            return ParameterStore {
                name: parameter_store_name.to_string(),
                parameters: HashMap::new(),
                details: EvaluationDetails::unrecognized_no_data(),
                _statsig_ref: self,
            };
        });

        let stores = &data.values.param_stores;
        let store = match stores {
            Some(stores) => {
                stores.get(parameter_store_name)
            }
            None => {
                return ParameterStore {
                    name: parameter_store_name.to_string(),
                    parameters: HashMap::new(),
                    details: EvaluationDetails::unrecognized(&data),
                    _statsig_ref: self,
                };
            }
        };
        match store {
            Some(store) => {
                ParameterStore {
                    name: parameter_store_name.to_string(),
                    parameters: store.clone(),
                    details: EvaluationDetails::recognized(&data, &EvaluatorResult::default()),
                    _statsig_ref: self,
                }
            }
            None => {
                ParameterStore {
                    name: parameter_store_name.to_string(),
                    parameters: HashMap::new(),
                    details: EvaluationDetails::unrecognized(&data),
                    _statsig_ref: self,
                }
            }
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
        log_d!(TAG, "Get Feature Gate {}", gate_name);

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
        let experiment = self.get_experiment_impl(&user_internal, experiment_name);

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
        self.get_layer_impl(&user_internal, layer_name, options)
    }

    pub fn manually_log_layer_parameter_exposure(
        &self,
        user: &StatsigUser,
        layer_name: &str,
        parameter_name: String,
    ) {
        let user_internal = self.internalize_user(user);
        let layer = self.get_layer_impl(
            &user_internal,
            layer_name,
            LayerEvaluationOptions::default(),
        );

        self.event_logger
            .enqueue(QueuedEventPayload::LayerExposure(LayerExposure {
                user: layer.__user,
                parameter_name,
                evaluation: layer.__evaluation,
                layer_name: layer.name,
                evaluation_details: layer.details,
                version: layer.__version,
                is_manual_exposure: true,
            }));
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
                self.ops_stats.clone(),
                TAG,
                "Failed to acquire read lock for spec store data"
            );
            return make_empty_result(EvaluationDetails::unrecognized_no_data());
        });
        let app_id = data.values.app_id.as_ref();
        let mut context = EvaluatorContext::new(
            user_internal,
            &data,
            &self.hashing,
            &app_id,
            &self.override_adapter,
        );

        match Evaluator::evaluate_with_details(&mut context, spec_name, spec_type) {
            Ok(eval_details) => make_result(context.result, eval_details),
            Err(e) => {
                log_error_to_statsig_and_console!(&self.ops_stats, TAG, "Error evaluating: {}", e);
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
            |eval_details| make_feature_gate(gate_name, None, eval_details, None),
            |mut result, eval_details| {
                let evaluation = result_to_gate_eval(gate_name, &mut result);
                make_feature_gate(gate_name, Some(evaluation), eval_details, result.version)
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
            |eval_details| make_dynamic_config(config_name, None, eval_details, None),
            |mut result, eval_details| {
                let evaluation = result_to_dynamic_config_eval(config_name, &mut result);
                make_dynamic_config(config_name, Some(evaluation), eval_details, result.version)
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
            |eval_details| make_experiment(experiment_name, None, eval_details, None),
            |mut result, eval_details| {
                let data = read_lock_or_else!(self.spec_store.data, {
                    let evaluation = result_to_experiment_eval(experiment_name, None, &mut result);
                    return make_experiment(
                        experiment_name,
                        Some(evaluation),
                        eval_details,
                        result.version,
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
                )
            },
            &SpecType::Experiment,
        )
    }

    fn get_layer_impl(
        &self,
        user_internal: &StatsigUserInternal,
        layer_name: &str,
        evaluation_options: LayerEvaluationOptions,
    ) -> Layer {
        let disable_exposure_logging = evaluation_options.disable_exposure_logging;
        if disable_exposure_logging {
            self.event_logger
                .increment_non_exposure_checks_count(layer_name.to_string());
        }

        self.evaluate_spec(
            user_internal,
            layer_name,
            |eval_details| {
                make_layer(
                    user_internal,
                    layer_name,
                    None,
                    eval_details,
                    None,
                    None,
                    disable_exposure_logging,
                )
            },
            |mut result, eval_details| {
                let evaluation = result_to_layer_eval(layer_name, &mut result);
                let event_logger_ptr = Arc::downgrade(&self.event_logger);

                make_layer(
                    user_internal,
                    layer_name,
                    Some(evaluation),
                    eval_details,
                    Some(event_logger_ptr),
                    result.version,
                    disable_exposure_logging,
                )
            },
            &SpecType::Layer,
        )
    }

    fn log_gate_exposure(
        &self,
        user_internal: StatsigUserInternal,
        gate_name: &str,
        gate: &FeatureGate,
        is_manual: bool,
    ) {
        let secondary_exposures = gate
            .__evaluation
            .as_ref()
            .map(|eval| &eval.base.secondary_exposures);

        self.event_logger
            .enqueue(QueuedEventPayload::GateExposure(GateExposure {
                user: user_internal,
                gate_name: gate_name.to_string(),
                value: gate.value,
                rule_id: Some(gate.rule_id.clone()),
                secondary_exposures: secondary_exposures.cloned(),
                evaluation_details: gate.details.clone(),
                version: gate.__version,
                is_manual_exposure: is_manual,
            }));
    }

    fn log_dynamic_config_exposure(
        &self,
        user_internal: StatsigUserInternal,
        dynamic_config_name: &str,
        dynamic_config: &DynamicConfig,
        is_manual: bool,
    ) {
        let base_eval = dynamic_config
            .__evaluation
            .as_ref()
            .map(|eval| eval.base.clone());

        self.event_logger
            .enqueue(QueuedEventPayload::ConfigExposure(ConfigExposure {
                user: user_internal,
                evaluation: base_eval,
                evaluation_details: dynamic_config.details.clone(),
                config_name: dynamic_config_name.to_string(),
                rule_passed: dynamic_config.__evaluation.as_ref().map(|eval| eval.passed),
                version: dynamic_config.__version,
                is_manual_exposure: is_manual,
            }));
    }

    fn log_experiment_exposure(
        &self,
        user_internal: StatsigUserInternal,
        experiment_name: &str,
        experiment: &Experiment,
        is_manual: bool,
    ) {
        let base_eval = experiment
            .__evaluation
            .as_ref()
            .map(|eval| eval.base.clone());

        self.event_logger
            .enqueue(QueuedEventPayload::ConfigExposure(ConfigExposure {
                user: user_internal,
                evaluation: base_eval,
                evaluation_details: experiment.details.clone(),
                config_name: experiment_name.to_string(),
                rule_passed: None,
                version: experiment.__version,
                is_manual_exposure: is_manual,
            }));
    }

    fn internalize_user(&self, user: &StatsigUser) -> StatsigUserInternal {
        StatsigUserInternal::new(user, self.get_statsig_env())
    }

    fn get_statsig_env(&self) -> Option<HashMap<String, DynamicValue>> {
        if let Some(env) = &self.statsig_environment {
            return Some(env.clone());
        }

        if let Ok(fallback_env) = self.fallback_environment.lock() {
            if let Some(env) = &*fallback_env {
                return Some(env.clone());
            }
        }

        None
    }

    fn set_default_environment_from_server(&self) {
        let data = read_lock_or_else!(self.spec_store.data, {
            return;
        });

        if let Some(default_env) = data.values.default_environment.as_ref() {
            let env_map = HashMap::from([("tier".to_string(), dyn_value!(default_env.as_str()))]);

            if let Ok(mut fallback_env) = self.fallback_environment.lock() {
                *fallback_env = Some(env_map)
            }
        }
    }

    fn log_init_finish(
        &self,
        success: bool,
        error_message: &Option<String>,
        start_time: Instant,
        specs_info: SpecsInfo,
    ) {
        self.ops_stats.log(ObservabilityEvent::new_event(
            MetricType::Dist,
            "initialization".to_string(),
            start_time.elapsed().as_millis() as f64,
            Some(HashMap::from([
                ("success".to_owned(), success.to_string()),
                ("source".to_owned(), specs_info.source.to_string()),
                (
                    "store_populated".to_owned(),
                    (specs_info.source != SpecsSource::NoValues).to_string(),
                ),
            ])),
        ));
        self.diagnostics.mark_init_overall_end(
            success,
            error_message.clone(),
            EvaluationDetails {
                reason: format!("{}", specs_info.source),
                lcut: specs_info.lcut,
                received_at: None,
            },
        );
    }
}

fn initialize_event_logging_adapter(
    sdk_key: &str,
    options: &StatsigOptions,
) -> Arc<dyn EventLoggingAdapter> {
    let adapter = options.event_logging_adapter.clone().unwrap_or_else(|| {
        Arc::new(StatsigHttpEventLoggingAdapter::new(
            sdk_key,
            options.log_event_url.as_ref(),
        ))
    });
    adapter
}

fn initialize_specs_adapter(
    sdk_key: &str,
    options: &StatsigOptions,
    hashing: &HashUtil,
) -> Arc<dyn SpecsAdapter> {
    if let Some(adapter) = options.specs_adapter.clone() {
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

    Arc::new(StatsigHttpSpecsAdapter::new(
        sdk_key,
        options.specs_url.as_ref(),
        options.fallback_to_statsig_api.unwrap_or(false),
        options.specs_sync_interval_ms,
    ))
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
) -> Arc<OpsStatsForInstance> {
    // TODO migrate output logger to use ops_stats
    initialize_simple_output_logger(&options.output_log_level);
    let ops_stat = OPS_STATS.get_for_instance(sdk_key);
    let error_observer = Arc::new(SDKErrorsObserver::new(
        sdk_key,
        serde_json::to_string(options).unwrap_or_default(),
    ));
    ops_stat.subscribe(statsig_runtime.clone(), error_observer);
    let maybe_ob_client = options.observability_client.clone();
    if let Some(ob_client) = maybe_ob_client {
        ob_client.init();
        ops_stat.subscribe(statsig_runtime, ob_client.to_ops_stats_event_observer());
    }

    ops_stat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::djb2;
    use crate::{
        evaluation::evaluation_types::AnyConfigEvaluation, output_logger::LogLevel,
        StatsigHttpIdListsAdapter,
    };
    use std::env;

    fn get_sdk_key() -> String {
        let key = env::var("test_api_key").expect("test_api_key environment variable not set");
        assert!(key.starts_with("secret-9IWf"));
        key
    }

    #[tokio::test]
    async fn test_check_gate() {
        let user = StatsigUser {
            email: Some(dyn_value!("daniel@statsig.com")),
            ..StatsigUser::with_user_id("a-user".to_string())
        };

        let statsig = Statsig::new(&get_sdk_key(), None);
        statsig.initialize().await.unwrap();

        let gate_result = statsig.check_gate(&user, "test_50_50");

        assert!(gate_result);
    }

    #[tokio::test]
    async fn test_check_gate_id_list() {
        let user = StatsigUser {
            custom_ids: Some(HashMap::from([(
                "companyID".to_string(),
                dyn_value!("marcos_1"),
            )])),
            ..StatsigUser::with_user_id("marcos_1".to_string())
        };

        let mut opts = StatsigOptions::new();

        let adapter = Arc::new(StatsigHttpIdListsAdapter::new(&get_sdk_key(), &opts));
        opts.id_lists_adapter = Some(adapter);

        let statsig = Statsig::new(&get_sdk_key(), Some(Arc::new(opts)));
        statsig.initialize().await.unwrap();

        let gate_result = statsig.check_gate(&user, "test_id_list");

        assert!(gate_result);
    }

    #[tokio::test]
    async fn test_get_experiment() {
        let user = StatsigUser {
            email: Some(dyn_value!("daniel@statsig.com")),
            ..StatsigUser::with_user_id("a-user".to_string())
        };

        let statsig = Statsig::new(&get_sdk_key(), None);
        statsig.initialize().await.unwrap();

        let experiment = statsig.get_experiment(&user, "running_exp_in_unlayered_with_holdout");
        let _ = statsig.shutdown().await;

        assert_ne!(experiment.value.len(), 0);
    }

    #[tokio::test]
    async fn test_gcir() {
        let user = StatsigUser {
            email: Some(dyn_value!("daniel@statsig.com")),
            ..StatsigUser::with_user_id("a-user".to_string())
        };
        let opts = StatsigOptions {
            output_log_level: Some(LogLevel::Debug),
            ..StatsigOptions::new()
        };

        let statsig = Statsig::new(&get_sdk_key(), Some(Arc::new(opts)));
        statsig.initialize().await.unwrap();

        let response = statsig.get_client_init_response(&user);
        let _ = statsig.shutdown().await;

        let gates = response.feature_gates;
        assert_eq!(gates.len(), 69);

        let configs = response.dynamic_configs.len();
        assert_eq!(configs, 62);

        let a_config_opt = response.dynamic_configs.get(&djb2("big_number"));
        let a_config = match a_config_opt {
            Some(v) => match v {
                AnyConfigEvaluation::DynamicConfig(config) => &config.value,
                AnyConfigEvaluation::Experiment(exp) => &exp.value,
            },
            None => panic!("Should have values"),
        };

        assert!(!a_config.is_empty());
    }
}
