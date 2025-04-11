use chrono::Utc;
use lazy_static::lazy_static;

use crate::evaluation::cmab_evaluator::evaluate_cmab;
use crate::evaluation::comparisons::{
    compare_arrays, compare_numbers, compare_str_with_regex, compare_strings_in_array,
    compare_time, compare_versions,
};
use crate::evaluation::dynamic_value::DynamicValue;
use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::evaluation::get_unit_id::get_unit_id;
use crate::evaluation::ua_parser::UserAgentParser;
use crate::spec_types::{Condition, Rule, Spec};
use crate::{dyn_value, log_e, unwrap_or_return, StatsigErr};

use super::country_lookup::CountryLookup;

const TAG: &str = stringify!(Evaluator);

pub struct Evaluator;

lazy_static! {
    static ref EMPTY_STR: String = String::new();
    static ref DEFAULT_RULE: String = "default".to_string();
    static ref DISABLED_RULE: String = "disabled".to_string();
    static ref EMPTY_DYNAMIC_VALUE: DynamicValue = DynamicValue::new();
}

#[derive(Clone)]
pub enum SpecType {
    Gate,
    DynamicConfig,
    Experiment,
    Layer,
}

#[derive(PartialEq, Eq)]
pub enum Recognition {
    Unrecognized,
    Recognized,
}

impl Evaluator {
    pub fn evaluate_with_details(
        ctx: &mut EvaluatorContext,
        spec_name: &str,
        spec_type: &SpecType,
    ) -> Result<EvaluationDetails, StatsigErr> {
        let recognition = Self::evaluate(ctx, spec_name, spec_type)?;

        if recognition == Recognition::Unrecognized {
            return Ok(EvaluationDetails::unrecognized(ctx.spec_store_data));
        }

        if let Some(reason) = ctx.result.override_reason {
            return Ok(EvaluationDetails::recognized_but_overridden(
                ctx.spec_store_data,
                reason,
            ));
        }

        Ok(EvaluationDetails::recognized(
            ctx.spec_store_data,
            &ctx.result,
        ))
    }

    pub fn evaluate(
        ctx: &mut EvaluatorContext,
        spec_name: &str,
        spec_type: &SpecType,
    ) -> Result<Recognition, StatsigErr> {
        let opt_spec = match spec_type {
            SpecType::Gate => ctx.spec_store_data.values.feature_gates.get(spec_name),
            SpecType::DynamicConfig => ctx.spec_store_data.values.dynamic_configs.get(spec_name),
            SpecType::Experiment => ctx.spec_store_data.values.dynamic_configs.get(spec_name),
            SpecType::Layer => ctx.spec_store_data.values.layer_configs.get(spec_name),
        };

        if try_apply_override(ctx, spec_name, spec_type, opt_spec) {
            return Ok(Recognition::Recognized);
        }

        if try_apply_config_mapping(ctx, spec_name, spec_type, opt_spec) {
            return Ok(Recognition::Recognized);
        }

        if evaluate_cmab(ctx, spec_name, spec_type) {
            return Ok(Recognition::Recognized);
        }

        let spec = unwrap_or_return!(opt_spec, Ok(Recognition::Unrecognized));

        if ctx.result.id_type.is_none() {
            ctx.result.id_type = Some(&spec.id_type);
        }

        if ctx.result.version.is_none() {
            if let Some(version) = spec.version {
                ctx.result.version = Some(version);
            }
        }

        if let Some(is_active) = spec.is_active {
            ctx.result.is_experiment_active = is_active;
        }

        if let Some(has_shared_params) = spec.has_shared_params {
            ctx.result.is_in_layer = has_shared_params;
        }

        if let Some(explicit_params) = &spec.explicit_parameters {
            ctx.result.explicit_parameters = Some(explicit_params);
        }

        for rule in &spec.rules {
            evaluate_rule(ctx, rule)?;

            if ctx.result.unsupported {
                return Ok(Recognition::Recognized);
            }

            if !ctx.result.bool_value {
                continue;
            }

            if evaluate_config_delegate(ctx, rule)? {
                ctx.finalize_evaluation(spec, Some(rule));
                return Ok(Recognition::Recognized);
            }

            let did_pass = evaluate_pass_percentage(ctx, rule, &spec.salt);

            if did_pass {
                ctx.result.bool_value = rule.return_value.string_value != "false";
                ctx.result.json_value = rule.return_value.json_value.clone();
            } else {
                ctx.result.bool_value = spec.default_value.string_value == "true";
                ctx.result.json_value = spec.default_value.json_value.clone();
            }

            ctx.result.rule_id = Some(&rule.id);
            ctx.result.group_name = rule.group_name.as_ref();
            ctx.result.is_experiment_group = rule.is_experiment_group.unwrap_or(false);
            ctx.result.is_experiment_active = spec.is_active.unwrap_or(false);
            ctx.finalize_evaluation(spec, Some(rule));
            return Ok(Recognition::Recognized);
        }

        ctx.result.bool_value = spec.default_value.string_value == "true";
        ctx.result.json_value = spec.default_value.json_value.clone();
        ctx.result.rule_id = match spec.enabled {
            true => Some(&DEFAULT_RULE),
            false => Some(&DISABLED_RULE),
        };
        ctx.finalize_evaluation(spec, None);

        Ok(Recognition::Recognized)
    }
}

