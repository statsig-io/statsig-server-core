mod utils;

use statsig_rust::{
    evaluation::evaluator_context::EvaluatorContext, gcir::gcir_formatter::GCIRFormatter,
    hashing::HashUtil, specs_response::spec_types::SpecsResponseFull, user::StatsigUserInternal,
    ClientInitResponseOptions, HashAlgorithm, StatsigUser,
};

use crate::utils::helpers::load_contents;

#[test]
fn test_gcir() {
    let user = StatsigUser::with_user_id("a_user_id");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let hashing = HashUtil::new();

    let contents = load_contents("eval_proj_dcs.json");
    let specs_data =
        serde_json::from_str::<SpecsResponseFull>(&contents).expect("should parse specs data");

    let mut ctx = EvaluatorContext::new(
        &user_internal,
        &specs_data,
        None,
        &hashing,
        None,
        None,
        false,
    );
    let options = ClientInitResponseOptions {
        hash_algorithm: Some(HashAlgorithm::None),
        ..Default::default()
    };

    let response =
        GCIRFormatter::generate_v1_format(&mut ctx, &options).expect("should have a response");
    let gate = response
        .feature_gates
        .get("test_public")
        .expect("should have a gate");

    assert!(gate.value);
}
