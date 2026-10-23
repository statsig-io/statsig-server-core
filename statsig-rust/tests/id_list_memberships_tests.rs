mod utils;

use crate::utils::helpers::load_contents;
use crate::utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{
    evaluation::{
        evaluator::{Evaluator, SpecType},
        evaluator_context::{EvaluatorContext, IdListResolution},
    },
    hashing::HashUtil,
    specs_response::spec_types::{ConfigMapping, Rule, SpecsResponseFull},
    user::StatsigUserInternal,
    ClientInitResponseOptions, EvaluationDetails, HashAlgorithm, SpecsSource, Statsig,
    StatsigOptions, StatsigUser, StatsigUserBuilder,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const COMPANY_LOOKUP: &str = "pmWkWSBC";
const COMPANY_LIST: &str = "company_id_list";

async fn setup() -> Statsig {
    let options = StatsigOptions {
        specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
            "tests/data/eval_proj_dcs.json",
        ))),
        event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
        ..Default::default()
    };
    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();
    statsig
}

fn enriched_user(memberships: HashMap<String, HashSet<String>>) -> StatsigUser {
    StatsigUserBuilder::new_with_user_id("dan_1")
        .custom_ids(Some([("companyID", "123")]))
        .id_list_memberships(memberships)
        .build()
}

#[tokio::test]
async fn enriched_memberships_drive_normal_and_gcir_evaluation() {
    let statsig = setup().await;
    let user = enriched_user(HashMap::from([(
        COMPANY_LOOKUP.to_string(),
        HashSet::from([COMPANY_LIST.to_string()]),
    )]));

    let gate = statsig.get_feature_gate(&user, "test_id_list");
    assert!(gate.value);
    assert!(gate.details.reason.ends_with(":IdListEnriched"));

    let response = statsig.get_client_init_response_with_options(
        &user,
        &ClientInitResponseOptions {
            hash_algorithm: Some(HashAlgorithm::None),
            ..Default::default()
        },
    );
    assert!(response.feature_gates["test_id_list"].value);
    statsig.shutdown().await.unwrap();
}

#[tokio::test]
async fn unenriched_user_uses_in_memory_id_lists() {
    let statsig = setup().await;
    let mut user = StatsigUser::with_user_id("dan_1");
    user.set_custom_ids(HashMap::from([("companyID", "123")]));

    let gate = statsig.get_feature_gate(&user, "test_id_list");
    assert!(!gate.value);
    assert!(gate.details.reason.ends_with(":IdListInMemory"));
    statsig.shutdown().await.unwrap();
}

#[test]
fn memberships_do_not_change_serialization_or_dedupe_hashes() {
    let plain = StatsigUserBuilder::new_with_user_id("dan_1")
        .custom_ids(Some([("companyID", "123")]))
        .build();
    let enriched = enriched_user(HashMap::from([(
        COMPANY_LOOKUP.to_string(),
        HashSet::from([COMPANY_LIST.to_string()]),
    )]));

    assert_eq!(plain.data.to_bytes(), enriched.data.to_bytes());
    assert_eq!(
        plain
            .data
            .create_exposure_dedupe_user_hash(Some("companyID")),
        enriched
            .data
            .create_exposure_dedupe_user_hash(Some("companyID"))
    );
}

#[test]
fn authoritative_memberships_respect_operator_inversion() {
    let original = load_contents("eval_proj_dcs.json");
    let inverted = original.replace("\"in_segment_list\"", "\"not_in_segment_list\"");
    let specs: SpecsResponseFull = serde_json::from_str(&inverted).unwrap();
    let no_in_memory_lists = HashMap::new();

    let member = enriched_user(HashMap::from([(
        COMPANY_LOOKUP.to_string(),
        HashSet::from([COMPANY_LIST.to_string()]),
    )]));
    assert!(!evaluate_gate(
        &specs,
        &member,
        IdListResolution::MapLookup(&no_in_memory_lists),
    ));

    let non_member = enriched_user(HashMap::new());
    assert!(evaluate_gate(
        &specs,
        &non_member,
        IdListResolution::MapLookup(&no_in_memory_lists),
    ));
}

