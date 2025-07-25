pub mod data_store_napi;
pub mod gcir_options_napi;
pub mod net_provider_napi;
pub mod observability_client_napi;
pub mod persistent_storage_napi;
pub mod statsig_core_api_options_napi;
pub mod statsig_napi;
pub mod statsig_options_napi;
pub mod statsig_result;
pub mod statsig_types_napi;
pub mod statsig_user_napi;

mod statsig_metadata_napi;

use napi::module_init;

#[module_init]
fn init() {
    statsig_metadata_napi::update_statsig_metadata(None);
}
