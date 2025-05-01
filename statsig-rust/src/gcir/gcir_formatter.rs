use crate::gcir::feature_gates_processor::get_gate_evaluations;

use crate::{
    evaluation::evaluator_context::EvaluatorContext,
    hashing::{HashAlgorithm, HashUtil},
    initialize_evaluations_response::InitializeEvaluationsResponse,
    initialize_response::InitializeResponse,
    log_e, read_lock_or_else,
    spec_store::{SpecStore, SpecStoreData},
    statsig_metadata::StatsigMetadata,
    user::StatsigUserInternal,
    OverrideAdapter, StatsigErr,
};

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use super::dynamic_configs_processor::{
    get_dynamic_config_evaluations, get_dynamic_config_evaluations_v2,
};
use super::feature_gates_processor::get_gate_evaluations_v2;
use super::gcir_options::ClientInitResponseOptions;
use super::layer_configs_processor::{get_layer_evaluations, get_layer_evaluations_v2};
use super::param_stores_processor::get_serializeable_param_stores;
use super::target_app_id_utils::select_app_id;

pub struct GCIRFormatter {
    spec_store: Arc<SpecStore>,
    default_options: ClientInitResponseOptions,
    override_adapter: Option<Arc<dyn OverrideAdapter>>,
}

#[derive(Deserialize)]
pub enum GCIRResponseFormat {
    Initialize,                             // v1
    InitializeWithSecondaryExposureMapping, // v2
}

impl GCIRResponseFormat {
    #[must_use]
    pub fn from_string(input: &str) -> Option<Self> {
        match input {
            "v1" => Some(GCIRResponseFormat::Initialize),
            "v2" => Some(GCIRResponseFormat::InitializeWithSecondaryExposureMapping),
            _ => None,
        }
    }
}

const TAG: &str = "GCIRFormatter";

impl GCIRFormatter {
    pub fn new(
        spec_store: &Arc<SpecStore>,
        override_adapter: &Option<Arc<dyn OverrideAdapter>>,
    ) -> Self {
        Self {
            spec_store: spec_store.clone(),
            override_adapter: override_adapter.as_ref().map(Arc::clone),
            default_options: ClientInitResponseOptions {
                hash_algorithm: Some(HashAlgorithm::Djb2),
                client_sdk_key: None,
                include_local_overrides: Some(false),
                response_format: None,
            },
        }
    }

    pub fn get_default_options(&self) -> &ClientInitResponseOptions {
        &self.default_options
    }

    pub fn get_as_v1_format(
        &self,
        user_internal: StatsigUserInternal,
        hashing: &HashUtil,
        options: &ClientInitResponseOptions,
    ) -> InitializeResponse {
        self.get_v1_impl(&user_internal, hashing, options)
            .unwrap_or_else(|e| {
                log_e!(TAG, "Error getting client init response: {}", e);
                InitializeResponse::blank(user_internal)
            })
    }

    pub fn get_as_v2_format(
        &self,
        user_internal: StatsigUserInternal,
        hashing: &HashUtil,
        options: &ClientInitResponseOptions,
    ) -> InitializeEvaluationsResponse {
        self.get_v2_impl(&user_internal, hashing, options)
            .unwrap_or_else(|e| {
                log_e!(TAG, "Error getting client init evaluations response: {}", e);
                InitializeEvaluationsResponse::blank(user_internal)
            })
    }

