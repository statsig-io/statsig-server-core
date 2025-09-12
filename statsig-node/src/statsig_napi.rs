use crate::gcir_options_napi::ClientInitResponseOptions;
use crate::net_provider_napi::{NapiNetworkFunc, NetworkProviderNapi};
use crate::observability_client_napi::ObservabilityClient;
use crate::statsig_core_api_options_napi::{
    DynamicConfigEvaluationOptionsNapi, ExperimentEvaluationOptionsNapi,
    FeatureGateEvaluationOptionsNapi, LayerEvaluationOptionsNapi,
    ParameterStoreEvaluationOptionsNapi,
};
use crate::statsig_metadata_napi;
use crate::statsig_options_napi::StatsigOptions;
use crate::statsig_result::StatsigResult;
use crate::statsig_types_napi::{DynamicConfig, Experiment, FeatureGate, Layer, ParameterStore};
use crate::statsig_user_napi::StatsigUser;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use parking_lot::Mutex;
use serde_json::Value;
use statsig_rust::event_logging::statsig_event_internal::StatsigLogLineLevel;
use statsig_rust::networking::providers::NetworkProviderGlobal;
use statsig_rust::networking::NetworkProvider;
use statsig_rust::{log_d, log_e, Statsig as StatsigActual};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const TAG: &str = "StatsigNapi";

#[napi]
pub struct StatsigNapiInternal {
    pub(crate) inner: Arc<StatsigActual>,
    observability_client: Mutex<Option<Arc<ObservabilityClient>>>,
    network_provider: Mutex<Option<Arc<dyn NetworkProvider>>>,
}

#[napi]
impl StatsigNapiInternal {
    #[napi(constructor)]
    pub fn new(
        env: Env,
        #[napi(ts_arg_type = "unknown")] network_func: NapiNetworkFunc,
        sdk_key: String,
        options: Option<StatsigOptions>,
    ) -> Self {
        within_runtime_if_available(|| {
            log_d!(TAG, "StatsigNapi new");

            statsig_metadata_napi::update_statsig_metadata(Some(env));

            let (inner_opts, obs_client) = options
                .map(|opts| opts.safe_convert_to_inner())
                .unwrap_or((None, None));

            let network_provider: Arc<dyn NetworkProvider> =
                Arc::new(NetworkProviderNapi { network_func });
            NetworkProviderGlobal::set(&network_provider);

            Self {
                inner: Arc::new(StatsigActual::new(&sdk_key, inner_opts)),
                observability_client: Mutex::new(obs_client),
                network_provider: Mutex::new(Some(network_provider)),
            }
        })
    }

    #[napi]
    pub async fn initialize(&self) -> StatsigResult {
        if let Err(e) = self.inner.initialize().await {
            log_e!(TAG, "Failed to initialize Statsig: {}", e);
            return StatsigResult {
                is_success: false,
                error: Some(e.to_string()),
            };
        }

        StatsigResult {
            is_success: true,
            error: None,
        }
    }

    #[napi]
    pub async fn shutdown(&self, timeout_ms: Option<u32>) -> StatsigResult {
        let network_provider = match self.network_provider.try_lock_for(Duration::from_secs(5)) {
            Some(mut lock) => lock.take(),
            None => {
                log_e!(TAG, "Failed to lock network provider");
                None
            }
        };

        let obs_client = match self
            .observability_client
            .try_lock_for(Duration::from_secs(5))
        {
            Some(mut lock) => lock.take(),
            None => {
                log_e!(TAG, "Failed to lock observability client");
                None
            }
        };

        let timeout = Duration::from_millis(timeout_ms.unwrap_or(3000) as u64);
        let result = match self.inner.shutdown_with_timeout(timeout).await {
            Ok(_) => StatsigResult::success(),
            Err(e) => StatsigResult::error(e.to_string()),
        };

        // held until the shutdown is complete
        drop(network_provider);
        drop(obs_client);

        result
    }

    #[napi]
    pub async fn flush_events(&self) -> StatsigResult {
        self.inner.flush_events().await;
        StatsigResult::success()
    }

    #[napi]
    pub fn log_event(
        &self,
        user: &StatsigUser,
        event_name: String,
        #[napi(ts_arg_type = "string | number | null")] value: Option<serde_json::Value>,
        metadata: Option<HashMap<String, String>>,
    ) {
        match value {
            Some(Value::Number(num)) => {
                let num = match num.as_f64() {
                    Some(num) => num,
                    None => {
                        log_e!(TAG, "Failed to convert number {} to f64", num);
                        return;
                    }
                };

                self.inner
                    .log_event_with_number(user.as_inner(), &event_name, Some(num), metadata)
            }
            Some(Value::String(s)) => {
                self.inner
                    .log_event(user.as_inner(), &event_name, Some(s), metadata)
            }
            _ => self
                .inner
                .log_event(user.as_inner(), &event_name, None, metadata),
        }
    }