fn try_apply_config_mapping(
    ctx: &mut EvaluatorContext,
    spec_name: &str,
    spec_type: &SpecType,
    opt_spec: Option<&Spec>,
) -> bool {
    let overrides = match &ctx.spec_store_data.values.overrides {
        Some(overrides) => overrides,
        None => return false,
    };

    let override_rules = match &ctx.spec_store_data.values.override_rules {
        Some(override_rules) => override_rules,
        None => return false,
    };

    let mapping_list = match overrides.get(spec_name) {
        Some(mapping_list) => mapping_list,
        None => return false,
    };

    let spec_salt = match opt_spec {
        Some(spec) => &spec.salt,
        None => &EMPTY_STR,
    };

    for mapping in mapping_list {
        for override_rule in &mapping.rules {
            let start_time = override_rule.start_time.unwrap_or_default();

            if start_time > Utc::now().timestamp_millis() {
                continue;
            }

            let rule = match override_rules.get(&override_rule.rule_name) {
                Some(rule) => rule,
                None => continue,
            };
            match evaluate_rule(ctx, rule) {
                Ok(_) => {}
                Err(_) => {
                    ctx.reset_result();
                    continue;
                }
            }

            if !ctx.result.bool_value || ctx.result.unsupported {
                ctx.reset_result();
                continue;
            }
            ctx.reset_result();
            let pass = evaluate_pass_percentage(ctx, rule, spec_salt);
            if pass {
                ctx.result.override_config_name = Some(mapping.new_config_name.clone());
                match Evaluator::evaluate(ctx, mapping.new_config_name.as_str(), spec_type) {
                    Ok(Recognition::Recognized) => {
                        return true;
                    }
                    _ => {
                        ctx.reset_result();
                        break;
                    }
                }
            }
        }
    }

    false
}

fn try_apply_override(
    ctx: &mut EvaluatorContext,
    spec_name: &str,
    spec_type: &SpecType,
    opt_spec: Option<&Spec>,
) -> bool {
    let adapter = match ctx.override_adapter {
        Some(adapter) => adapter,
        None => return false,
    };

    match spec_type {
        SpecType::Gate => adapter.get_gate_override(ctx.user.user_data, spec_name, &mut ctx.result),

        SpecType::DynamicConfig => {
            adapter.get_dynamic_config_override(ctx.user.user_data, spec_name, &mut ctx.result)
        }

        SpecType::Experiment => adapter.get_experiment_override(
            ctx.user.user_data,
            spec_name,
            &mut ctx.result,
            opt_spec,
        ),

        SpecType::Layer => {
            adapter.get_layer_override(ctx.user.user_data, spec_name, &mut ctx.result)
        }
    }
}

