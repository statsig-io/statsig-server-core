use serde::Deserialize;
use std::collections::HashSet;

use crate::{GCIRResponseFormat, HashAlgorithm};

#[derive(Deserialize)]
pub struct ClientInitResponseOptions {
    pub hash_algorithm: Option<HashAlgorithm>,
    pub client_sdk_key: Option<String>,
    pub include_local_overrides: Option<bool>,
    pub feature_gate_filter: Option<HashSet<String>>,
    pub experiment_filter: Option<HashSet<String>>,
    pub dynamic_config_filter: Option<HashSet<String>>,
    pub layer_filter: Option<HashSet<String>>,
    pub param_store_filter: Option<HashSet<String>>,
    pub response_format: Option<GCIRResponseFormat>,
    pub remove_id_type: Option<bool>,
    pub remove_default_value_gates: Option<bool>,
    pub previous_response_hash: Option<String>,
    pub remove_experiments_in_layers: Option<bool>,
}

impl ClientInitResponseOptions {
    pub(crate) fn get_hash_algorithm(&self) -> &HashAlgorithm {
        self.hash_algorithm.as_ref().unwrap_or(&HashAlgorithm::Djb2)
    }
}

impl Default for ClientInitResponseOptions {
    fn default() -> Self {
        Self {
            hash_algorithm: Some(HashAlgorithm::Djb2),
            client_sdk_key: None,
            include_local_overrides: Some(false),
            feature_gate_filter: None,
            experiment_filter: None,
            dynamic_config_filter: None,
            layer_filter: None,
            param_store_filter: None,
            response_format: None,
            remove_id_type: Some(false),
            remove_default_value_gates: Some(false),
            previous_response_hash: None,
            remove_experiments_in_layers: Some(false),
        }
    }
}