    #[napi]
    pub fn forward_log_line_event(
        &self,
        user: &StatsigUser,
        #[napi(ts_arg_type = "'trace' | 'debug' |'log' | 'info' | 'warn' | 'error'")]
        log_level: String,
        value: Option<String>,
        metadata: Option<HashMap<String, String>>,
    ) {
        let level = match log_level.to_lowercase().as_str() {
            "debug" => StatsigLogLineLevel::Debug,
            "log" => StatsigLogLineLevel::Log,
            "trace" => StatsigLogLineLevel::Trace,
            "info" => StatsigLogLineLevel::Info,
            "warn" => StatsigLogLineLevel::Warn,
            "error" => StatsigLogLineLevel::Error,
            _ => {
                log_e!(TAG, "Invalid log level: {}", log_level);
                return;
            }
        };
        self.inner
            .forward_log_line_event(user.as_inner(), level, value, metadata);
    }

    #[napi]
    pub fn check_gate(
        &self,
        user: &StatsigUser,
        gate_name: String,
        options: Option<FeatureGateEvaluationOptionsNapi>,
    ) -> bool {
        self.inner.check_gate_with_options(
            user.as_inner(),
            &gate_name,
            options.map(|opts| opts.into()).unwrap_or_default(),
        )
    }

    #[napi]
    pub fn get_feature_gate(
        &self,
        user: &StatsigUser,
        feature_name: String,
        options: Option<FeatureGateEvaluationOptionsNapi>,
    ) -> FeatureGate {
        self.inner
            .get_feature_gate_with_options(
                user.as_inner(),
                &feature_name,
                options.map(|opts| opts.into()).unwrap_or_default(),
            )
            .into()
    }

    #[napi]
    pub fn get_fields_needed_for_gate(&self, gate_name: String) -> Vec<String> {
        self.inner.get_fields_needed_for_gate(gate_name.as_str())
    }

    #[napi]
    pub fn get_dynamic_config(
        &self,
        user: &StatsigUser,
        config_name: String,
        options: Option<DynamicConfigEvaluationOptionsNapi>,
    ) -> DynamicConfig {
        self.inner
            .get_dynamic_config_with_options(
                user.as_inner(),
                &config_name,
                options.map(|opts| opts.into()).unwrap_or_default(),
            )
            .into()
    }

    #[napi]
    pub fn get_fields_needed_for_dynamic_config(&self, config_name: String) -> Vec<String> {
        self.inner
            .get_fields_needed_for_dynamic_config(config_name.as_str())
    }

    #[napi]
    pub fn get_experiment(
        &self,
        user: &StatsigUser,
        experiment_name: String,
        options: Option<ExperimentEvaluationOptionsNapi>,
    ) -> Experiment {
        self.inner
            .get_experiment_with_options(
                user.as_inner(),
                &experiment_name,
                options.map(|opts| opts.into()).unwrap_or_default(),
            )
            .into()
    }

    #[napi]
    pub fn get_experiment_by_group_name(
        &self,
        experiment_name: String,
        group_name: String,
    ) -> Experiment {
        self.inner
            .get_experiment_by_group_name(&experiment_name, &group_name)
            .into()
    }

    #[napi]
    pub fn get_fields_needed_for_experiment(&self, experiment_name: String) -> Vec<String> {
        self.inner
            .get_fields_needed_for_experiment(experiment_name.as_str())
    }

    #[napi]
    pub fn get_layer(
        &self,
        user: &StatsigUser,
        layer_name: String,
        options: Option<LayerEvaluationOptionsNapi>,
    ) -> Layer {
        self.inner
            .get_layer_with_options(
                user.as_inner(),
                &layer_name,
                options.map(|opts| opts.into()).unwrap_or_default(),
            )
            .into()
    }

    #[napi]
    pub fn get_prompt(
        &self,
        user: &StatsigUser,
        prompt_name: String,
        options: Option<LayerEvaluationOptionsNapi>,
    ) -> Layer {
        self.inner
            .get_prompt_with_options(
                user.as_inner(),
                &prompt_name,
                options.map(|opts| opts.into()).unwrap_or_default(),
            )
            .into()
    }

    #[napi]
    pub fn get_fields_needed_for_layer(&self, layer_name: String) -> Vec<String> {
        self.inner.get_fields_needed_for_layer(layer_name.as_str())
    }

    #[napi]
    pub fn identify(&self, user: &StatsigUser) {
        self.inner.identify(user.as_inner());
    }

