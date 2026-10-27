mod utils;

use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
};

use serde_json::json;
use statsig_rust::{
    evaluation::{
        evaluator::{Evaluator, SpecType},
        evaluator_context::IdListResolution,
    },
    hashing::HashUtil,
    interned_string::InternedString,
    specs_response::spec_types::{ConfigMapping, OverrideRule, Rule},
    StatsigUserBuilder,
};

use crate::utils::{
    gcir::evaluator_context,
    gcir_experiment_group_fixtures::{
        add_condition, add_config, add_gate, add_layer, experiment_group_specs, gate_condition,
        rule, test_user, unit_condition,
    },
};

#[test]
fn nested_gate_isolation_preserves_id_list_sources_on_fresh_memo_and_unsupported_paths() {
    for enriched in [false, true] {
        for (child_has_list, unsupported) in
            [(true, false), (true, true), (false, false), (false, true)]
        {
            let mut specs = experiment_group_specs();
            let mut id_condition = unit_condition(&[], "userID");
            id_condition["operator"] = json!("in_segment_list");
            id_condition["targetValue"] = json!("test-list");
            add_condition(&mut specs, "list-condition", id_condition);
            let mut invalid = unit_condition(&[], "userID");
            invalid["operator"] = json!("unsupported_operator");
            add_condition(&mut specs, "invalid-condition", invalid);

            let mut child_conditions = Vec::new();
            if child_has_list {
                child_conditions.push("list-condition");
            }
            if unsupported {
                child_conditions.push("invalid-condition");
            }
            let mut child_rule = rule("child-rule", &child_conditions, json!(true), None);
            child_rule["samplingRate"] = json!(201);
            add_gate(&mut specs, "child", "feature_gate", json!([child_rule]));
            add_condition(
                &mut specs,
                "child-condition",
                gate_condition("pass_gate", "child"),
            );
            let mut parent_conditions = Vec::new();
            if !child_has_list {
                parent_conditions.push("list-condition");
            }
            // The second reference must use the sampled child's memo, even
            // when the child itself has no provenance to restore.
            parent_conditions.extend(["child-condition", "child-condition"]);
            add_gate(
                &mut specs,
                "parent",
                "feature_gate",
                json!([rule("parent-rule", &parent_conditions, json!(true), None)]),
            );

            let hashing = HashUtil::new();
            let lookup: String = hashing.sha256("eligible").chars().take(8).collect();
            let mut builder = StatsigUserBuilder::new_with_user_id("eligible");
            if enriched {
                builder = builder.id_list_memberships(HashMap::from([(
                    lookup.clone(),
                    HashSet::from(["test-list".to_owned()]),
                )]));
            }
            let user = builder.build();
            let internal = statsig_rust::user::StatsigUserInternal {
                user_ref: &user,
                statsig_instance: None,
            };
            let calls = Cell::new(0);
            let resolver = |list: &str, id: &str| {
                calls.set(calls.get() + 1);
                assert_eq!(list, "test-list");
                assert_eq!(id, lookup);
                true
            };
            let mut ctx = evaluator_context(
                &internal,
                &specs,
                IdListResolution::Callback(&resolver),
                &hashing,
                None,
            );
            Evaluator::evaluate(&mut ctx, "parent", &SpecType::Gate).unwrap();
            assert_eq!(
                ctx.result.id_list_source,
                Some(if enriched {
                    "IdListEnriched"
                } else {
                    "IdListInMemory"
                }),
                "enriched={enriched}, child_has_list={child_has_list}, unsupported={unsupported}"
            );
            assert_eq!(ctx.result.unsupported, unsupported);
            if !unsupported {
                assert!(ctx.result.bool_value);
                assert_eq!(ctx.result.rule_id.as_deref(), Some("parent-rule"));
                assert!(ctx
                    .nested_gate_memo
                    .contains_key(&InternedString::from_str_ref("child")));
            }
            // Unsupported children are intentionally not memoized, so both
            // references evaluate their ID-list condition independently.
            let expected_calls = if enriched {
                0
            } else if child_has_list && unsupported {
                2
            } else {
                1
            };
            assert_eq!(
                calls.get(),
                expected_calls,
                "enriched={enriched}, child_has_list={child_has_list}, unsupported={unsupported}"
            );
        }
    }
}

