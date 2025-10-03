use napi_derive::napi;
use statsig_rust::{ClientInitResponseOptions as ClientInitResponseOptionsActual, HashAlgorithm};
use std::collections::HashSet;

#[napi(object, object_to_js = false)]
pub struct ClientInitResponseOptions {
    pub hash_algorithm: Option<String>,
    pub client_sdk_key: Option<String>,
    pub include_local_overrides: Option<bool>,
    pub feature_gate_filter: Option<HashSet<String>>,
    pub experiment_filter: Option<HashSet<String>>,
    pub dynamic_config_filter: Option<HashSet<String>>,
    pub layer_filter: Option<HashSet<String>>,
    pub param_store_filter: Option<HashSet<String>>,
    pub remove_id_type: Option<bool>,
    pub remove_default_value_gates: Option<bool>,
}

impl From<ClientInitResponseOptions> for ClientInitResponseOptionsActual {
    fn from(options: ClientInitResponseOptions) -> Self {
        let hash_algorithm = options
            .hash_algorithm
            .map(|s| HashAlgorithm::from_string(&s))
            .unwrap_or_default();

        ClientInitResponseOptionsActual {
            hash_algorithm,
            client_sdk_key: options.client_sdk_key,
            include_local_overrides: options.include_local_overrides,
            feature_gate_filter: options.feature_gate_filter,
            experiment_filter: options.experiment_filter,
            dynamic_config_filter: options.dynamic_config_filter,
            layer_filter: options.layer_filter,
            param_store_filter: options.param_store_filter,
            response_format: None,
            remove_id_type: options.remove_id_type,
            remove_default_value_gates: options.remove_default_value_gates,
        }
    }
}
