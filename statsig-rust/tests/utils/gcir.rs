use std::sync::Arc;

use statsig_rust::{
    evaluation::evaluator_context::{EvaluatorContext, IdListResolution},
    gcir::gcir_formatter::GCIRFormatter,
    hashing::HashUtil,
    specs_response::spec_types::SpecsResponseFull,
    user::StatsigUserInternal,
    ClientInitResponseOptions, HashAlgorithm, InitializeResponse, OverrideAdapter, StatsigErr,
    StatsigUser,
};

use super::helpers::load_contents;

pub fn load_specs_data(path: &str) -> SpecsResponseFull {
    let contents = load_contents(path);
    serde_json::from_str::<SpecsResponseFull>(&contents).expect("should parse specs data")
}

pub fn client_init_response_options() -> ClientInitResponseOptions {
    ClientInitResponseOptions {
        hash_algorithm: Some(HashAlgorithm::None),
        ..Default::default()
    }
}

pub fn evaluator_context<'a>(
    user_internal: &'a StatsigUserInternal<'a, 'a>,
    specs: &'a SpecsResponseFull,
    id_list_resolution: IdListResolution<'a>,
    hashing: &'a HashUtil,
    override_adapter: Option<&'a Arc<dyn OverrideAdapter>>,
) -> EvaluatorContext<'a> {
    EvaluatorContext::new(
        user_internal,
        specs,
        id_list_resolution,
        hashing,
        None,
        override_adapter,
        false,
        None,
        true,
    )
}

pub fn generate_v1_response<'a>(
    user: &'a StatsigUser,
    specs: &'a SpecsResponseFull,
    id_list_resolution: IdListResolution<'a>,
    override_adapter: Option<&'a Arc<dyn OverrideAdapter>>,
) -> Result<InitializeResponse, StatsigErr> {
    let user_internal = StatsigUserInternal {
        user_ref: user,
        statsig_instance: None,
    };
    let hashing = HashUtil::new();
    let mut context = evaluator_context(
        &user_internal,
        specs,
        id_list_resolution,
        &hashing,
        override_adapter,
    );

    GCIRFormatter::generate_v1_format(&mut context, &client_init_response_options())
}
