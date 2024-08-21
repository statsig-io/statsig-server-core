use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::evaluation::evaluation_types::BaseEvaluation;
use crate::StatsigUser;
use std::collections::HashMap;

pub(crate) fn get_metadata_with_details(
    evaluation_details: EvaluationDetails,
) -> HashMap<String, String> {
    let mut metadata: HashMap<String, String> = HashMap::new();

    metadata.insert("reason".into(), evaluation_details.reason);

    if let Some(lcut) = evaluation_details.lcut {
        metadata.insert("lcut".into(), lcut.to_string());
    }

    if let Some(received_at) = evaluation_details.received_at {
        metadata.insert("receivedAt".into(), received_at.to_string());
    }

    metadata
}

pub(crate) fn make_exposure_key(
    user: &StatsigUser,
    spec_name: &String,
    base_evaluation: Option<&BaseEvaluation>,
) -> String {
    let empty_str = "".to_string();

    let user_id = user
        .user_id
        .as_ref()
        .and_then(|id| id.string_value.as_ref())
        .unwrap_or(&empty_str);

    let mut custom_ids = "".to_string();
    if let Some(ids) = &user.custom_ids {
        let values: Vec<String> = ids
            .values()
            .map(|v| v.string_value.clone().unwrap_or_default())
            .collect();
        custom_ids = values.join("|");
    }

    let mut rule_id = &empty_str;
    if let Some(eval) = base_evaluation {
        rule_id = &eval.rule_id;
    }

    format!("{}|{}|{}|{}", spec_name, rule_id, user_id, custom_ids)
}
