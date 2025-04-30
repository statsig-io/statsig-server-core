use serde::Deserialize;

use crate::{GCIRResponseFormat, HashAlgorithm};

#[derive(Default, Deserialize)]
pub struct ClientInitResponseOptions {
    pub hash_algorithm: Option<HashAlgorithm>,
    pub client_sdk_key: Option<String>,
    pub include_local_overrides: Option<bool>,
    pub response_format: Option<GCIRResponseFormat>,
}

impl ClientInitResponseOptions {
    pub(crate) fn get_hash_algorithm(&self) -> &HashAlgorithm {
        self.hash_algorithm.as_ref().unwrap_or(&HashAlgorithm::Djb2)
    }
}
