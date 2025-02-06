use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use sigstat::{log_d, log_e, Statsig as StatsigActual};

use crate::gcir_options_napi::ClientInitResponseOptions;
use crate::observability_client_napi::ObservabilityClient;
use crate::statsig_metadata_napi;
use crate::statsig_options_napi::StatsigOptions;
use crate::statsig_result::StatsigResult;
use crate::statsig_types_napi::{DynamicConfig, Experiment, FeatureGate, Layer};
use crate::statsig_user_napi::StatsigUser;

const TAG: &str = "StatsigNapi";

#[napi]
pub struct Statsig {
    inner: Arc<StatsigActual>,
    observability_client: Mutex<Option<Arc<ObservabilityClient>>>,
}

#[napi]
impl Statsig {
    #[napi(constructor)]
    pub fn new(env: Env, sdk_key: String, options: Option<StatsigOptions>) -> Self {
        log_d!(TAG, "StatsigNapi new");

        statsig_metadata_napi::update_statsig_metadata(Some(env));

        let (inner_opts, obs_client) = options
            .map(|opts| opts.safe_convert_to_inner())
            .unwrap_or((None, None));

        Self {
            inner: Arc::new(StatsigActual::new(&sdk_key, inner_opts)),
            observability_client: Mutex::new(obs_client),
        }
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
    pub fn shutdown<'env>(&self, env: &'env Env) -> Result<PromiseRaw<'env, StatsigResult>> {
        if let Ok(mut lock) = self.observability_client.lock() {
            lock.take();
        }
        let inst = self.inner.clone();

        env.spawn_future(async move {
            match inst.__shutdown_internal(Duration::from_secs(3)).await {
                Ok(_) => Ok(StatsigResult::success()),
                Err(e) => Ok(StatsigResult::error(e.to_string())),
            }
        })
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
    pub fn check_gate(&self, user: &StatsigUser, gate_name: String) -> bool {
        self.inner.check_gate(user.as_inner(), &gate_name)
    }

    #[napi]
    pub fn get_feature_gate(&self, user: &StatsigUser, feature_name: String) -> FeatureGate {
        self.inner
            .get_feature_gate(user.as_inner(), &feature_name)
            .into()
    }

    #[napi]
    pub fn get_dynamic_config(&self, user: &StatsigUser, config_name: String) -> DynamicConfig {
        self.inner
            .get_dynamic_config(user.as_inner(), &config_name)
            .into()
    }

    #[napi]
    pub fn get_experiment(&self, user: &StatsigUser, experiment_name: String) -> Experiment {
        self.inner
            .get_experiment(user.as_inner(), &experiment_name)
            .into()
    }

    #[napi]
    pub fn get_layer(&self, user: &StatsigUser, layer_name: String) -> Layer {
        self.inner.get_layer(user.as_inner(), &layer_name).into()
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
                .get_client_init_response_with_options_as_string(user.as_inner(), &options.into())
                .to_string(),
            None => self
                .inner
                .get_client_init_response_as_string(user.as_inner()),
        }
    }
}
