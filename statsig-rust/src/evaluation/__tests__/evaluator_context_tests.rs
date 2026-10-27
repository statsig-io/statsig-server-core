use std::collections::HashMap;

use crate::{
    evaluation::evaluator_context::{EvaluatorContext, IdListResolution},
    hashing::HashUtil,
    interned_string::InternedString,
    specs_response::spec_types::SpecsResponseFull,
    user::StatsigUserInternal,
    SecondaryExposure, StatsigErr, StatsigUser,
};

fn evaluate_isolated_recursively(
    context: &mut EvaluatorContext,
    remaining_levels: u16,
) -> Result<(), StatsigErr> {
    if remaining_levels == 0 {
        return Ok(());
    }

    context
        .evaluate_with_isolated_result(|child| {
            evaluate_isolated_recursively(child, remaining_levels - 1)
        })
        .map(|_| ())
}

fn evaluate_isolated_with_result_resets_recursively(
    context: &mut EvaluatorContext,
    remaining_levels: u16,
) -> Result<(), StatsigErr> {
    if remaining_levels == 0 {
        return Ok(());
    }

    context
        .evaluate_with_isolated_result(|child| {
            child.reset_result();
            evaluate_isolated_with_result_resets_recursively(child, remaining_levels - 1)
        })
        .map(|_| ())
}

#[test]
fn isolated_evaluation_restores_parent_result_and_recursive_depth() {
    let user = StatsigUser::with_user_id("user");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let specs = SpecsResponseFull::default();
    let id_lists = HashMap::new();
    let hashing = HashUtil::new();
    let mut context = EvaluatorContext::new(
        &user_internal,
        &specs,
        IdListResolution::MapLookup(&id_lists),
        &hashing,
        None,
        None,
        false,
        None,
        true,
    );
    context.result.name = Some(InternedString::from_str_ref("outer"));
    context.result.rule_id = Some(InternedString::from_str_ref("outer-rule"));
    let outer_exposure = SecondaryExposure {
        gate: InternedString::from_str_ref("outer-gate"),
        gate_value: InternedString::from_str_ref("true"),
        rule_id: InternedString::from_str_ref("outer-rule"),
    };
    context.result.secondary_exposures = vec![outer_exposure.clone()];
    context.result.undelegated_secondary_exposures = Some(vec![outer_exposure.clone()]);

    let (_, child_result) = context
        .evaluate_with_isolated_result(|child| {
            assert_eq!(child.nested_count, 1);
            child.result.name = Some(InternedString::from_str_ref("child"));
            Ok(())
        })
        .unwrap();

    assert_eq!(child_result.name.as_deref(), Some("child"));
    assert_eq!(context.nested_count, 0);
    assert_eq!(context.result.name.as_deref(), Some("outer"));
    assert_eq!(context.result.rule_id.as_deref(), Some("outer-rule"));
    assert_eq!(context.result.secondary_exposures.len(), 1);
    assert_eq!(
        context.result.secondary_exposures[0].gate.as_str(),
        outer_exposure.gate.as_str()
    );
    let undelegated = context
        .result
        .undelegated_secondary_exposures
        .as_ref()
        .expect("parent undelegated exposures should be restored");
    assert_eq!(undelegated.len(), 1);
    assert_eq!(undelegated[0].gate.as_str(), outer_exposure.gate.as_str());

    let error = context.evaluate_with_isolated_result::<()>(|child| {
        child.result.secondary_exposures.clear();
        child.result.undelegated_secondary_exposures = None;
        child.result.name = Some(InternedString::from_str_ref("failed-child"));
        Err(StatsigErr::StackOverflowError)
    });
    assert!(matches!(error, Err(StatsigErr::StackOverflowError)));
    assert_eq!(context.nested_count, 0);
    assert_eq!(context.result.name.as_deref(), Some("outer"));
    assert_eq!(context.result.rule_id.as_deref(), Some("outer-rule"));
    assert_eq!(context.result.secondary_exposures.len(), 1);
    assert_eq!(
        context.result.secondary_exposures[0].gate.as_str(),
        outer_exposure.gate.as_str()
    );
    let undelegated = context
        .result
        .undelegated_secondary_exposures
        .as_ref()
        .expect("parent undelegated exposures should be restored after error");
    assert_eq!(undelegated.len(), 1);
    assert_eq!(undelegated[0].gate.as_str(), outer_exposure.gate.as_str());
}

#[test]
fn isolated_evaluation_enforces_recursive_depth_limit() {
    let user = StatsigUser::with_user_id("user");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let specs = SpecsResponseFull::default();
    let id_lists = HashMap::new();
    let hashing = HashUtil::new();
    let mut context = EvaluatorContext::new(
        &user_internal,
        &specs,
        IdListResolution::MapLookup(&id_lists),
        &hashing,
        None,
        None,
        false,
        None,
        true,
    );

    assert!(evaluate_isolated_recursively(&mut context, 64).is_ok());
    assert_eq!(context.nested_count, 0);

    assert!(matches!(
        evaluate_isolated_recursively(&mut context, 65),
        Err(StatsigErr::StackOverflowError)
    ));
    assert_eq!(context.nested_count, 0);
}

#[test]
fn isolated_evaluation_depth_survives_result_resets() {
    let user = StatsigUser::with_user_id("user");
    let user_internal = StatsigUserInternal {
        user_ref: &user,
        statsig_instance: None,
    };
    let specs = SpecsResponseFull::default();
    let id_lists = HashMap::new();
    let hashing = HashUtil::new();
    let mut context = EvaluatorContext::new(
        &user_internal,
        &specs,
        IdListResolution::MapLookup(&id_lists),
        &hashing,
        None,
        None,
        false,
        None,
        true,
    );
    context.result.name = Some(InternedString::from_str_ref("outer"));

    assert!(evaluate_isolated_with_result_resets_recursively(&mut context, 64).is_ok());
    assert_eq!(context.nested_count, 0);
    assert_eq!(context.result.name.as_deref(), Some("outer"));

    assert!(matches!(
        evaluate_isolated_with_result_resets_recursively(&mut context, 65),
        Err(StatsigErr::StackOverflowError)
    ));
    assert_eq!(context.nested_count, 0);
    assert_eq!(context.result.name.as_deref(), Some("outer"));
}
