use serde::Deserialize;
use std::collections::HashSet;

use crate::{GCIRResponseFormat, HashAlgorithm};

#[derive(Default, Deserialize)]
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
}

impl ClientInitResponseOptions {
    pub(crate) fn get_hash_algorithm(&self) -> &HashAlgorithm {
        self.hash_algorithm.as_ref().unwrap_or(&HashAlgorithm::Djb2)
    }
}
