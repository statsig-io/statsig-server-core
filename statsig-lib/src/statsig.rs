use crate::client_init_response_formatter::ClientInitResponseFormatter;
use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluator::Evaluator;
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::evaluation::evaluator_result::{
    result_to_dynamic_config_eval, result_to_experiment_eval, result_to_gate_eval,
    result_to_layer_eval,
};
use crate::event_logging::config_exposure::ConfigExposure;
use crate::event_logging::event_logger::{EventLogger, QueuedEventPayload};
use crate::event_logging::gate_exposure::GateExposure;
use crate::event_logging::layer_exposure::LayerExposure;
use crate::event_logging::statsig_event::StatsigEvent;
use crate::event_logging::statsig_event_internal::make_custom_event;
use crate::event_logging_adapter::event_logging_adapter::EventLoggingAdapter;
use crate::event_logging_adapter::statsig_event_logging_adapter::StatsigEventLoggingAdapter;
use crate::initialize_response::InitializeResponse;
use crate::memo_sha_256::MemoSha256;
use crate::spec_store::{SpecStore, SpecStoreData};
use crate::specs_adapter::statsig_http_specs_adapter::StatsigHttpSpecsAdapter;
use crate::statsig_err::StatsigErr;
use crate::statsig_options::StatsigOptions;
use crate::statsig_type_factories::{
    make_dynamic_config, make_experiment, make_feature_gate, make_layer,
};
use crate::statsig_types::{DynamicConfig, Experiment, FeatureGate, Layer};
use crate::statsig_user_internal::StatsigUserInternal;
use crate::{
    dyn_value, log_d, log_e, read_lock_or_else, SpecsAdapter, SpecsSource, SpecsUpdateListener,
    StatsigUser,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::{Builder, Handle, Runtime};
use tokio::try_join;
use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::output_logger::initialize_simple_output_logger;

pub struct Statsig {
    sdk_key: String,
    options: Arc<StatsigOptions>,
    event_logger: Arc<EventLogger>,
    specs_adapter: Arc<dyn SpecsAdapter>,
    spec_store: Arc<SpecStore>,
    sha_hasher: MemoSha256,
    gcir_formatter: Arc<ClientInitResponseFormatter>,
    statsig_environment: Option<HashMap<String, DynamicValue>>,
    runtime: Mutex<Option<Runtime>>,
    runtime_handle: Handle,
}

impl Statsig {
    pub fn new(sdk_key: &str, options: Option<Arc<StatsigOptions>>) -> Self {
        let spec_store = Arc::new(SpecStore::new());
        let options = options.unwrap_or_default();

        initialize_simple_output_logger(&options.output_log_level);

        let specs_adapter = initialize_specs_adapter(sdk_key, &options);

        let event_logging_adapter = initialize_event_logging_adapter(sdk_key, &options);

        let (opt_runtime, runtime_handle) = create_runtime_if_required();

        let environment = options
            .environment
            .as_ref()
            .map(|env| HashMap::from([("tier".into(), dyn_value!(env.as_str()))]));

        let event_logger =
            EventLogger::new(event_logging_adapter.clone(), &options, &runtime_handle);

        Statsig {
            sdk_key: sdk_key.to_string(),
            options,
            gcir_formatter: Arc::new(ClientInitResponseFormatter::new(&spec_store)),
            event_logger: Arc::new(event_logger),
            sha_hasher: MemoSha256::new(),
            statsig_environment: environment,
            runtime: Mutex::new(opt_runtime),
            spec_store,
            specs_adapter,
            runtime_handle,
        }
    }

    pub async fn initialize(&self) -> Result<(), StatsigErr> {
        self.event_logger.clone().start_background_task();

        self.specs_adapter
            .clone()
            .start(&self.runtime_handle, self.spec_store.clone())
            .await?;

        if let Some(id_lists_adapter) = &self.options.id_lists_adapter {
            id_lists_adapter.clone().start(&self.runtime_handle).await?;
        }

        let info = self.spec_store.get_current_specs_info();
        let is_uninitialized = info.source == SpecsSource::Uninitialized;
        if is_uninitialized {
            self.spec_store.set_source(SpecsSource::Loading);
        }

        match self.specs_adapter.manually_sync_specs(info.lcut).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if is_uninitialized {
                    self.spec_store.set_source(SpecsSource::NoValues);
                }
                Err(e)
            }
        }
    }

    pub fn initialize_with_callback<F>(&self, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.event_logger.clone().start_background_task();

        let info = self.spec_store.get_current_specs_info();
        let is_uninitialized = info.source == SpecsSource::Uninitialized;
        if is_uninitialized {
            self.spec_store.set_source(SpecsSource::Loading);
        }

        let adapter = self.specs_adapter.clone();
        let handle = self.runtime_handle.clone();
        let store = self.spec_store.clone();

        self.runtime_handle.spawn(async move {
            // todo: return result to callback
            let _ = adapter.clone().start(&handle, store).await;

            let _ = adapter.manually_sync_specs(info.lcut).await;
            callback();
        });
    }

    pub async fn shutdown(&self) -> Result<(), StatsigErr> {
        let timeout = Duration::from_millis(1000);
        try_join!(
            self.event_logger.shutdown(timeout),
            self.specs_adapter.shutdown(timeout)
        )?;

        if let Ok(mut lock) = self.runtime.lock() {
            if let Some(runtime) = lock.take() {
                runtime.shutdown_timeout(Duration::from_secs(1))
            }
        }

        Ok(())
    }

    pub fn get_context(&self) -> (String, Arc<StatsigOptions>) {
        (self.sdk_key.clone(), self.options.clone())
    }

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
        let user_internal = StatsigUserInternal::new(user, &self.statsig_environment);

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

    pub fn check_gate(&self, user: &StatsigUser, gate_name: &str) -> bool {
        log_d!("Check Gate {}", gate_name);

        let user_internal = StatsigUserInternal::new(user, &self.statsig_environment);
        let (value, rule_id, secondary_exposures, details) =
            self.check_gate_impl(&user_internal, gate_name);

        self.event_logger
            .enqueue(QueuedEventPayload::GateExposure(GateExposure {
                user: user_internal,
                gate_name: gate_name.to_string(),
                value,
                rule_id,
                secondary_exposures,
                evaluation_details: details,
            }));

        value
    }

    pub fn get_feature_gate(&self, user: &StatsigUser, gate_name: &str) -> FeatureGate {
        let user_internal = StatsigUserInternal::new(user, &self.statsig_environment);
        let gate = self.get_feature_gate_impl(&user_internal, gate_name);

        self.log_gate_exposure(user_internal, gate_name, &gate);

        gate
    }

    pub fn get_dynamic_config(
        &self,
        user: &StatsigUser,
        dynamic_config_name: &str,
    ) -> DynamicConfig {
        let user_internal = StatsigUserInternal::new(user, &self.statsig_environment);
        let dynamic_config = self.get_dynamic_config_impl(&user_internal, dynamic_config_name);

        self.log_dynamic_config_exposure(user_internal, dynamic_config_name, &dynamic_config);

        dynamic_config
    }

    pub fn get_experiment(&self, user: &StatsigUser, experiment_name: &str) -> Experiment {
        let user_internal = StatsigUserInternal::new(user, &self.statsig_environment);
        let experiment = self.get_experiment_impl(&user_internal, experiment_name);

        self.log_experiment_exposure(user_internal, experiment_name, &experiment);

        experiment
    }

    pub fn get_layer(&self, user: &StatsigUser, layer_name: &str) -> Layer {
        let user_internal = StatsigUserInternal::new(user, &self.statsig_environment);
        self.get_layer_impl(&user_internal, layer_name)
    }

    pub fn get_client_init_response(&self, user: &StatsigUser) -> InitializeResponse {
        let user_internal = StatsigUserInternal::new(user, &self.statsig_environment);
        self.gcir_formatter.get(user_internal, &self.sha_hasher)
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
            }));
    }

    pub async fn flush_events(&self) {
        self.event_logger.flush_blocking().await
    }

    // ---------––
    //   Private
    // ---------––
    fn check_gate_impl(
        &self,
        user_internal: &StatsigUserInternal,
        gate_name: &str,
    ) -> (
        bool,
        Option<String>,
        Option<Vec<SecondaryExposure>>,
        EvaluationDetails,
    ) {
        let data = read_lock_or_else!(self.spec_store.data, {
            return (false, None, None, EvaluationDetails::unrecognized_no_data());
        });

        let spec = data.values.feature_gates.get(gate_name);

        match spec {
            Some(spec) => {
                let mut context = EvaluatorContext::new(user_internal, &data, &self.sha_hasher);
                Evaluator::evaluate(&mut context, spec);

                (
                    context.result.bool_value,
                    context.result.rule_id.cloned(),
                    Some(context.result.secondary_exposures),
                    EvaluationDetails::recognized(&data),
                )
            }
            None => (false, None, None, EvaluationDetails::unrecognized(&data)),
        }
    }
    fn get_feature_gate_impl(
        &self,
        user_internal: &StatsigUserInternal,
        gate_name: &str,
    ) -> FeatureGate {
        let data = read_lock_or_else!(self.spec_store.data, {
            return make_feature_gate(gate_name, None, EvaluationDetails::unrecognized_no_data());
        });

        let spec = data.values.feature_gates.get(gate_name);

        match spec {
            Some(spec) => {
                let mut context = EvaluatorContext::new(user_internal, &data, &self.sha_hasher);
                Evaluator::evaluate(&mut context, spec);

                let evaluation = result_to_gate_eval(gate_name, spec, &mut context.result);
                make_feature_gate(
                    gate_name,
                    Some(evaluation),
                    EvaluationDetails::recognized(&data),
                )
            }
            None => make_feature_gate(gate_name, None, EvaluationDetails::unrecognized(&data)),
        }
    }

    fn get_dynamic_config_impl(
        &self,
        user_internal: &StatsigUserInternal,
        config_name: &str,
    ) -> DynamicConfig {
        let data = read_lock_or_else!(self.spec_store.data, {
            return make_dynamic_config(
                config_name,
                None,
                EvaluationDetails::unrecognized_no_data(),
            );
        });

        let spec = data.values.dynamic_configs.get(config_name);

        match spec {
            Some(spec) => {
                let mut context = EvaluatorContext::new(user_internal, &data, &self.sha_hasher);
                Evaluator::evaluate(&mut context, spec);

                let evaluation =
                    result_to_dynamic_config_eval(config_name, spec, &mut context.result);
                make_dynamic_config(
                    config_name,
                    Some(evaluation),
                    EvaluationDetails::recognized(&data),
                )
            }
            None => make_dynamic_config(config_name, None, EvaluationDetails::unrecognized(&data)),
        }
    }

    fn get_experiment_impl(
        &self,
        user_internal: &StatsigUserInternal,
        experiment_name: &str,
    ) -> Experiment {
        let data = read_lock_or_else!(self.spec_store.data, {
            return make_experiment(
                experiment_name,
                None,
                EvaluationDetails::unrecognized_no_data(),
            );
        });

        let spec = data.values.dynamic_configs.get(experiment_name);

        match spec {
            Some(spec) => {
                let mut context = EvaluatorContext::new(user_internal, &data, &self.sha_hasher);
                Evaluator::evaluate(&mut context, spec);

                let evaluation =
                    result_to_experiment_eval(experiment_name, spec, &mut context.result);
                make_experiment(
                    experiment_name,
                    Some(evaluation),
                    EvaluationDetails::recognized(&data),
                )
            }
            None => make_experiment(
                experiment_name,
                None,
                EvaluationDetails::unrecognized(&data),
            ),
        }
    }

    fn get_layer_impl(&self, user_internal: &StatsigUserInternal, layer_name: &str) -> Layer {
        let data = read_lock_or_else!(self.spec_store.data, {
            return make_layer(
                user_internal,
                layer_name,
                None,
                EvaluationDetails::unrecognized_no_data(),
                None,
            );
        });

        let spec = data.values.layer_configs.get(layer_name);

        match spec {
            Some(spec) => {
                let mut context = EvaluatorContext::new(user_internal, &data, &self.sha_hasher);
                Evaluator::evaluate(&mut context, spec);

                let evaluation = result_to_layer_eval(layer_name, spec, &mut context.result);
                let event_logger_ptr = Arc::downgrade(&self.event_logger);
                make_layer(
                    user_internal,
                    layer_name,
                    Some(evaluation),
                    EvaluationDetails::recognized(&data),
                    Some(event_logger_ptr)
                )
            }
            None => make_layer(
                user_internal,
                layer_name,
                None,
                EvaluationDetails::unrecognized(&data),
                None
            ),
        }
    }

    fn log_gate_exposure(
        &self,
        user_internal: StatsigUserInternal,
        gate_name: &str,
        gate: &FeatureGate,
    ) {
        let secondary_exposures = match &gate.__evaluation {
            Some(eval) => Some(&eval.base.secondary_exposures),
            None => None,
        };

        self.event_logger
            .enqueue(QueuedEventPayload::GateExposure(GateExposure {
                user: user_internal,
                gate_name: gate_name.to_string(),
                value: gate.value,
                rule_id: Some(gate.rule_id.clone()),
                secondary_exposures: secondary_exposures.cloned(),
                evaluation_details: gate.details.clone(),
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
            .map(|eval| eval.base.base.clone());

        self.event_logger
            .enqueue(QueuedEventPayload::ConfigExposure(ConfigExposure {
                user: user_internal,
                evaluation: base_eval,
                evaluation_details: experiment.details.clone(),
                config_name: experiment_name.to_string(),
            }));
    }
}

fn initialize_event_logging_adapter(
    sdk_key: &str,
    options: &StatsigOptions,
) -> Arc<dyn EventLoggingAdapter> {
    let adapter = options
        .event_logging_adapter
        .clone()
        .unwrap_or_else(|| Arc::new(StatsigEventLoggingAdapter::new()));
    adapter.bind(sdk_key, options);
    adapter
}

fn initialize_specs_adapter(sdk_key: &str, options: &StatsigOptions) -> Arc<dyn SpecsAdapter> {
    options
        .specs_adapter
        .clone()
        .unwrap_or_else(|| Arc::new(StatsigHttpSpecsAdapter::new(sdk_key, options)))
}

fn create_runtime_if_required() -> (Option<Runtime>, Handle) {
    if let Ok(handle) = Handle::try_current() {
        return (None, handle);
    }

    let rt = Builder::new_multi_thread()
        .worker_threads(3)
        .thread_name("statsig")
        .enable_all()
        .build()
        .expect("Failed to find or create a tokio Runtime");

    let handle = rt.handle().clone();
    (Some(rt), handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::evaluation_types::AnyConfigEvaluation;

    const SDK_KEY: &str = "secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW";

    #[tokio::test]
    async fn test_check_gate() {
        let user = StatsigUser {
            email: Some(dyn_value!("daniel@statsig.com")),
            ..StatsigUser::with_user_id("a-user".to_string())
        };

        let statsig = Statsig::new(SDK_KEY, None);
        statsig.initialize().await.unwrap();

        let gate_result = statsig.check_gate(&user, "test_50_50");

        assert!(gate_result);
    }

    #[tokio::test]
    async fn test_get_experiment() {
        let user = StatsigUser {
            email: Some(dyn_value!("daniel@statsig.com")),
            ..StatsigUser::with_user_id("a-user".to_string())
        };

        let statsig = Statsig::new(SDK_KEY, None);
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
            ..StatsigOptions::new()
        };

        let statsig = Statsig::new(SDK_KEY, Some(Arc::new(opts)));
        statsig.initialize().await.unwrap();

        let response = statsig.get_client_init_response(&user);
        let _ = statsig.shutdown().await;

        let gates = response.feature_gates;
        assert_eq!(gates.len(), 64);

        let configs = response.dynamic_configs;
        assert_eq!(configs.len(), 62);

        let value = match configs.values().next() {
            Some(v) => match v {
                AnyConfigEvaluation::DynamicConfig(config) => &config.value,
                AnyConfigEvaluation::Experiment(exp) => &exp.base.value,
            },
            None => panic!("Should have values"),
        };

        assert!(value.is_object());
    }
}