fn evaluate_rule<'a>(ctx: &mut EvaluatorContext<'a>, rule: &'a Rule) -> Result<(), StatsigErr> {
    let mut all_conditions_pass = true;
    // println!("--- Eval Rule {} ---", rule.id);
    for condition_hash in &rule.conditions {
        // println!("Condition Hash {}", condition_hash);
        let opt_condition = ctx.spec_store_data.values.condition_map.get(condition_hash);
        let condition = if let Some(c) = opt_condition {
            c
        } else {
            // todo: log condition not found error
            ctx.result.unsupported = true;
            return Ok(());
        };

        evaluate_condition(ctx, condition)?;

        if !ctx.result.bool_value {
            all_conditions_pass = false;
        }
    }

    ctx.result.bool_value = all_conditions_pass;

    Ok(())
}

fn evaluate_condition<'a>(
    ctx: &mut EvaluatorContext<'a>,
    condition: &'a Condition,
) -> Result<(), StatsigErr> {
    let temp_value;
    let target_value = condition
        .target_value
        .as_ref()
        .unwrap_or(&EMPTY_DYNAMIC_VALUE);
    let condition_type = &condition.condition_type;

    let value = match condition_type as &str {
        "public" => {
            ctx.result.bool_value = true;
            return Ok(());
        }
        "fail_gate" | "pass_gate" => {
            evaluate_nested_gate(ctx, target_value, condition_type)?;
            return Ok(());
        }
        "ua_based" => match ctx.user.get_user_value(&condition.field) {
            Some(value) => Some(value),
            None => {
                temp_value = UserAgentParser::get_value_from_user_agent(ctx.user, &condition.field);
                temp_value.as_ref()
            }
        },
        "ip_based" => match ctx.user.get_user_value(&condition.field) {
            Some(value) => Some(value),
            None => {
                temp_value = CountryLookup::get_value_from_ip(ctx.user, &condition.field);
                temp_value.as_ref()
            }
        },
        "user_field" => ctx.user.get_user_value(&condition.field),
        "environment_field" => {
            temp_value = ctx.user.get_value_from_environment(&condition.field);
            temp_value.as_ref()
        }
        "current_time" => {
            temp_value = Some(DynamicValue::for_timestamp_evaluation(
                Utc::now().timestamp_millis(),
            ));
            temp_value.as_ref()
        }
        "user_bucket" => {
            temp_value = Some(get_hash_for_user_bucket(ctx, condition));
            temp_value.as_ref()
        }
        "target_app" => *ctx.app_id,
        "unit_id" => ctx.user.get_unit_id(&condition.id_type),
        _ => {
            ctx.result.unsupported = true;
            return Ok(());
        }
    }
    .unwrap_or(&EMPTY_DYNAMIC_VALUE);

    // println!("Eval Condition {}, {:?}", condition_type, value);

    let operator = match &condition.operator {
        Some(operator) => operator,
        None => {
            ctx.result.unsupported = true;
            return Ok(());
        }
    };

    ctx.result.bool_value = match operator as &str {
        // numerical comparisons
        "gt" | "gte" | "lt" | "lte" => compare_numbers(value, target_value, operator),

        // version comparisons
        "version_gt" | "version_gte" | "version_lt" | "version_lte" | "version_eq"
        | "version_neq" => compare_versions(value, target_value, operator),

        // string/array comparisons
        "any"
        | "none"
        | "str_starts_with_any"
        | "str_ends_with_any"
        | "str_contains_any"
        | "str_contains_none" => compare_strings_in_array(value, target_value, operator, true),
        "any_case_sensitive" | "none_case_sensitive" => {
            compare_strings_in_array(value, target_value, operator, false)
        }
        "str_matches" => compare_str_with_regex(value, target_value),

        // time comparisons
        "before" | "after" | "on" => compare_time(value, target_value, operator),

        // strict equals
        "eq" => value == target_value,
        "neq" => value != target_value,

        // id_lists
        "in_segment_list" | "not_in_segment_list" => {
            evaluate_id_list(ctx, operator, target_value, value)
        }

        "array_contains_any"
        | "array_contains_none"
        | "array_contains_all"
        | "not_array_contains_all" => compare_arrays(value, target_value, operator),

        _ => {
            ctx.result.unsupported = true;
            return Ok(());
        }
    };

    Ok(())
}