#[test]
fn nested_gate_isolation_preserves_results_exposures_memo_and_layer_failures() {
    let mut specs = experiment_group_specs();

    let mut sampled_health_rule = rule(
        "sampled-health-rule",
        &["health-condition"],
        json!(true),
        None,
    );
    sampled_health_rule["samplingRate"] = json!(201);
    add_gate(
        &mut specs,
        "sampled-health-gate",
        "feature_gate",
        json!([sampled_health_rule]),
    );
    add_condition(
        &mut specs,
        "memo-health-one",
        gate_condition("pass_gate", "sampled-health-gate"),
    );
    add_condition(
        &mut specs,
        "memo-health-two",
        gate_condition("pass_gate", "sampled-health-gate"),
    );
    add_gate(
        &mut specs,
        "memo-parent",
        "feature_gate",
        json!([rule(
            "memo-parent-rule",
            &["memo-health-one", "memo-health-two"],
            json!(true),
            None
        )]),
    );

    add_condition(
        &mut specs,
        "analytical-segment-health",
        gate_condition("pass_gate", "health-gate"),
    );
    add_gate(
        &mut specs,
        "segment:analytical-parent",
        "segment",
        json!([rule(
            "analytical-segment-rule",
            &["analytical-segment-health"],
            json!(true),
            None
        )]),
    );
    add_condition(
        &mut specs,
        "analytical-segment-parent-condition",
        gate_condition("pass_gate", "segment:analytical-parent"),
    );
    let mut analytical_segment_parent_rule = rule(
        "analytical-segment-parent-rule",
        &["analytical-segment-parent-condition"],
        json!({ "value": "analytical-segment" }),
        Some("analytical-segment"),
    );
    analytical_segment_parent_rule["samplingRate"] = json!(201);
    add_config(
        &mut specs,
        "analytical-segment-parent",
        json!([analytical_segment_parent_rule]),
    );

    add_condition(
        &mut specs,
        "mapped-parent-health",
        gate_condition("pass_gate", "sampled-health-gate"),
    );
    add_config(&mut specs, "mapped-original", json!([]));
    add_config(
        &mut specs,
        "mapped-target",
        json!([rule(
            "mapped-target-rule",
            &["mapped-parent-health"],
            json!({ "value": "mapped" }),
            None
        )]),
    );
    specs.override_rules = Some(HashMap::from([(
        "mapping-rule".to_owned(),
        serde_json::from_value::<Rule>(rule("mapping-rule", &[], json!(true), None)).unwrap(),
    )]));
    specs.overrides = Some(HashMap::from([(
        "mapped-original".to_owned(),
        vec![ConfigMapping {
            new_config_name: InternedString::from_str_ref("mapped-target"),
            rules: vec![OverrideRule {
                rule_name: "mapping-rule".to_owned(),
                start_time: None,
            }],
        }],
    )]));

    let mut unsupported_condition = unit_condition(&["eligible"], "userID");
    unsupported_condition["operator"] = json!("unsupported_operator");
    add_condition(&mut specs, "unsupported-condition", unsupported_condition);
    add_condition(
        &mut specs,
        "unsupported-child-health",
        gate_condition("pass_gate", "health-gate"),
    );
    add_gate(
        &mut specs,
        "unsupported-gate",
        "feature_gate",
        json!([rule(
            "unsupported-rule",
            &["unsupported-child-health", "unsupported-condition"],
            json!(true),
            None
        )]),
    );
    add_condition(
        &mut specs,
        "parent-health",
        gate_condition("pass_gate", "sampled-health-gate"),
    );
    add_condition(
        &mut specs,
        "parent-unsupported",
        gate_condition("pass_gate", "unsupported-gate"),
    );
    add_gate(
        &mut specs,
        "unsupported-parent",
        "feature_gate",
        json!([rule(
            "unsupported-parent-rule",
            &["parent-health", "parent-unsupported"],
            json!(true),
            None
        )]),
    );
    add_layer(
        &mut specs,
        "unsupported-layer",
        json!([rule(
            "unsupported-layer-rule",
            &["parent-health", "parent-unsupported"],
            json!({ "layer": "value" }),
            None
        )]),
    );

    let id_list_resolver = |_: &str, _: &str| false;
    let hashing = HashUtil::new();
    let eligible = test_user("eligible", "profile-control");
    let eligible_internal = statsig_rust::user::StatsigUserInternal {
        user_ref: &eligible,
        statsig_instance: None,
    };

    let mut memo_ctx = evaluator_context(
        &eligible_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    Evaluator::evaluate(&mut memo_ctx, "memo-parent", &SpecType::Gate).unwrap();
    assert!(memo_ctx.result.bool_value);
    assert!(!memo_ctx.result.unsupported);
    assert_eq!(memo_ctx.result.secondary_exposures.len(), 2);
    assert!(memo_ctx
        .result
        .secondary_exposures
        .iter()
        .all(|exposure| exposure.gate.as_str() == "sampled-health-gate"));
    assert_eq!(memo_ctx.nested_gate_memo.len(), 1);
    assert_ne!(memo_ctx.result.has_seen_analytical_gates, Some(true));

    let mut analytical_segment_ctx = evaluator_context(
        &eligible_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    Evaluator::evaluate(
        &mut analytical_segment_ctx,
        "analytical-segment-parent",
        &SpecType::Experiment,
    )
    .unwrap();
    assert!(analytical_segment_ctx.result.bool_value);
    assert_eq!(analytical_segment_ctx.result.sampling_rate, Some(201));
    assert_eq!(
        analytical_segment_ctx.result.has_seen_analytical_gates,
        Some(true)
    );
    assert!(analytical_segment_ctx.nested_gate_memo.is_empty());

    let mut mapped_ctx = evaluator_context(
        &eligible_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    Evaluator::evaluate(&mut mapped_ctx, "mapped-original", &SpecType::Experiment).unwrap();
    assert_eq!(
        mapped_ctx.result.override_config_name.as_deref(),
        Some("mapped-target")
    );

    let ineligible = test_user("ineligible", "profile-other");
    let ineligible_internal = statsig_rust::user::StatsigUserInternal {
        user_ref: &ineligible,
        statsig_instance: None,
    };
    let mut false_ctx = evaluator_context(
        &ineligible_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    Evaluator::evaluate(
        &mut false_ctx,
        "segment:inline_targeting:gate-only",
        &SpecType::Gate,
    )
    .unwrap();
    assert!(!false_ctx.result.bool_value);
    assert!(!false_ctx.result.unsupported);
    assert_eq!(false_ctx.result.rule_id.as_deref(), Some("default"));
    assert_eq!(false_ctx.result.secondary_exposures.len(), 1);
    assert_eq!(
        false_ctx.result.secondary_exposures[0].gate_value.as_str(),
        "false"
    );

    let mut unsupported_ctx = evaluator_context(
        &eligible_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    Evaluator::evaluate(&mut unsupported_ctx, "unsupported-parent", &SpecType::Gate).unwrap();
    assert!(unsupported_ctx.result.unsupported);
    assert!(unsupported_ctx.result.bool_value);
    assert_eq!(
        unsupported_ctx.result.rule_id.as_deref(),
        Some("sampled-health-rule")
    );
    assert_eq!(unsupported_ctx.result.has_seen_analytical_gates, Some(true));
    assert_eq!(unsupported_ctx.result.secondary_exposures.len(), 2);
    assert_eq!(
        unsupported_ctx.result.secondary_exposures[0].gate.as_str(),
        "sampled-health-gate"
    );
    assert_eq!(
        unsupported_ctx.result.secondary_exposures[1].gate.as_str(),
        "health-gate"
    );

    let mut layer_ctx = evaluator_context(
        &eligible_internal,
        &specs,
        IdListResolution::Callback(&id_list_resolver),
        &hashing,
        None,
    );
    Evaluator::evaluate(&mut layer_ctx, "unsupported-layer", &SpecType::Layer).unwrap();
    assert!(layer_ctx.result.unsupported);
    assert!(layer_ctx.result.bool_value);
    assert_eq!(layer_ctx.result.secondary_exposures.len(), 2);
    assert_eq!(
        layer_ctx.result.secondary_exposures[0].gate.as_str(),
        "sampled-health-gate"
    );
    assert_eq!(
        layer_ctx.result.secondary_exposures[1].gate.as_str(),
        "health-gate"
    );
    let undelegated = layer_ctx
        .result
        .undelegated_secondary_exposures
        .as_ref()
        .expect("layer should preserve undelegated exposures");
    assert_eq!(undelegated.len(), 2);
    assert_eq!(
        undelegated[0].gate.as_str(),
        layer_ctx.result.secondary_exposures[0].gate.as_str()
    );
}