    fn get_v2_impl(
        &self,
        user_internal: &StatsigUserInternal,
        hashing: &HashUtil,
        options: &ClientInitResponseOptions,
    ) -> Result<InitializeEvaluationsResponse, StatsigErr> {
        let data = read_lock_or_else!(self.spec_store.data, {
            return Err(StatsigErr::LockFailure(
                "Failed to acquire read lock for spec store data".to_string(),
            ));
        });

        let mut sec_expo_hash_memo = HashMap::new();
        let mut context = self.setup_evaluator_context(user_internal, &data, options, hashing);
        let mut exposures = HashMap::new();

        let param_stores = get_serializeable_param_stores(&mut context, options);
        let evaluated_keys = get_evaluated_keys(user_internal);

        Ok(InitializeEvaluationsResponse {
            feature_gates: get_gate_evaluations_v2(
                &mut context,
                options,
                &mut sec_expo_hash_memo,
                &mut exposures,
            )?,
            dynamic_configs: get_dynamic_config_evaluations_v2(
                &mut context,
                options,
                &mut sec_expo_hash_memo,
                &mut exposures,
            )?,
            layer_configs: get_layer_evaluations_v2(
                &mut context,
                options,
                &mut sec_expo_hash_memo,
                &mut exposures,
            )?,
            time: data.values.time,
            has_updates: true,
            hash_used: options.get_hash_algorithm().to_string(),
            user: user_internal.to_loggable(),
            sdk_params: HashMap::new(),
            evaluated_keys,
            sdk_info: get_sdk_info(),
            param_stores,
            exposures,
        })
    }

    fn get_v1_impl(
        &self,
        user_internal: &StatsigUserInternal,
        hashing: &HashUtil,
        options: &ClientInitResponseOptions,
    ) -> Result<InitializeResponse, StatsigErr> {
        let data = read_lock_or_else!(self.spec_store.data, {
            return Err(StatsigErr::LockFailure(
                "Failed to acquire read lock for spec store data".to_string(),
            ));
        });

        let mut sec_expo_hash_memo = HashMap::new();
        let mut context = self.setup_evaluator_context(user_internal, &data, options, hashing);

        let param_stores = get_serializeable_param_stores(&mut context, options);
        let evaluated_keys = get_evaluated_keys(user_internal);

        Ok(InitializeResponse {
            feature_gates: get_gate_evaluations(&mut context, options, &mut sec_expo_hash_memo)?,
            dynamic_configs: get_dynamic_config_evaluations(
                &mut context,
                options,
                &mut sec_expo_hash_memo,
            )?,
            layer_configs: get_layer_evaluations(&mut context, options, &mut sec_expo_hash_memo)?,
            time: data.values.time,
            has_updates: true,
            hash_used: options.get_hash_algorithm().to_string(),
            user: user_internal.to_loggable(),
            sdk_params: HashMap::new(),
            evaluated_keys,
            sdk_info: get_sdk_info(),
            param_stores,
        })
    }

    fn setup_evaluator_context<'a>(
        &'a self,
        user_internal: &'a StatsigUserInternal,
        data: &'a SpecStoreData,
        options: &'a ClientInitResponseOptions,
        hashing: &'a HashUtil,
    ) -> EvaluatorContext<'a> {
        let app_id = select_app_id(options, &data.values, hashing);

        let override_adapter = match options.include_local_overrides {
            Some(true) => self.override_adapter.as_ref(),
            _ => None,
        };

        EvaluatorContext::new(user_internal, data, hashing, app_id, override_adapter)
    }
}

fn get_evaluated_keys(user_internal: &StatsigUserInternal) -> HashMap<String, String> {
    let mut evaluated_keys = HashMap::new();

    if let Some(user_id) = user_internal.user_data.user_id.as_ref() {
        evaluated_keys.insert(
            "userID".to_string(),
            user_id
                .string_value
                .as_ref()
                .map(|s| s.value.clone())
                .unwrap_or_default(),
        );
    }

    if let Some(custom_ids) = user_internal.user_data.custom_ids.as_ref() {
        for (key, value) in custom_ids {
            evaluated_keys.insert(
                key.clone(),
                value
                    .string_value
                    .as_ref()
                    .map(|s| s.value.clone())
                    .unwrap_or_default(),
            );
        }
    }

    evaluated_keys
}

fn get_sdk_info() -> HashMap<String, String> {
    let metadata = StatsigMetadata::get_metadata();
    HashMap::from([
        ("sdkType".to_string(), metadata.sdk_type),
        ("sdkVersion".to_string(), metadata.sdk_version),
    ])
}
