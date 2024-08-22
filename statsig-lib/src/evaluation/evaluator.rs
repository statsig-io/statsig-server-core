use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::spec_types::{Condition, Rule, Spec};
use crate::{dyn_value, log_e, unwrap_or_noop, unwrap_or_return};
use chrono::{Duration, Utc};
use lazy_static::lazy_static;
use crate::evaluation::evaluation_types::SecondaryExposure;

pub struct Evaluator;

lazy_static! {
    static ref EMPTY_STR: String = "".to_string();
    static ref DEFAULT_RULE: String = "default".to_string();
    static ref DISABLED_RULE: String = "disabled".to_string();
    static ref EMPTY_DYNAMIC_VALUE: DynamicValue = DynamicValue::new();
}

impl Evaluator {
    pub fn evaluate<'a>(ctx: &mut EvaluatorContext<'a>, spec: &'a Spec) {
        for rule in spec.rules.iter() {
            evaluate_rule(ctx, rule);

            if ctx.result.unsupported {
                return;
            }

            if !ctx.result.bool_value {
                continue;
            }

            if evaluate_config_delegate(ctx, rule) {
                return;
            }

            let did_pass = evaluate_pass_percentage(ctx, rule, &spec.salt);

            if did_pass {
                ctx.result.bool_value = rule.return_value.string_value == "true";
                ctx.result.json_value = Some(&rule.return_value);
            } else {
                ctx.result.bool_value = spec.default_value.string_value == "true";
                ctx.result.json_value = Some(&spec.default_value);
            }

            ctx.result.finalize_secondary_exposures();
            ctx.result.rule_id = Some(&rule.id);
            ctx.result.group_name = rule.group_name.as_ref();
            ctx.result.is_experiment_group = rule.is_experiment_group.unwrap_or(false);
            return;
        }

        ctx.result.finalize_secondary_exposures();
        ctx.result.bool_value = spec.default_value.string_value == "true";
        ctx.result.json_value = Some(&spec.default_value);
        ctx.result.rule_id = match spec.enabled {
            true => Some(&DEFAULT_RULE),
            false => Some(&DISABLED_RULE),
        };
    }
}



fn evaluate_rule<'a>(ctx: &mut EvaluatorContext<'a>, rule: &'a Rule) {
    let mut all_conditions_pass = true;
    // println!("--- Eval Rule {} ---", rule.id);
    for condition_hash in rule.conditions.iter() {
        // println!("Condition Hash {}", condition_hash);
        let opt_condition = ctx.spec_store_data.values.condition_map.get(condition_hash);
        let condition = match opt_condition {
            Some(c) => c,
            None => {
                // todo: log condition not found error
                ctx.result.unsupported = true;
                return;
            }
        };

        evaluate_condition(ctx, condition);

        if !ctx.result.bool_value {
            all_conditions_pass = false
        }
    }

    ctx.result.bool_value = all_conditions_pass;
}

fn evaluate_condition<'a>(ctx: &mut EvaluatorContext<'a>, condition: &'a Condition) {
    let temp_value;
    let target_value = condition
        .target_value
        .as_ref()
        .unwrap_or(&EMPTY_DYNAMIC_VALUE);
    let condition_type = &condition.condition_type;

    let value = match condition_type as &str {
        "public" => {
            ctx.result.bool_value = true;
            return;
        }
        "fail_gate" | "pass_gate" => {
            evaluate_nested_gate(ctx, target_value, condition_type);
            return;
        }
        "user_field" | "ua_based" | "ip_based" => ctx.user.get_user_value(&condition.field),
        "environment_field" => ctx.user.get_value_from_environment(&condition.field),
        "current_time" => {
            temp_value = DynamicValue::for_timestamp_evaluation(Utc::now().timestamp_millis());
            Some(&temp_value)
        }
        "user_bucket" => {
            temp_value = get_hash_for_user_bucket(ctx, condition);
            Some(&temp_value)
        }
        "unit_id" => ctx.user.get_unit_id(&condition.id_type),
        _ => {
            ctx.result.unsupported = true;
            return;
        }
    }
        .unwrap_or(&EMPTY_DYNAMIC_VALUE);

    // println!("Eval Condition {}, {:?}", condition_type, value);

    let operator = match &condition.operator {
        Some(operator) => operator,
        None => {
            ctx.result.unsupported = true;
            return;
        }
    };

    ctx.result.bool_value = match operator as &str {
        // numerical comparison
        "gt" | "gte" | "lt" | "lte" => compare_numbers(value, target_value, operator),

        // version comparison
        "version_gt" | "version_gte" | "version_lt" | "version_lte" | "version_eq"
        | "version_neq" =>
            compare_versions(value, target_value, operator),

        // string/array comparison
        "any"
        | "none"
        | "str_starts_with_any"
        | "str_ends_with_any"
        | "str_contains_any"
        | "str_contains_none" =>
            compare_strings_in_array(value, target_value, operator, true),
        "any_case_sensitive" | "none_case_sensitive" => {
            compare_strings_in_array(value, target_value, operator, false)
        }
        "str_matches" =>
            compare_str_with_regex(value, target_value),

        // time comparison
        "before" | "after" | "on" =>
            compare_time(value, target_value, operator).unwrap_or(false),

        // strict equals
        "eq" => value == target_value,
        "neq" => value != target_value,

        _ => {
            ctx.result.unsupported = true;
            return;
        }
    }
}

