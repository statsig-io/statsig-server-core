mod utils;

use std::{collections::HashMap, sync::Arc};

use rusty_fork::rusty_fork_test;
use serde_json::{json, Value};
use statsig_rust::{
    evaluation::evaluator_context::IdListResolution,
    gcir::gcir_formatter::GCIRFormatter,
    hashing::HashUtil,
    specs_response::spec_types::{ConfigMapping, OverrideRule, Rule, SpecsResponseFull},
    user::StatsigUserInternal,
    ClientInitResponseOptions, HashAlgorithm, OverrideAdapter, Statsig,
    StatsigLocalOverrideAdapter, StatsigOptions,
};

use crate::utils::{
    gcir::{client_init_response_options, evaluator_context},
    gcir_experiment_group_fixtures::{
        add_condition, add_config, add_gate, add_target_experiment, experiment_group_condition,
        experiment_group_specs, rule, test_user, unit_condition,
    },
    mock_event_logging_adapter::MockEventLoggingAdapter,
    mock_specs_adapter::MockSpecsAdapter,
};

fn assert_v1_experiment(response: &Value, name: &str, assigned: bool) {
    let experiment = &response["dynamic_configs"][name];
    if assigned {
        assert_eq!(experiment["value"], json!({ "payload": "treatment" }));
        assert_eq!(experiment["rule_id"], format!("{name}-treatment-rule"));
        assert_eq!(experiment["group_name"], "treatment");
        assert_eq!(experiment["is_experiment_active"], true);
        assert_eq!(experiment["is_user_in_experiment"], true);
        assert_eq!(experiment["id_type"], "userID");
    } else {
        assert_eq!(experiment["value"], json!({}));
        assert_eq!(experiment["rule_id"], "inlineTargetingRules");
        assert!(experiment.get("group_name").is_none());
        assert_eq!(experiment["is_user_in_experiment"], false);
    }
}

fn assert_health_secondary_exposure(response: &Value, name: &str, value: bool) {
    assert_eq!(
        response["dynamic_configs"][name]["secondary_exposures"],
        json!([{
            "gate": "health-gate",
            "gateValue": value.to_string(),
            "ruleID": if value { "health-rule" } else { "default" }
        }])
    );
}

#[test]
fn gcir_v1_evaluates_gate_only_group_only_and_combined_inline_targeting() {
    let specs = experiment_group_specs();
    let id_list_resolver = |_: &str, _: &str| false;
    let hashing = HashUtil::new();

    for (user, assigned) in [
        (test_user("eligible", "profile-control"), true),
        (test_user("eligible", "profile-other"), false),
    ] {
        let user_internal = StatsigUserInternal {
            user_ref: &user,
            statsig_instance: None,
        };
        let mut ctx = evaluator_context(
            &user_internal,
            &specs,
            IdListResolution::Callback(&id_list_resolver),
            &hashing,
            None,
        );
        let response = serde_json::to_value(
            GCIRFormatter::generate_v1_format(&mut ctx, &client_init_response_options()).unwrap(),
        )
        .unwrap();

        assert_v1_experiment(&response, "group-only", assigned);
        assert_v1_experiment(&response, "combined", assigned);
        assert_v1_experiment(&response, "gate-only", assigned);
        assert_eq!(
            response["dynamic_configs"]["group-only"]["secondary_exposures"],
            json!([])
        );
        assert_health_secondary_exposure(&response, "combined", assigned);
        assert_health_secondary_exposure(&response, "gate-only", assigned);

        let health = &response["feature_gates"]["health-gate"];
        assert_eq!(health["value"], assigned);
    }
}

