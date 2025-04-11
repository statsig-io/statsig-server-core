use crate::evaluation::evaluation_types::SecondaryExposure;
use crate::event_logging::config_exposure::CONFIG_EXPOSURE_EVENT_NAME;
use crate::event_logging::gate_exposure::GATE_EXPOSURE_EVENT_NAME;
use crate::event_logging::layer_exposure::LAYER_EXPOSURE_EVENT_NAME;
use crate::event_logging::statsig_event::StatsigEvent;
use crate::sdk_diagnostics::diagnostics::DIAGNOSTICS_EVENT;
use crate::statsig_user_internal::StatsigUserLoggable;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsigEventInternal {
    #[serde(flatten)]
    pub event_data: StatsigEvent,

    pub user: StatsigUserLoggable,
    pub time: u64,
    pub secondary_exposures: Option<Vec<SecondaryExposure>>,
}

impl StatsigEventInternal {
    pub fn new(
        user: StatsigUserLoggable,
        event: StatsigEvent,
        secondary_exposures: Option<Vec<SecondaryExposure>>,
    ) -> Self {
        StatsigEventInternal {
            event_data: event,
            user,
            time: Utc::now().timestamp_millis() as u64,
            secondary_exposures: secondary_exposure_keys_to_expos(secondary_exposures),
        }
    }

    pub fn new_diagnostic_event(event: StatsigEvent) -> Self {
        StatsigEventInternal {
            event_data: event,
            user: StatsigUserLoggable { value: Value::Null },
            time: Utc::now().timestamp_millis() as u64,
            secondary_exposures: None,
        }
    }

    pub fn new_non_exposed_checks_event(event: StatsigEvent) -> Self {
        StatsigEventInternal {
            event_data: event,
            user: StatsigUserLoggable { value: Value::Null },
            time: Utc::now().timestamp_millis() as u64,
            secondary_exposures: None,
        }
    }

    pub fn is_exposure_event(&self) -> bool {
        self.event_data.event_name == GATE_EXPOSURE_EVENT_NAME
            || self.event_data.event_name == CONFIG_EXPOSURE_EVENT_NAME
            || self.event_data.event_name == LAYER_EXPOSURE_EVENT_NAME
    }

    pub fn is_diagnostic_event(&self) -> bool {
        self.event_data.event_name == DIAGNOSTICS_EVENT
    }
}

pub(crate) fn make_custom_event(
    user: StatsigUserLoggable,
    event: StatsigEvent,
) -> StatsigEventInternal {
    StatsigEventInternal::new(user, event, None)
}

fn secondary_exposure_keys_to_expos(
    secondary_exposures: Option<Vec<SecondaryExposure>>,
) -> Option<Vec<SecondaryExposure>> {
    match secondary_exposures.as_ref() {
        Some(secondary_exposures) => {
            let mut seen = HashSet::new();
            let mut filtered = Vec::new();
            for expo in secondary_exposures {
                let key = format!("{}.{}.{}", expo.gate, expo.rule_id, expo.gate_value);
                if !seen.contains(&key) {
                    seen.insert(key);
                    filtered.push(expo);
                }
            }

            Some(secondary_exposures.clone())
        }
        None => None,
    }
}

#[cfg(test)]
mod statsig_event_internal_tests {
    use crate::event_logging::statsig_event::StatsigEvent;
    use crate::event_logging::statsig_event_internal::{make_custom_event, StatsigEventInternal};
    use crate::statsig_user_internal::StatsigUserInternal;
    use crate::StatsigUser;
    use serde_json::{json, Value};
    use std::collections::HashMap;

    fn create_test_event() -> StatsigEventInternal {
        let user_data = StatsigUser::with_user_id("a-user".into());
        let user = StatsigUserInternal::new(&user_data, None);
        let mut sampling_statsig_metadata: HashMap<String, Value> = HashMap::new();
        sampling_statsig_metadata.insert("samplingMode".into(), "on".into());
        sampling_statsig_metadata.insert("samplingRate".into(), 101.into());
        sampling_statsig_metadata.insert("shadowLogged".into(), "logged".into());

        make_custom_event(
            user.to_loggable(),
            StatsigEvent {
                event_name: "foo".into(),
                value: Some(json!("bar")),
                metadata: Some(HashMap::from([("key".into(), "value".into())])),
                statsig_metadata: Some(sampling_statsig_metadata),
            },
        )
    }

    #[test]
    fn test_custom_event_fields() {
        let event = create_test_event();
        let data = event.event_data;

        assert_eq!(data.event_name, "foo");
        assert_eq!(data.value.unwrap().as_str(), Some("bar"));
        assert_eq!(data.metadata.unwrap().get("key").unwrap(), "value");
    }

    #[test]
    fn test_custom_event_serialization() {
        let event = create_test_event();

        let value = json!(event).as_object().cloned().unwrap();
        assert_eq!(value.get("eventName").unwrap(), "foo");
        assert_eq!(value.get("value").unwrap(), "bar");
        assert_eq!(
            value.get("metadata").unwrap().to_string(),
            "{\"key\":\"value\"}"
        );
    }
}