fn evaluate_nested_gate<'a>(
    ctx: &mut EvaluatorContext<'a>,
    target_value: &'a DynamicValue,
    condition_type: &'a String,
) {
    let gate_name = match target_value.string_value.as_ref() {
        Some(name) => name,
        None => {
            log_e!("Invalid target_value for condition {}", condition_type);
            ctx.result.unsupported = true;
            return;
        }
    };

    let spec = unwrap_or_noop!(ctx
        .spec_store_data
        .values
        .feature_gates
        .get(gate_name.as_str()));

    Evaluator::evaluate(ctx, spec);

    if ctx.result.unsupported {
        return;
    }

    if !&gate_name.starts_with("segment:") {
        let res = &ctx.result;
        let expo = SecondaryExposure {
            gate: gate_name.clone(),
            gate_value: res.bool_value.to_string(),
            rule_id: res.rule_id.unwrap_or(&EMPTY_STR).clone(),
        };

        ctx.result.secondary_exposures.push(expo);
    }

    if condition_type == "fail_gate" {
        ctx.result.bool_value = !ctx.result.bool_value
    }
}

fn evaluate_config_delegate<'a>(ctx: &mut EvaluatorContext<'a>, rule: &'a Rule) -> bool {
    let delegate = unwrap_or_return!(&rule.config_delegate, false);
    let delegate_spec = unwrap_or_return!(
        ctx.spec_store_data.values.dynamic_configs.get(delegate),
        false
    );

    ctx.result.undelegated_secondary_exposures = Some(ctx.result.secondary_exposures.clone());

    Evaluator::evaluate(ctx, delegate_spec);

    ctx.result.explicit_parameters = delegate_spec.explicit_parameters.as_ref();
    ctx.result.config_delegate = rule.config_delegate.as_ref();

    true
}

fn evaluate_pass_percentage(ctx: &mut EvaluatorContext, rule: &Rule, spec_salt: &String) -> bool {
    if rule.pass_percentage == 100f64 {
        return true;
    }

    if rule.pass_percentage == 0f64 {
        return false;
    }

    let rule_salt = rule.salt.as_ref().unwrap_or(&rule.id);
    let unit_id = ctx
        .user
        .get_unit_id(&rule.id_type)
        .unwrap_or(&EMPTY_DYNAMIC_VALUE)
        .string_value
        .as_ref()
        .unwrap_or(&EMPTY_STR);
    let input = format!("{}.{}.{}", spec_salt, rule_salt, unit_id);
    match ctx.sha_hasher.compute_hash(&input) {
        Some(hash) => ((hash % 10000) as f64) < rule.pass_percentage * 100.0,
        None => false,
    }
}

fn get_hash_for_user_bucket(ctx: &mut EvaluatorContext, condition: &Condition) -> DynamicValue {
    let unit_id = ctx
        .user
        .get_unit_id(&condition.id_type)
        .unwrap_or(&EMPTY_DYNAMIC_VALUE)
        .string_value
        .as_ref()
        .unwrap_or(&EMPTY_STR);

    let mut salt: &String = &EMPTY_STR;

    if let Some(add_values) = &condition.additional_values {
        if let Some(v) = &add_values["salt"].string_value {
            salt = v;
        }
    }

    let input = format!("{}.{}", salt, unit_id);
    let hash = ctx.sha_hasher.compute_hash(&input).unwrap_or(1);
    dyn_value!(hash % 1000)
}

