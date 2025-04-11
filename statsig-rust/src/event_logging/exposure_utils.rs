use crate::evaluation::evaluation_details::EvaluationDetails;
use crate::sampling_processor::SamplingDecision;
use crate::statsig_user_internal::StatsigUserLoggable;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) fn get_statsig_metadata_with_sampling_details(
    sampling_details: SamplingDecision,
) -> HashMap<String, Value> {
    let mut statsig_metadata: HashMap<String, Value> = HashMap::new();

    if let Some(rate) = sampling_details.sampling_rate {
        statsig_metadata.insert("samplingRate".into(), Value::Number(rate.into()));
    }

    statsig_metadata.insert(
        "samplingMode".into(),
        Value::String(format!("{:?}", sampling_details.sampling_mode).to_lowercase()),
    );
    statsig_metadata.insert(
        "shadowLogged".into(),
        Value::String(format!("{:?}", sampling_details.sampling_status).to_lowercase()),
    );

    statsig_metadata
}

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
    user: &StatsigUserLoggable,
    spec_name: &String,
    rule_id: Option<&String>,
    additional_values: Option<Vec<String>>,
) -> String {
    let mut expo_key = String::from(spec_name);
    expo_key += "|";

    expo_key += rule_id.map(|x| x.as_str()).unwrap_or_default();
    expo_key += "|";

    let user_id = user
        .value
        .get("userID")
        .map(|x| x.as_str())
        .unwrap_or_default()
        .unwrap_or_default();

    expo_key += user_id;
    expo_key += "|";

    let custom_ids = user
        .value
        .get("customIDs")
        .map(|x| x.as_object())
        .unwrap_or_default();

    if let Some(custom_ids) = custom_ids {
        for (_, val) in custom_ids.iter() {
            if let Some(string_value) = &val.as_str() {
                expo_key += string_value;
                expo_key += "|";
            }
        }
    };

    if let Some(additional_values) = additional_values {
        for value in additional_values {
            expo_key += &value;
            expo_key += "|";
        }
    }

    expo_key
}

#[cfg(test)]
mod tests {
    use crate::{dyn_value, statsig_user_internal::StatsigUserInternal, StatsigUser};

    use super::*;

    #[test]
    fn test_expo_key_with_user_id() {
        let user: StatsigUser = StatsigUser::with_user_id("test_user_id".to_string());
        let user = StatsigUserInternal::new(&user, None);

        let spec_name = "test_spec_name".to_string();
        let rule_id = "test_rule_id".to_string();

        let key = make_exposure_key(&user.to_loggable(), &spec_name, Some(&rule_id), None);
        assert_eq!(key, "test_spec_name|test_rule_id|test_user_id|");
    }

    #[test]
    fn test_expo_key_with_custom_id() {
        let user: StatsigUser = StatsigUser::with_custom_ids(HashMap::from([(
            "test_custom_id".to_string(),
            "test_custom_id_value".to_string(),
        )]));
        let user = StatsigUserInternal::new(&user, None);

        let spec_name = "test_spec_name".to_string();
        let rule_id = "test_rule_id".to_string();

        let key = make_exposure_key(&user.to_loggable(), &spec_name, Some(&rule_id), None);
        assert_eq!(key, "test_spec_name|test_rule_id||test_custom_id_value|");
    }

    #[test]
    fn test_expo_key_full() {
        let user: StatsigUser = StatsigUser {
            user_id: Some(dyn_value!("test_user_id")),
            email: Some(dyn_value!("test_email@mail.com")),
            custom_ids: Some(HashMap::from([(
                "test_custom_id".to_string(),
                dyn_value!("test_custom_id_value"),
            )])),
            ..StatsigUser::with_custom_ids(HashMap::from([(
                "test_custom_id".to_string(),
                "test_custom_id_value".to_string(),
            )]))
        };
        let user = StatsigUserInternal::new(&user, None);

        let spec_name = "test_spec_name".to_string();
        let rule_id = "test_rule_id".to_string();
        let additional_values = vec!["test_additional_value".to_string()];

        let key = make_exposure_key(
            &user.to_loggable(),
            &spec_name,
            Some(&rule_id),
            Some(additional_values),
        );
        assert_eq!(
            key,
            "test_spec_name|test_rule_id|test_user_id|test_custom_id_value|test_additional_value|"
        );
    }
}