fn evaluate_id_list(
    ctx: &mut EvaluatorContext<'_>,
    op: &str,
    target_value: &DynamicValue,
    value: &DynamicValue,
) -> bool {
    let list_name = unwrap_or_return!(&target_value.string_value, false);
    let id_lists = &ctx.spec_store_data.id_lists;

    let list = unwrap_or_return!(id_lists.get(list_name), false);

    let value = unwrap_or_return!(&value.string_value, false);
    let hashed = ctx.hashing.sha256(value);
    let lookup_id: String = hashed.chars().take(8).collect();

    let is_in_list = list.ids.contains(&lookup_id);

    if op == "not_in_segment_list" {
        return !is_in_list;
    }

    is_in_list
}

fn evaluate_nested_gate<'a>(
    ctx: &mut EvaluatorContext<'a>,
    target_value: &'a DynamicValue,
    condition_type: &'a String,
) -> Result<(), StatsigErr> {
    let gate_name = if let Some(name) = target_value.string_value.as_ref() {
        name
    } else {
        log_e!(
            TAG,
            "Invalid target_value for condition {}, {:?}",
            condition_type,
            target_value
        );
        ctx.result.unsupported = true;
        return Ok(());
    };

    ctx.prep_for_nested_evaluation()?;
    let _ = Evaluator::evaluate(ctx, gate_name, &SpecType::Gate)?;

    if ctx.result.unsupported {
        return Ok(());
    }

    if !&gate_name.starts_with("segment:") {
        let res = &ctx.result;
        let expo = SecondaryExposure {
            gate: gate_name.clone(),
            gate_value: res.bool_value.to_string(),
            rule_id: res.rule_id.unwrap_or(&EMPTY_STR).clone(),
        };

        if res.sampling_rate.is_none() {
            ctx.result.has_seen_analytical_gates = Option::from(true);
        }

        ctx.result.secondary_exposures.push(expo);
    }

    if condition_type == "fail_gate" {
        ctx.result.bool_value = !ctx.result.bool_value;
    }
    Ok(())
}

fn evaluate_config_delegate<'a>(
    ctx: &mut EvaluatorContext<'a>,
    rule: &'a Rule,
) -> Result<bool, StatsigErr> {
    let delegate = unwrap_or_return!(&rule.config_delegate, Ok(false));
    let delegate_spec = unwrap_or_return!(
        ctx.spec_store_data.values.dynamic_configs.get(delegate),
        Ok(false)
    );

    ctx.result.undelegated_secondary_exposures = Some(ctx.result.secondary_exposures.clone());

    ctx.prep_for_nested_evaluation()?;
    let recognition = Evaluator::evaluate(ctx, delegate, &SpecType::Experiment)?;
    if recognition == Recognition::Unrecognized {
        ctx.result.undelegated_secondary_exposures = None;
        return Ok(false);
    }

    ctx.result.explicit_parameters = delegate_spec.explicit_parameters.as_ref();
    ctx.result.config_delegate = rule.config_delegate.as_ref();

    Ok(true)
}

fn evaluate_pass_percentage(ctx: &mut EvaluatorContext, rule: &Rule, spec_salt: &String) -> bool {
    if rule.pass_percentage == 100f64 {
        return true;
    }

    if rule.pass_percentage == 0f64 {
        return false;
    }

    let rule_salt = rule.salt.as_ref().unwrap_or(&rule.id);
    let unit_id = get_unit_id(ctx, &rule.id_type);
    let input = format!("{spec_salt}.{rule_salt}.{unit_id}");
    match ctx.hashing.evaluation_hash(&input) {
        Some(hash) => ((hash % 10000) as f64) < rule.pass_percentage * 100.0,
        None => false,
    }
}

fn get_hash_for_user_bucket(ctx: &mut EvaluatorContext, condition: &Condition) -> DynamicValue {
    let unit_id = get_unit_id(ctx, &condition.id_type);

    let mut salt: &String = &EMPTY_STR;

    if let Some(add_values) = &condition.additional_values {
        if let Some(v) = &add_values["salt"].string_value {
            salt = v;
        }
    }

    let input = format!("{salt}.{unit_id}");
    let hash = ctx.hashing.evaluation_hash(&input).unwrap_or(1);
    dyn_value!(hash % 1000)
}
