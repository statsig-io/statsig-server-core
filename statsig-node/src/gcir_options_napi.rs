use napi_derive::napi;
use statsig_rust::{ClientInitResponseOptions as ClientInitResponseOptionsActual, HashAlgorithm};

#[napi(object, object_to_js = false)]
pub struct ClientInitResponseOptions {
    pub hash_algorithm: Option<String>,
    pub client_sdk_key: Option<String>,
    pub include_local_overrides: Option<bool>,
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
        }
    }
}