#[test]
fn memberships_are_keyed_by_the_conditions_id_type_lookup() {
    let specs: SpecsResponseFull =
        serde_json::from_str(&load_contents("eval_proj_dcs.json")).unwrap();
    let no_in_memory_lists = HashMap::new();
    let user_lookup: String = HashUtil::new().sha256("dan_1").chars().take(8).collect();

    let wrong_id_type = enriched_user(HashMap::from([(
        user_lookup,
        HashSet::from([COMPANY_LIST.to_string()]),
    )]));
    assert!(!evaluate_gate(
        &specs,
        &wrong_id_type,
        IdListResolution::MapLookup(&no_in_memory_lists),
    ));

    let custom_id_type = enriched_user(HashMap::from([(
        COMPANY_LOOKUP.to_string(),
        HashSet::from([COMPANY_LIST.to_string()]),
    )]));
    assert!(evaluate_gate(
        &specs,
        &custom_id_type,
        IdListResolution::MapLookup(&no_in_memory_lists),
    ));
}

#[test]
fn config_mapping_keeps_id_list_provenance_for_a_destination_without_id_lists() {
    let mut specs: SpecsResponseFull =
        serde_json::from_str(&load_contents("eval_proj_dcs.json")).unwrap();
    specs.override_rules = Some(HashMap::from([(
        "enriched_mapping".to_string(),
        mapping_rule(100.0, &["3614985732"]),
    )]));
    specs.overrides = Some(HashMap::from([(
        "test_id_list".to_string(),
        vec![config_mapping("enriched_mapping", "segment:user_id_list")],
    )]));

    let user = enriched_user(HashMap::from([(
        COMPANY_LOOKUP.to_string(),
        HashSet::from([COMPANY_LIST.to_string()]),
    )]));
    let (value, reason) = evaluate_test_gate(&specs, &user);
    assert!(value);
    assert!(
        reason.ends_with(":IdListEnriched"),
        "mapping source was dropped: {reason}"
    );
}

#[test]
fn nested_gate_memo_restores_id_list_provenance_after_discarded_mapping() {
    let mut specs: SpecsResponseFull =
        serde_json::from_str(&load_contents("eval_proj_dcs.json")).unwrap();
    specs.override_rules = Some(HashMap::from([(
        "discarded_mapping".to_string(),
        mapping_rule(0.0, &["2586864920"]),
    )]));
    specs.overrides = Some(HashMap::from([(
        "test_id_list".to_string(),
        vec![config_mapping("discarded_mapping", "segment:user_id_list")],
    )]));

    let user = enriched_user(HashMap::from([(
        COMPANY_LOOKUP.to_string(),
        HashSet::from([COMPANY_LIST.to_string()]),
    )]));
    let (value, reason) = evaluate_test_gate(&specs, &user);
    assert!(value);
    assert!(
        reason.ends_with(":IdListEnriched"),
        "cached nested gate dropped its source: {reason}"
    );
}

fn mapping_rule(pass_percentage: f64, conditions: &[&str]) -> Rule {
    serde_json::from_value(serde_json::json!({
        "name": "mapping_rule",
        "passPercentage": pass_percentage,
        "conditions": conditions,
        "returnValue": true,
        "id": "mapping_rule",
        "idType": "userID"
    }))
    .unwrap()
}

fn config_mapping(rule_name: &str, destination: &str) -> ConfigMapping {
    serde_json::from_value(serde_json::json!({
        "new_config_name": destination,
        "rules": [{ "rule_name": rule_name, "start_time": 0 }]
    }))
    .unwrap()
}

fn evaluate_test_gate(specs: &SpecsResponseFull, user: &StatsigUser) -> (bool, String) {
    let no_in_memory_lists = HashMap::new();
    let internal = StatsigUserInternal {
        user_ref: user,
        statsig_instance: None,
    };
    let hashing = HashUtil::new();
    let mut context = EvaluatorContext::new(
        &internal,
        specs,
        IdListResolution::MapLookup(&no_in_memory_lists),
        &hashing,
        None,
        None,
        false,
        None,
        true,
    );
    Evaluator::evaluate(&mut context, "test_id_list", &SpecType::Gate).unwrap();
    let reason =
        EvaluationDetails::recognized(&SpecsSource::Network, 0, None, &context.result).reason;
    (context.result.bool_value, reason)
}

fn evaluate_gate(
    specs: &SpecsResponseFull,
    user: &StatsigUser,
    resolver: IdListResolution<'_>,
) -> bool {
    let internal = StatsigUserInternal {
        user_ref: user,
        statsig_instance: None,
    };
    let hashing = HashUtil::new();
    let mut context = EvaluatorContext::new(
        &internal, specs, resolver, &hashing, None, None, false, None, true,
    );
    Evaluator::evaluate(&mut context, "test_id_list", &SpecType::Gate).unwrap();
    context.result.bool_value
}
