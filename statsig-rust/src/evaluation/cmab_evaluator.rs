use super::dynamic_string::DynamicString;
use super::evaluator::Evaluator;
use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::evaluation::evaluator::SpecType;
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::evaluation::get_unit_id::get_unit_id;
use crate::interned_string::InternedString;
use crate::specs_response::cmab_types::{CMABConfig, CMABGroup, CMABGroupConfig};
use crate::unwrap_or_return;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;

const EXPLORE_RULE_ID_SUFFIX: &str = "explore";

lazy_static! {
    static ref NOT_STARTED_RULE: InternedString = InternedString::from_str_ref("prestart");
    static ref FAILS_TARGETING: InternedString =
        InternedString::from_str_ref("inlineTargetingRules");
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CMABRankedGroup {
    pub score: f64,
    pub variant_name: String,
    pub rule_id: InternedString,
    pub value: Option<HashMap<String, Value>>,
    pub cmab_name: String,
}

pub fn get_cmab_ranked_list(ctx: &mut EvaluatorContext, name: &str) -> Vec<CMABRankedGroup> {
    let cmabs = match &ctx.specs_data.cmab_configs {
        Some(cmabs) => cmabs,
        None => return vec![],
    };
    let cmab = unwrap_or_return!(cmabs.get(name), vec![]);
    if !is_cmab_started(cmab) {
        return vec![];
    }

    if !get_passes_targeting(ctx, cmab) {
        return vec![];
    }

    let config = match &cmab.config {
        Some(config) => config,
        None => {
            return get_shuffled_groups(cmab)
                .iter()
                .map(|group| CMABRankedGroup {
                    score: 0.0001,
                    variant_name: group.name.clone(),
                    rule_id: group.id.clone(),
                    value: group.parameter_values.get_json(),
                    cmab_name: name.to_string(),
                })
                .collect();
        }
    };

    let unit_id = get_unit_id(ctx, &cmab.id_type);
    let input = format!("{}.{}", cmab.salt, unit_id);
    let user_hash = ctx.hashing.evaluation_hash(&input);

    let should_sample = match user_hash {
        Some(hash) => ((hash % 10000) as f64) < (cmab.sample_rate * 10000.0),
        None => false,
    };

    if should_sample {
        return get_shuffled_groups(cmab)
            .iter()
            .map(|group| CMABRankedGroup {
                score: 0.0001,
                variant_name: group.name.clone(),
                rule_id: InternedString::from_str_parts(&[group.id.as_str(), ":explore"]),
                value: group.parameter_values.get_json(),
                cmab_name: name.to_string(),
            })
            .collect();
    }
    let mut result = cmab
        .groups
        .iter()
        .map(|group| CMABRankedGroup {
            score: get_cmab_score_for_group(ctx, group, config).unwrap_or(0.0),
            variant_name: group.name.clone(),
            rule_id: group.id.clone(),
            value: group.parameter_values.get_json(),
            cmab_name: name.to_string(),
        })
        .collect::<Vec<CMABRankedGroup>>();
    let higher_better = cmab.higher_is_better;
    result.sort_by(|a, b| match higher_better {
        true => b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal),
        false => a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal),
    });
    result
}

pub(crate) fn evaluate_cmab(
    ctx: &mut EvaluatorContext,
    spec_name: &str,
    spec_type: &SpecType,
) -> bool {
    match spec_type {
        SpecType::Experiment => {}
        _ => return false,
    }

    let cmabs = match &ctx.specs_data.cmab_configs {
        Some(cmabs) => cmabs,
        None => return false,
    };
    let cmab = unwrap_or_return!(cmabs.get(spec_name), false);

    if !is_cmab_started(cmab) {
        ctx.result.id_type = Some(InternedString::from_str_ref(&cmab.id_type.value));
        ctx.result.version = Some(cmab.version);
        ctx.result.is_experiment_active = cmab.enabled;
        ctx.result.bool_value = false;
        ctx.result.rule_id = Some(NOT_STARTED_RULE.clone());
        ctx.result.json_value = Some(cmab.default_value.clone());
        return true;
    }

    if !get_passes_targeting(ctx, cmab) {
        ctx.result.id_type = Some(InternedString::from_dynamic_string(&cmab.id_type));
        ctx.result.version = Some(cmab.version);
        ctx.result.is_experiment_active = cmab.enabled;
        ctx.result.bool_value = false;
        ctx.result.rule_id = Some(FAILS_TARGETING.clone());
        ctx.result.json_value = Some(cmab.default_value.clone());
        return true;
    }

    ctx.result.id_type = Some(InternedString::from_dynamic_string(&cmab.id_type));
    ctx.result.version = Some(cmab.version);
    ctx.result.is_experiment_active = cmab.enabled;

    let unit_id = get_unit_id(ctx, &cmab.id_type);
    let input = format!("{}.{}", cmab.salt, unit_id);
    let user_hash = ctx.hashing.evaluation_hash(&input);
    let config = match &cmab.config {
        Some(config) => config,
        None => {
            apply_random_group(ctx, cmab, user_hash);
            return true;
        }
    };

    let should_sample = match user_hash {
        Some(hash) => ((hash % 10000) as f64) < (cmab.sample_rate * 10000.0),
        None => false,
    };

    if should_sample && apply_sampling_group(ctx, cmab, config) {
        return true;
    }
    apply_best_group(ctx, cmab, config);
    true
}