fn compare_numbers(left: &DynamicValue, right: &DynamicValue, op: &str) -> bool {
    let left_num = unwrap_or_return!(left.float_value, false);
    let right_num = unwrap_or_return!(right.float_value, false);

    match op {
        "gt" => left_num > right_num,
        "gte" => left_num >= right_num,
        "lt" => left_num < right_num,
        "lte" => left_num <= right_num,
        _ => false,
    }
}

fn compare_versions(left: &DynamicValue, right: &DynamicValue, op: &str) -> bool {
    let left_str = unwrap_or_return!(&left.string_value, false);
    let right_str = unwrap_or_return!(&right.string_value, false);

    fn comparison(left_str: &str, right_str: &str) -> i32 {
        let left_version = left_str.split('-').next().unwrap_or("");
        let right_version = right_str.split('-').next().unwrap_or("");

        let mut left_parts = left_version.split('.');
        let mut right_parts = right_version.split('.');

        loop {
            let opt_left_num = left_parts
                .next()
                .and_then(|s| s.parse::<i32>().ok());
            let opt_right_num = right_parts
                .next()
                .and_then(|s| s.parse::<i32>().ok());

            // If both iterators are exhausted, we break the loop
            if opt_left_num.is_none() && opt_right_num.is_none() {
                break;
            }

            let left_num = opt_left_num.unwrap_or_default();
            let right_num = opt_right_num.unwrap_or_default();

            if left_num < right_num {
                return -1;
            }

            if left_num > right_num {
                return 1;
            }
        }

        0
    }

    let result = comparison(left_str, right_str);
    match op {
        "version_gt" => result > 0,
        "version_gte" => result >= 0,
        "version_lt" => result < 0,
        "version_lte" => result <= 0,
        "version_eq" => result == 0,
        "version_neq" => result != 0,
        _ => false,
    }
}

fn compare_strings_in_array(
    value: &DynamicValue,
    target_value: &DynamicValue,
    op: &str,
    ignore_case: bool,
) -> bool {
    let value_str = value.string_value.as_ref().unwrap_or(&EMPTY_STR);
    let lowered_value_str = value.lowercase_string_value.as_ref().unwrap_or(&EMPTY_STR);

    let result = {
        if op == "any" || op == "none" {
            if let Some(dict) = &target_value.object_value {
                let contains = dict.contains_key(value_str);
                return if op == "none" {
                    !contains
                } else {
                    contains
                }
            }
        }

        let array = unwrap_or_return!(&target_value.array_value, false);

        array.iter().any(|current| {
            let (curr_str, curr_lower_str) =
                match (&current.string_value, &current.lowercase_string_value) {
                    (Some(s), Some(ls)) => (s, ls),
                    _ => return false,
                };
            let left = if ignore_case {
                lowered_value_str
            } else {
                value_str
            };
            let right = if ignore_case {
                curr_lower_str
            } else {
                curr_str
            };

            match op {
                "any" | "none" | "any_case_sensitive" | "none_case_sensitive" => left.eq(right),
                "str_starts_with_any" => left.starts_with(right),
                "str_ends_with_any" => left.ends_with(right),
                "str_contains_any" | "str_contains_none" => left.contains(right),
                _ => false,
            }
        })
    };

    if op == "none" || op == "none_case_sensitive" || op == "str_contains_none" {
        return !result;
    }
    result
}

fn compare_str_with_regex(value: &DynamicValue, regex_value: &DynamicValue) -> bool {
    let value_str = unwrap_or_return!(&value.string_value, false);
    let regex = unwrap_or_return!(&regex_value.regex_value, false);
    regex.is_match(value_str)
}

fn compare_time(left: &DynamicValue, right: &DynamicValue, op: &str) -> Option<bool> {
    let left_num = left.int_value?;
    let right_num = right.int_value?;

    match op {
        "before" => Some(left_num < right_num),
        "after" => Some(left_num > right_num),
        "on" => Some(
            Duration::milliseconds(left_num).num_days()
                == Duration::milliseconds(right_num).num_days(),
        ),
        _ => None,
    }
}
