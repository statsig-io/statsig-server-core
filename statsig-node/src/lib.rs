pub mod data_store_napi;
pub mod gcir_options_napi;
pub mod observability_client_napi;
pub mod output_logger_napi;
pub mod persistent_storage_napi;
pub mod sdk_event_emitter_napi;
pub mod statsig_core_api_options_napi;
pub mod statsig_napi;
pub mod statsig_options_napi;
pub mod statsig_result;
pub mod statsig_types_napi;
pub mod statsig_user_napi;

mod statsig_metadata_napi;

use napi::{bindgen_prelude::create_custom_tokio_runtime, module_init};

#[module_init]
fn init() {
    statsig_metadata_napi::update_statsig_metadata(None);

    let rt = statsig_rust::statsig_runtime::create_new_runtime();
    create_custom_tokio_runtime(rt);
}