fn get_passes_targeting<'a>(ctx: &mut EvaluatorContext<'a>, cmab: &'a CMABConfig) -> bool {
    let targeting_gate_name = match &cmab.targeting_gate_name {
        Some(name) => name,
        None => return true,
    };

    match ctx.prep_for_nested_evaluation() {
        Ok(_) => {}
        Err(_) => {
            ctx.result.bool_value = false;
            ctx.result.rule_id = Some(FAILS_TARGETING.clone());
            return false;
        }
    }

    let result = match Evaluator::evaluate(ctx, targeting_gate_name.as_str(), &SpecType::Gate) {
        Ok(_) => ctx.result.bool_value,
        Err(_) => false,
    };

    let expo = SecondaryExposure {
        gate: targeting_gate_name.clone(),
        gate_value: InternedString::from_bool(result),
        rule_id: ctx.result.rule_id.clone().unwrap_or_default(),
    };

    ctx.result.secondary_exposures.push(expo);

    result
}

fn get_shuffled_groups(cmab: &CMABConfig) -> Vec<&CMABGroup> {
    let mut groups = cmab.groups.iter().collect::<Vec<&CMABGroup>>();
    let mut rng = rand::thread_rng();
    groups.shuffle(&mut rng);
    groups
}

fn is_cmab_started(cmab: &CMABConfig) -> bool {
    cmab.enabled && !cmab.groups.is_empty()
}

fn apply_random_group<'a>(
    ctx: &mut EvaluatorContext<'a>,
    cmab: &'a CMABConfig,
    user_hash: Option<u64>,
) {
    let group_size = 10000.0 / (cmab.groups.len() as f64);
    let group = match user_hash {
        Some(hash) => &cmab.groups[(((hash % 10000) as f64) / group_size).floor() as usize],
        None => &cmab.groups[0],
    };
    ctx.result.bool_value = true;
    ctx.result.rule_id = Some(group.id.clone());
    ctx.result.rule_id_suffix = Some(EXPLORE_RULE_ID_SUFFIX);
    ctx.result.group_name = Some(InternedString::from_str_ref(&group.name));
    ctx.result.json_value = Some(group.parameter_values.clone());
}

fn apply_sampling_group<'a>(
    ctx: &mut EvaluatorContext<'a>,
    cmab: &'a CMABConfig,
    config: &HashMap<String, CMABGroupConfig>,
) -> bool {
    let mut total_records: f64 = 0.0;
    for group in &cmab.groups {
        let cur_count = match config.get(group.id.as_str()) {
            Some(config_for_group) => config_for_group.records + 1,
            None => 1,
        };
        total_records += 1.0 / cur_count as f64;
    }

    let mut sum: f64 = 0.0;
    let mut rng = rand::thread_rng();
    let value: f64 = rng.gen::<f64>();
    for group in &cmab.groups {
        let cur_count = match config.get(group.id.as_str()) {
            Some(config_for_group) => config_for_group.records + 1,
            None => 1,
        };
        sum += 1.0 / (cur_count as f64) / total_records;
        if value < sum {
            ctx.result.rule_id = Some(group.id.clone());
            ctx.result.rule_id_suffix = Some(EXPLORE_RULE_ID_SUFFIX);
            ctx.result.bool_value = true;
            ctx.result.group_name = Some(InternedString::from_str_ref(&group.name));
            ctx.result.json_value = Some(group.parameter_values.clone());
            return true;
        }
    }
    false
}

fn apply_best_group<'a>(
    ctx: &mut EvaluatorContext<'a>,
    cmab: &'a CMABConfig,
    config: &HashMap<String, CMABGroupConfig>,
) {
    let higher_better = cmab.higher_is_better;
    let mut best_score = if higher_better {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut best_group = &cmab.groups[0];
    let mut has_score = false;
    for group in &cmab.groups {
        let score = match get_cmab_score_for_group(ctx, group, config) {
            Some(s) => s,
            None => continue,
        };
        has_score = true;
        if (higher_better && score > best_score) || (!higher_better && score < best_score) {
            best_score = score;
            best_group = group;
        }
    }

    // if we had no score for any group randomly select one
    if !has_score {
        let mut rng = rand::thread_rng();
        let random: f64 = rng.gen::<f64>();
        best_group = &cmab.groups[(random * cmab.groups.len() as f64).floor() as usize];
    }
    ctx.result.bool_value = true;
    ctx.result.rule_id = Some(best_group.id.clone());
    ctx.result.group_name = Some(InternedString::from_str_ref(&best_group.name));
    ctx.result.json_value = Some(best_group.parameter_values.clone());
}

fn get_cmab_score_for_group(
    ctx: &mut EvaluatorContext,
    group: &CMABGroup,
    config: &HashMap<String, CMABGroupConfig>,
) -> Option<f64> {
    let mut score = 0.0;
    let config_for_group = config.get(group.id.as_str())?;
    let weights_numerical = &config_for_group.weights_numerical;
    let weights_categorical = &config_for_group.weights_categorical;
    if weights_numerical.is_empty() && weights_categorical.is_empty() {
        return None;
    }

    score += config_for_group.intercept;
    score += config_for_group.alpha;
    for (key, value) in weights_categorical {
        let user_value = ctx
            .user
            .get_user_value(&Some(DynamicString::from(key.clone())));
        let user_value_string = match user_value {
            Some(v) => match &v.string_value {
                Some(s) => s.value.unperformant_to_string(),
                None => v.json_value.to_string(),
            },
            None => continue,
        };
        let weight = match value.get(&user_value_string) {
            Some(w) => w,
            None => continue,
        };
        score += weight;
    }

    for (key, value) in weights_numerical {
        let user_value = ctx
            .user
            .get_user_value(&Some(DynamicString::from(key.clone())));
        let user_value_float = match user_value {
            Some(v) => match v.float_value {
                Some(f) => f,
                None => continue,
            },
            None => continue,
        };
        let weight = value;
        score += user_value_float * weight;
    }
    Some(score)
}
