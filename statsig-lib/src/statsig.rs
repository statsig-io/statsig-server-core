use crate::client_init_response_formatter::{
    ClientInitResponseFormatter, ClientInitResponseOptions,
};
use crate::diagnostics::diagnostics::Diagnostics;
use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::evaluation::evaluator::Evaluator;
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
use crate::hashing::Hashing;
use crate::initialize_response::InitializeResponse;
use crate::output_logger::initialize_simple_output_logger;
use crate::statsig_core_api_options::{CheckGateOptions, GetDynamicConfigOptions, GetExperimentOptions, GetFeatureGateOptions};
use crate::spec_store::{SpecStore, SpecStoreData};
use crate::spec_types::Spec;
use crate::specs_adapter::{StatsigCustomizedSpecsAdapter, StatsigHttpSpecsAdapter};
use crate::statsig_err::StatsigErr;
use crate::statsig_options::StatsigOptions;
use crate::statsig_runtime::StatsigRuntime;
use crate::statsig_type_factories::{
    make_dynamic_config, make_experiment, make_feature_gate, make_layer,
};
use crate::statsig_types::{DynamicConfig, Experiment, FeatureGate, Layer};
use crate::statsig_user_internal::StatsigUserInternal;
use crate::{
    dyn_value, log_d, log_e, log_w, read_lock_or_else, IdListsAdapter, SpecsAdapter, SpecsSource,
    StatsigUser,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::try_join;

pub struct Statsig {
    sdk_key: String,
    options: Arc<StatsigOptions>,
    event_logger: Arc<EventLogger>,
    specs_adapter: Arc<dyn SpecsAdapter>,
    event_logging_adapter: Arc<dyn EventLoggingAdapter>,
    id_lists_adapter: Option<Arc<dyn IdListsAdapter>>,
    spec_store: Arc<SpecStore>,
    hashing: Hashing,
    gcir_formatter: Arc<ClientInitResponseFormatter>,
    statsig_environment: Option<HashMap<String, DynamicValue>>,
    statsig_runtime: Arc<StatsigRuntime>,
    fallback_environment: Mutex<Option<HashMap<String, DynamicValue>>>,
    diagnostics: Diagnostics,
}

impl Statsig {
    pub fn new(sdk_key: &str, options: Option<Arc<StatsigOptions>>) -> Self {
        let statsig_runtime = StatsigRuntime::get_runtime();

        let spec_store = Arc::new(SpecStore::new());
        let options = options.unwrap_or_default();

        initialize_simple_output_logger(&options.output_log_level);

        let specs_adapter = initialize_specs_adapter(sdk_key, &options);
        let id_lists_adapter = initialize_id_lists_adapter(&options);
        let event_logging_adapter = initialize_event_logging_adapter(sdk_key, &options);

        let environment = options
            .environment
            .as_ref()
            .map(|env| HashMap::from([("tier".into(), dyn_value!(env.as_str()))]));

        let event_logger = Arc::new(EventLogger::new(
            event_logging_adapter.clone(),
            &options,
            &statsig_runtime,
        ));
        let diagnostics = Diagnostics::new(event_logger.clone());

        Statsig {
            sdk_key: sdk_key.to_string(),
            options,
            gcir_formatter: Arc::new(ClientInitResponseFormatter::new(&spec_store)),
            event_logger,
            hashing: Hashing::new(),
            statsig_environment: environment,
            fallback_environment: Mutex::new(None),
            spec_store,
            specs_adapter,
            event_logging_adapter,
            id_lists_adapter,
            diagnostics,
            statsig_runtime,
        }
    }

    pub async fn initialize(&self) -> Result<(), StatsigErr> {
        self.diagnostics.mark_init_overall_start();
        self.spec_store.set_source(SpecsSource::Loading);
        self.event_logger
            .clone()
            .start_background_task(&self.statsig_runtime);

        let mut success = true;
        let mut error_message: Option<String> = None;

        let init_res = match self
            .specs_adapter
            .clone()
            .start(&self.statsig_runtime, self.spec_store.clone())
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
            adapter
                .clone()
                .start(&self.statsig_runtime, self.spec_store.clone())
                .await?;

            if let Err(e) = adapter.sync_id_lists().await {
                log_e!("Failed to sync id lists: {}", e);
                success = false;
                error_message.get_or_insert_with(|| format!("Failed to sync ID lists: {}", e));
            }
        }

        self.event_logging_adapter
            .clone()
            .start(&self.statsig_runtime)
            .await?;

        let _ = self
            .specs_adapter
            .clone()
            .schedule_background_sync(&self.statsig_runtime);

        self.set_default_environment_from_server();

        self.diagnostics
            .mark_init_overall_end(success, error_message);

        init_res
    }

    pub async fn shutdown_with_timeout(&self, timeout: Duration) -> Result<(), StatsigErr> {
        log_d!(
            "Shutting down Statsig with timeout {}ms",
            timeout.as_millis()
        );

        let start = Instant::now();
        let final_result = tokio::select! {
            _ = tokio::time::sleep(timeout) => {
                log_w!("Statsig shutdown timed out. {}", start.elapsed().as_millis());
                Err(StatsigErr::ShutdownTimeout)
            }
            sub_result = async {
                try_join!(
                    self.event_logger.shutdown(timeout),
                    self.specs_adapter.shutdown(timeout, &self.statsig_runtime),
                )
            } => {
                match sub_result {
                    Ok(_) => {
                        log_d!("All Statsig tasks shutdown successfully");
                        Ok(())
                    }
                    Err(e) => {
                        log_w!("Error during shutdown: {:?}", e);
                        Err(e)
                    }
                }
            }
        };

        self.finalize_shutdown(timeout.saturating_sub(start.elapsed()));
        final_result
    }

    pub async fn shutdown(&self) -> Result<(), StatsigErr> {
        self.shutdown_with_timeout(Duration::from_secs(3)).await
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
                        log_d!("Shutdown successfully");
                        callback();
                    }
                    Err(e) => {
                        log_e!("Shutdown failed: {:?}", e);
                        callback();
                    }
                }
            });
    }

    pub fn finalize_shutdown(&self, timeout: Duration) {
        self.statsig_runtime.shutdown(timeout);
    }

    // todo: add type for Context
    pub fn get_context(&self) -> (String, Arc<StatsigOptions>) {
        (self.sdk_key.clone(), self.options.clone())
    }

    // todo: merge into get_context
    pub fn get_current_values(&self) -> Option<SpecStoreData> {
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

    // ---------––
    //   Core Apis
    // ---------––

    pub fn check_gate_with_options(&self, user: &StatsigUser, gate_name: &str, check_gate_options: CheckGateOptions) -> bool {
        log_d!("Check Gate {}", gate_name);

        let user_internal = self.internalize_user(user);
        let (value, rule_id, secondary_exposures, details, version) = 
            self.check_gate_impl(&user_internal, gate_name);

        if check_gate_options.disable_exposure_logging {
            log_d!("Exposure logging is disabled for gate {}", gate_name);
            self.event_logger.increment_non_exposure_checks_count(gate_name.to_string());
            return value;
        }

        self.event_logger
            .enqueue(QueuedEventPayload::GateExposure(GateExposure {
                user: user_internal,
                gate_name: gate_name.to_string(),
                value,
                rule_id,
                secondary_exposures,
                evaluation_details: details,
                version,
            }));

        value  
    }

    pub fn check_gate(&self, user: &StatsigUser, gate_name: &str) -> bool {
        self.check_gate_with_options(user, gate_name, CheckGateOptions::default())
    }

    pub fn get_feature_gate_with_options(&self, user: &StatsigUser, gate_name: &str, get_feature_gate_options: GetFeatureGateOptions) -> FeatureGate {
        log_d!("Get Feature Gate {}", gate_name);

        let user_internal = self.internalize_user(user);
        let gate = self.get_feature_gate_impl(&user_internal, gate_name);

        if get_feature_gate_options.disable_exposure_logging {
            log_d!("Exposure logging is disabled for gate {}", gate_name);
            self.event_logger.increment_non_exposure_checks_count(gate_name.to_string());
            return gate;
        }

        self.log_gate_exposure(user_internal, gate_name, &gate);

        gate
    }

    pub fn get_feature_gate(&self, user: &StatsigUser, gate_name: &str) -> FeatureGate {
        self.get_feature_gate_with_options(user, gate_name, GetFeatureGateOptions::default())
    }

    pub fn get_dynamic_config_with_options(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
        get_dynamic_config_options: GetDynamicConfigOptions,
    ) -> DynamicConfig {
        let user_internal = self.internalize_user(user);
        let dynamic_config = self.get_dynamic_config_impl(&user_internal, dynamic_config_name);

        if get_dynamic_config_options.disable_exposure_logging {
            log_d!("Exposure logging is disabled for dynamic_config {}", dynamic_config_name);
            self.event_logger.increment_non_exposure_checks_count(dynamic_config_name.to_string());
            return dynamic_config;
        }
        self.log_dynamic_config_exposure(user_internal, dynamic_config_name, &dynamic_config);

        dynamic_config
    }

    pub fn get_dynamic_config(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
    ) -> DynamicConfig {
        self.get_dynamic_config_with_options(user, dynamic_config_name, GetDynamicConfigOptions::default())
    }

    pub fn get_experiment_with_options(&self, user: &StatsigUser, experiment_name: &str, get_experiment_options: GetExperimentOptions) -> Experiment {
        let user_internal = self.internalize_user(user);
        let experiment = self.get_experiment_impl(&user_internal, experiment_name);

        if get_experiment_options.disable_exposure_logging {
            log_d!("Exposure logging is disabled for experiment {}", experiment_name);
            self.event_logger.increment_non_exposure_checks_count(experiment_name.to_string());
            return experiment;
        }
        self.log_experiment_exposure(user_internal, experiment_name, &experiment);

        experiment
    }

    pub fn get_experiment(&self, user: &StatsigUser, experiment_name: &str) -> Experiment {
        self.get_experiment_with_options(user, experiment_name, GetExperimentOptions::default())
    }

    pub fn get_layer(&self, user: &StatsigUser, layer_name: &str) -> Layer {
        let user_internal = self.internalize_user(user);
        self.get_layer_impl(&user_internal, layer_name)
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

    pub fn log_layer_param_exposure(&self, layer_json: String, parameter_name: String) {
        let layer = match serde_json::from_str::<Layer>(&layer_json) {
            Ok(layer) => layer,
            Err(e) => {
                log_e!("Failed to parse Layer. Exposure will be dropped. {}", e);
                return;
            }
        };

        self.event_logger
            .enqueue(QueuedEventPayload::LayerExposure(LayerExposure {
                user: layer.__user,
                parameter_name,
                evaluation: layer.__evaluation,
                layer_name: layer.name,
                evaluation_details: layer.details,
                version: layer.__version,
            }));
    }

    pub async fn flush_events(&self) {
        self.event_logger.flush_blocking().await
    }

    // ---------––
    //   Private
    // ---------––

    fn evaluate_spec<T>(
        &self,
        user_internal: &StatsigUserInternal,
        get_spec: impl FnOnce(&SpecStoreData) -> Option<&Spec>,
        make_empty_result: impl FnOnce(EvaluationDetails) -> T,
        make_result: impl FnOnce(&Spec, EvaluatorResult, EvaluationDetails) -> T,
    ) -> T {
        let data = read_lock_or_else!(self.spec_store.data, {
            return make_empty_result(EvaluationDetails::unrecognized_no_data());
        });

        let spec = get_spec(&data);

        match spec {
            Some(spec) => {
                let mut context = EvaluatorContext::new(user_internal, &data, &self.hashing);
                Evaluator::evaluate(&mut context, spec);
                let eval_details = EvaluationDetails::recognized(&data, &context.result);

                make_result(spec, context.result, eval_details)
            }
            None => make_empty_result(EvaluationDetails::unrecognized(&data)),
        }
    }

    fn check_gate_impl(
        &self,
        user_internal: &StatsigUserInternal,
        gate_name: &str,
    ) -> (
        bool,
        Option<String>,
        Option<Vec<SecondaryExposure>>,
        EvaluationDetails,
        Option<u32>,
    ) {
        self.evaluate_spec(
            user_internal,
            |data| data.values.feature_gates.get(gate_name),
            |eval_details| (false, None, None, eval_details, None),
            |_spec, result, eval_details| {
                (
                    result.bool_value,
                    result.rule_id.cloned(),
                    Some(result.secondary_exposures),
                    eval_details,
                    result.version,
                )
            },
        )
    }

    fn get_feature_gate_impl(
        &self,
        user_internal: &StatsigUserInternal,
        gate_name: &str,
    ) -> FeatureGate {
        self.evaluate_spec(
            user_internal,
            |data| data.values.feature_gates.get(gate_name),
            |eval_details| make_feature_gate(gate_name, None, eval_details, None),
            |_spec, mut result, eval_details| {
                let evaluation = result_to_gate_eval(gate_name, &mut result);
                make_feature_gate(gate_name, Some(evaluation), eval_details, result.version)
            },
        )
    }

    fn get_dynamic_config_impl(
        &self,
        user_internal: &StatsigUserInternal,
        config_name: &str,
    ) -> DynamicConfig {
        self.evaluate_spec(
            user_internal,
            |data| data.values.dynamic_configs.get(config_name),
            |eval_details| make_dynamic_config(config_name, None, eval_details, None),
            |_spec, mut result, eval_details| {
                let evaluation = result_to_dynamic_config_eval(config_name, &mut result);
                make_dynamic_config(config_name, Some(evaluation), eval_details, result.version)
            },
        )
    }

    fn get_experiment_impl(
        &self,
        user_internal: &StatsigUserInternal,
        experiment_name: &str,
    ) -> Experiment {
        self.evaluate_spec(
            user_internal,
            |data| data.values.dynamic_configs.get(experiment_name),
            |eval_details| make_experiment(experiment_name, None, eval_details, None),
            |spec, mut result, eval_details| {
                let evaluation = result_to_experiment_eval(experiment_name, spec, &mut result);
                make_experiment(
                    experiment_name,
                    Some(evaluation),
                    eval_details,
                    result.version,
                )
            },
        )
    }

    fn get_layer_impl(&self, user_internal: &StatsigUserInternal, layer_name: &str) -> Layer {
        self.evaluate_spec(
            user_internal,
            |data| data.values.layer_configs.get(layer_name),
            |eval_details| make_layer(user_internal, layer_name, None, eval_details, None, None),
            |_spec, mut result, eval_details| {
                let evaluation = result_to_layer_eval(layer_name, &mut result);
                let event_logger_ptr = Arc::downgrade(&self.event_logger);

                make_layer(
                    user_internal,
                    layer_name,
                    Some(evaluation),
                    eval_details,
                    Some(event_logger_ptr),
                    result.version,
                )
            },
        )
    }

    fn log_gate_exposure(
        &self,
        user_internal: StatsigUserInternal,
        gate_name: &str,
        gate: &FeatureGate,
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
            }));
    }

    fn log_dynamic_config_exposure(
        &self,
        user_internal: StatsigUserInternal,
        dynamic_config_name: &str,
        dynamic_config: &DynamicConfig,
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
            }));
    }

    fn log_experiment_exposure(
        &self,
        user_internal: StatsigUserInternal,
        experiment_name: &str,
        experiment: &Experiment,
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
            }));
    }

    fn internalize_user(&self, user: &StatsigUser) -> StatsigUserInternal {
        return StatsigUserInternal::new(user, self.get_statsig_env());
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

fn initialize_specs_adapter(sdk_key: &str, options: &StatsigOptions) -> Arc<dyn SpecsAdapter> {
    if let Some(adapter) = options.specs_adapter.clone() {
        return adapter;
    }

    if let Some(adapter_config) = options.spec_adapters_config.clone() {
        return Arc::new(StatsigCustomizedSpecsAdapter::new(sdk_key, adapter_config));
    }

    Arc::new(StatsigHttpSpecsAdapter::new(
        sdk_key,
        options.specs_url.as_ref(),
        options.specs_sync_interval_ms,
    ))
}

fn initialize_id_lists_adapter(options: &StatsigOptions) -> Option<Arc<dyn IdListsAdapter>> {
    options.id_lists_adapter.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let configs = response.dynamic_configs;
        assert_eq!(configs.len(), 62);

        let value = match configs.values().next() {
            Some(v) => match v {
                AnyConfigEvaluation::DynamicConfig(config) => &config.value,
                AnyConfigEvaluation::Experiment(exp) => &exp.value,
            },
            None => panic!("Should have values"),
        };

        assert!(value.is_object());
    }
}