#[test]
fn gcir_id_override_precedes_inline_targeting_without_marking_enrollment() {
    let specs = experiment_group_specs();
    let id_list_resolver = |_: &str, _: &str| false;
    let hashing = HashUtil::new();
    let user = test_user("not-eligible", "profile-forced");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let mut ctx = evaluator_context(
        &user_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    let response = serde_json::to_value(
        GCIRFormatter::generate_v1_format(&mut ctx, &client_init_response_options()).unwrap(),
    )
    .unwrap();

    for name in ["group-only", "combined", "gate-only"] {
        let experiment = &response["dynamic_configs"][name];
        assert_eq!(
            experiment["value"],
            json!({ "payload": "forced-treatment" })
        );
        assert_eq!(
            experiment["rule_id"],
            format!("{name}-treatment-rule:profileId:id_override")
        );
        assert_eq!(experiment["group_name"], "treatment");
        assert_eq!(experiment["is_experiment_active"], true);
        assert_eq!(experiment["is_user_in_experiment"], false);
        assert_eq!(experiment["id_type"], "userID");
        assert_eq!(experiment["secondary_exposures"], json!([]));
    }
}

#[test]
fn gcir_isolated_evaluation_honors_only_included_local_overrides() {
    let specs = experiment_group_specs();
    let id_list_resolver = |_: &str, _: &str| false;
    let hashing = HashUtil::new();
    let user = test_user("eligible", "profile-control");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let adapter = Arc::new(StatsigLocalOverrideAdapter::new());
    adapter.override_experiment_by_group_name("reference-experiment", "test", None);
    let adapter: Arc<dyn OverrideAdapter> = adapter;

    let mut excluded_ctx = evaluator_context(
        &user_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    let excluded = serde_json::to_value(
        GCIRFormatter::generate_v1_format(&mut excluded_ctx, &client_init_response_options())
            .unwrap(),
    )
    .unwrap();
    assert_v1_experiment(&excluded, "group-only", true);
    assert_v1_experiment(&excluded, "combined", true);
    assert_v1_experiment(&excluded, "gate-only", true);

    let mut included_ctx = evaluator_context(
        &user_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        Some(&adapter),
    );
    let included = serde_json::to_value(
        GCIRFormatter::generate_v1_format(&mut included_ctx, &client_init_response_options())
            .unwrap(),
    )
    .unwrap();
    assert_v1_experiment(&included, "group-only", false);
    assert_v1_experiment(&included, "combined", false);
    assert_v1_experiment(&included, "gate-only", true);
}

#[test]
fn gcir_unsupported_referenced_experiment_does_not_contaminate_outer_result() {
    let mut specs = experiment_group_specs();
    add_config(
        &mut specs,
        "unsupported-reference",
        json!([rule(
            "unsupported-reference-rule",
            &["condition-that-does-not-exist"],
            json!({ "reference": "control" }),
            Some("control")
        )]),
    );
    add_condition(
        &mut specs,
        "unsupported-group-condition",
        experiment_group_condition("unsupported-reference", "control"),
    );
    add_gate(
        &mut specs,
        "segment:inline_targeting:unsupported-target",
        "segment",
        json!([rule(
            "unsupported-segment-rule",
            &["unsupported-group-condition"],
            json!(true),
            None
        )]),
    );
    add_target_experiment(
        &mut specs,
        "unsupported-target",
        "segment:inline_targeting:unsupported-target",
    );

    let id_list_resolver = |_: &str, _: &str| false;
    let hashing = HashUtil::new();
    let user = test_user("eligible", "profile-control");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let mut ctx = evaluator_context(
        &user_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    let response = serde_json::to_value(
        GCIRFormatter::generate_v1_format(&mut ctx, &client_init_response_options()).unwrap(),
    )
    .unwrap();

    assert_v1_experiment(&response, "unsupported-target", false);
    assert_eq!(
        response["dynamic_configs"]["unsupported-target"]["name"],
        "unsupported-target"
    );
}

#[test]
fn gcir_v2_and_init_v2_preserve_experiment_assignment_fields() {
    let specs = experiment_group_specs();
    let id_list_resolver = |_: &str, _: &str| false;
    let hashing = HashUtil::new();
    let user = test_user("eligible", "profile-control");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };

    let mut v2_ctx = evaluator_context(
        &user_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    let v2 = serde_json::to_value(
        GCIRFormatter::generate_v2_format(&mut v2_ctx, &client_init_response_options()).unwrap(),
    )
    .unwrap();
    assert_v1_experiment(&v2, "group-only", true);
    assert_v1_experiment(&v2, "combined", true);
    let v2_exposure_key = v2["dynamic_configs"]["combined"]["secondary_exposures"][0]
        .as_str()
        .expect("combined evaluation should reference its health-gate exposure");
    assert_eq!(
        v2["exposures"][v2_exposure_key],
        json!({
            "gate": "health-gate",
            "gateValue": "true",
            "ruleID": "health-rule"
        })
    );

    let mut init_v2_ctx = evaluator_context(
        &user_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    let init_v2 = serde_json::to_value(
        GCIRFormatter::generate_init_v2_format(&mut init_v2_ctx, &client_init_response_options())
            .unwrap(),
    )
    .unwrap();
    for name in ["group-only", "combined"] {
        let experiment = &init_v2["dynamic_configs"][name];
        assert_eq!(experiment["gn"], "treatment");
        assert_eq!(experiment["ea"], true);
        assert_eq!(experiment["ue"], true);
        assert_eq!(experiment["i"], "userID");
        assert_eq!(experiment["r"], format!("{name}-treatment-rule"));
        let value_key = experiment["v"].as_str().unwrap();
        assert_eq!(
            init_v2["values"][value_key],
            json!({ "payload": "treatment" })
        );
    }
    let init_v2_exposure_key = init_v2["dynamic_configs"]["combined"]["s"][0]
        .as_str()
        .expect("combined init-v2 evaluation should reference its health-gate exposure");
    assert_eq!(
        init_v2["exposures"][init_v2_exposure_key],
        json!({
            "gate": "health-gate",
            "gateValue": "true",
            "ruleID": "health-rule"
        })
    );
}

#[test]
fn gcir_experiment_group_cycles_resolve_as_no_group_without_overflowing() {
    let mut specs = SpecsResponseFull {
        has_updates: true,
        time: 1,
        ..Default::default()
    };
    add_condition(
        &mut specs,
        "a-references-a",
        experiment_group_condition("cycle-a", "a"),
    );
    add_config(
        &mut specs,
        "cycle-a",
        json!([rule(
            "cycle-a-rule",
            &["a-references-a"],
            json!({}),
            Some("a")
        )]),
    );

    let id_list_resolver = |_: &str, _: &str| false;
    let hashing = HashUtil::new();
    let user = test_user("eligible", "profile-control");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let mut ctx = evaluator_context(
        &user_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );

    let response = serde_json::to_value(
        GCIRFormatter::generate_v1_format(&mut ctx, &client_init_response_options()).unwrap(),
    )
    .unwrap();
    assert!(response["dynamic_configs"].get("cycle-a").is_some());
}

rusty_fork_test! {
#![rusty_fork(timeout_ms = 1_000)]

#[test]
fn gcir_experiment_group_cycles_with_matching_config_mapping_complete_without_overflow() {
    let mut specs = SpecsResponseFull {
        has_updates: true,
        time: 1,
        ..Default::default()
    };
    add_condition(
        &mut specs,
        "a-references-b",
        experiment_group_condition("cycle-b", "b"),
    );
    add_condition(
        &mut specs,
        "b-references-a",
        experiment_group_condition("cycle-a", "a"),
    );
    add_condition(
        &mut specs,
        "matching-mapping-rule-condition",
        unit_condition(&["eligible"], "userID"),
    );
    add_config(
        &mut specs,
        "cycle-a",
        json!([rule(
            "cycle-a-rule",
            &["a-references-b"],
            json!({}),
            Some("a")
        )]),
    );
    add_config(
        &mut specs,
        "cycle-b",
        json!([rule(
            "cycle-b-rule",
            &["b-references-a"],
            json!({}),
            Some("b")
        )]),
    );
    specs.override_rules = Some(HashMap::from([(
        "matching-mapping-rule".to_owned(),
        serde_json::from_value::<Rule>(rule(
            "matching-mapping-rule",
            &["matching-mapping-rule-condition"],
            json!(true),
            None,
        ))
        .unwrap(),
    )]));
    specs.overrides = Some(HashMap::from([(
        "cycle-b".to_owned(),
        vec![ConfigMapping {
            new_config_name: statsig_rust::interned_string::InternedString::from_str_ref(
                "cycle-a",
            ),
            rules: vec![OverrideRule {
                rule_name: "matching-mapping-rule".to_owned(),
                start_time: None,
            }],
        }],
    )]));

    let id_list_resolver = |_: &str, _: &str| false;
    let hashing = HashUtil::new();
    let user = test_user("eligible", "profile-control");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let mut ctx = evaluator_context(
        &user_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );

    let response = serde_json::to_value(
        GCIRFormatter::generate_v1_format(&mut ctx, &client_init_response_options()).unwrap(),
    )
    .unwrap();
    for name in ["cycle-a", "cycle-b"] {
        let config = &response["dynamic_configs"][name];
        assert_eq!(config["value"], json!({}));
        assert_eq!(config["rule_id"], "default");
        assert!(config.get("group_name").is_none());
        assert_eq!(config["is_user_in_experiment"], false);
    }
}
}

#[tokio::test]
async fn public_gcir_response_keeps_unrelated_specs_when_experiment_group_cycles() {
    let mut specs = SpecsResponseFull {
        has_updates: true,
        time: 1,
        ..Default::default()
    };
    add_condition(
        &mut specs,
        "a-references-a",
        experiment_group_condition("cycle-a", "a"),
    );
    add_config(
        &mut specs,
        "cycle-a",
        json!([rule(
            "cycle-a-rule",
            &["a-references-a"],
            json!({}),
            Some("a")
        )]),
    );
    add_gate(
        &mut specs,
        "unrelated-gate",
        "feature_gate",
        json!([rule("unrelated-gate-rule", &[], json!(true), None)]),
    );
    add_config(
        &mut specs,
        "unrelated-config",
        json!([rule(
            "unrelated-config-rule",
            &[],
            json!({"safe": true}),
            None
        )]),
    );
    crate::utils::gcir_experiment_group_fixtures::add_layer(
        &mut specs,
        "unrelated-layer",
        json!([rule(
            "unrelated-layer-rule",
            &[],
            json!({"safe": true}),
            None
        )]),
    );

    let mut options = StatsigOptions::new();
    options.specs_adapter = Some(Arc::new(MockSpecsAdapter::with_json_data(
        serde_json::to_string(&specs).unwrap(),
    )));
    options.event_logging_adapter = Some(Arc::new(MockEventLoggingAdapter::new()));
    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    let response = serde_json::to_value(statsig.get_client_init_response_with_options(
        &test_user("eligible", "profile-control"),
        &ClientInitResponseOptions {
            hash_algorithm: Some(HashAlgorithm::None),
            ..Default::default()
        },
    ))
    .unwrap();
    assert_eq!(
        response["feature_gates"]["unrelated-gate"]["value"], true,
        "response was {response}"
    );
    assert_eq!(
        response["dynamic_configs"]["unrelated-config"]["value"],
        json!({ "safe": true })
    );
    assert_eq!(
        response["layer_configs"]["unrelated-layer"]["value"],
        json!({ "default": "value", "safe": true })
    );
    assert_eq!(response["dynamic_configs"]["cycle-a"]["value"], json!({}));
    assert_eq!(response["dynamic_configs"]["cycle-a"]["rule_id"], "default");
}