    #[napi]
    pub fn get_parameter_store<'a, 'b: 'a, 'c: 'a>(
        &'b self,
        user: &'c StatsigUser,
        parameter_store_name: String,
        options: Option<ParameterStoreEvaluationOptionsNapi>,
    ) -> ParameterStore<'a> {
        let inner = self.inner.get_parameter_store_with_options(
            &parameter_store_name,
            options.map(|opts| opts.into()).unwrap_or_default(),
        );
        ParameterStore {
            name: parameter_store_name,
            user,
            inner,
        }
    }

    #[napi]
    pub fn get_client_initialize_response(
        &self,
        user: &StatsigUser,
        options: Option<ClientInitResponseOptions>,
    ) -> String {
        match options {
            Some(options) => self
                .inner
                .get_client_init_response_with_options_as_string(user.as_inner(), &options.into()),
            None => self
                .inner
                .get_client_init_response_as_string(user.as_inner()),
        }
    }

    #[napi]
    pub fn manually_log_feature_gate_exposure(&self, user: &StatsigUser, gate_name: String) {
        self.inner
            .manually_log_gate_exposure(user.as_inner(), &gate_name);
    }

    #[napi]
    pub fn manually_log_dynamic_config_exposure(&self, user: &StatsigUser, config_name: String) {
        self.inner
            .manually_log_dynamic_config_exposure(user.as_inner(), &config_name);
    }

    #[napi]
    pub fn manually_log_experiment_exposure(&self, user: &StatsigUser, experiment_name: String) {
        self.inner
            .manually_log_experiment_exposure(user.as_inner(), &experiment_name);
    }

    #[napi]
    pub fn manually_log_layer_param_exposure(
        &self,
        user: &StatsigUser,
        layer_name: String,
        param_name: String,
    ) {
        self.inner
            .manually_log_layer_parameter_exposure(user.as_inner(), &layer_name, param_name);
    }

    #[napi]
    pub fn override_gate(&self, gate_name: String, value: bool, id: Option<String>) {
        self.inner.override_gate(&gate_name, value, id.as_deref());
    }

    #[napi]
    pub fn override_dynamic_config(
        &self,
        config_name: String,
        value: HashMap<String, Value>,
        id: Option<String>,
    ) {
        self.inner
            .override_dynamic_config(&config_name, value, id.as_deref());
    }

    #[napi]
    pub fn override_experiment(
        &self,
        experiment_name: String,
        value: HashMap<String, Value>,
        id: Option<String>,
    ) {
        self.inner
            .override_experiment(&experiment_name, value, id.as_deref());
    }

    #[napi]
    pub fn override_experiment_by_group_name(
        &self,
        experiment_name: String,
        group_name: String,
        id: Option<String>,
    ) {
        self.inner
            .override_experiment_by_group_name(&experiment_name, &group_name, id.as_deref());
    }

    #[napi]
    pub fn override_layer(
        &self,
        layer_name: String,
        value: HashMap<String, Value>,
        id: Option<String>,
    ) {
        self.inner.override_layer(&layer_name, value, id.as_deref());
    }

    #[napi]
    pub fn remove_gate_override(&self, gate_name: String, id: Option<String>) {
        self.inner.remove_gate_override(&gate_name, id.as_deref());
    }

    #[napi]
    pub fn remove_dynamic_config_override(&self, config_name: String, id: Option<String>) {
        self.inner
            .remove_dynamic_config_override(&config_name, id.as_deref());
    }

    #[napi]
    pub fn remove_experiment_override(&self, experiment_name: String, id: Option<String>) {
        self.inner
            .remove_experiment_override(&experiment_name, id.as_deref());
    }

    #[napi]
    pub fn remove_layer_override(&self, layer_name: String, id: Option<String>) {
        self.inner.remove_layer_override(&layer_name, id.as_deref());
    }

    #[napi]
    pub fn remove_all_overrides(&self) {
        self.inner.remove_all_overrides();
    }

    #[napi]
    pub fn get_feature_gate_list(&self) -> Vec<String> {
        self.inner.get_feature_gate_list()
    }

    #[napi]
    pub fn get_dynamic_config_list(&self) -> Vec<String> {
        self.inner.get_dynamic_config_list()
    }

    #[napi]
    pub fn get_experiment_list(&self) -> Vec<String> {
        self.inner.get_experiment_list()
    }

    #[napi]
    pub fn get_parameter_store_list(&self) -> Vec<String> {
        self.inner.get_parameter_store_list()
    }

    #[napi]
    pub fn __get_parsed_user_agent_value_for_user(
        &self,
        user: &StatsigUser,
    ) -> Option<HashMap<String, Option<String>>> {
        self.inner
            .__get_parsed_user_agent_value(user.as_inner())
            .map(|v| {
                HashMap::from([
                    ("os_name".to_string(), v.os_name),
                    ("os_version".to_string(), v.os_version),
                    ("browser_name".to_string(), v.browser_name),
                    ("browser_version".to_string(), v.browser_version),
                ])
            })
    }
}
