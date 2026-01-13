mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{sdk_event_emitter::SdkEvent, Statsig, StatsigOptions, StatsigUser};
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};

#[derive(Debug, PartialEq)]
enum EventData {
    Evaluation(String, String),
    SpecsUpdated(String, u64),
}

struct ReceivedEvent {
    event_name: String,
    data: EventData,
}

fn setup(sdk_key: &str) -> (Statsig, StatsigUser, mpsc::Receiver<ReceivedEvent>) {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/eval_proj_dcs.json"));
    let options = StatsigOptions {
        specs_adapter: Some(specs_adapter),
        ..StatsigOptions::default()
    };
    let statsig = Statsig::new(sdk_key, Some(Arc::new(options)));
    let (tx, rx) = mpsc::channel::<ReceivedEvent>();
    let user = StatsigUser::with_user_id("a_user".to_string());

    statsig.subscribe(SdkEvent::ALL, move |event| {
        let mut result = ReceivedEvent {
            event_name: event.get_name().to_string(),
            data: EventData::Evaluation(String::new(), String::new()),
        };

        match event {
            SdkEvent::GateEvaluated {
                gate_name, reason, ..
            } => {
                result.data = EventData::Evaluation(gate_name.to_string(), reason.to_string());
            }
            SdkEvent::DynamicConfigEvaluated {
                config_name,
                reason,
                rule_id: _,
                value: _,
            } => {
                result.data = EventData::Evaluation(config_name.to_string(), reason.to_string());
            }
            SdkEvent::ExperimentEvaluated {
                experiment_name,
                reason,
                rule_id: _,
                value: _,
                group_name: _,
            } => {
                result.data =
                    EventData::Evaluation(experiment_name.to_string(), reason.to_string());
            }
            SdkEvent::LayerEvaluated {
                layer_name,
                reason,
                rule_id: _,
            } => {
                result.data = EventData::Evaluation(layer_name.to_string(), reason.to_string());
            }
            SdkEvent::SpecsUpdated {
                source,
                source_api: _,
                values,
            } => {
                result.data = EventData::SpecsUpdated(source.to_string(), values.time);
            }
        }

        tx.send(result).unwrap();
    });

    (statsig, user, rx)
}

#[tokio::test]
async fn test_gate_evaluated_event_for_check_gate() {
    let (statsig, user, rx) = setup("secret-check_gate");

    statsig.check_gate(&user, "test_gate");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "gate_evaluated");
    assert_eq!(
        event.data,
        EventData::Evaluation("test_gate".to_string(), "Uninitialized".to_string())
    );
}

#[tokio::test]
async fn test_gate_evaluated_event_for_get_feature_gate() {
    let (statsig, user, rx) = setup("secret-get_feature_gate");

    statsig.get_feature_gate(&user, "test_gate");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "gate_evaluated");
    assert_eq!(
        event.data,
        EventData::Evaluation("test_gate".to_string(), "Uninitialized".to_string())
    );
}

#[tokio::test]
async fn test_dynamic_config_evaluated_event() {
    let (statsig, user, rx) = setup("secret-get_dynamic_config");

    statsig.get_dynamic_config(&user, "test_config");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "dynamic_config_evaluated");
    assert_eq!(
        event.data,
        EventData::Evaluation("test_config".to_string(), "Uninitialized".to_string())
    );
}

#[tokio::test]
async fn test_experiment_evaluated_event() {
    let (statsig, user, rx) = setup("secret-get_experiment");

    statsig.get_experiment(&user, "test_experiment");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "experiment_evaluated");
    assert_eq!(
        event.data,
        EventData::Evaluation("test_experiment".to_string(), "Uninitialized".to_string())
    );
}

#[tokio::test]
async fn test_layer_evaluated_event() {
    let (statsig, user, rx) = setup("secret-get_layer");

    statsig.get_layer(&user, "test_layer");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "layer_evaluated");
    assert_eq!(
        event.data,
        EventData::Evaluation("test_layer".to_string(), "Uninitialized".to_string())
    );
}

#[tokio::test]
async fn test_specs_updated_event() {
    let (statsig, _, rx) = setup("secret-get_specs_updated");

    statsig.initialize().await.unwrap();

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "specs_updated");
    assert_eq!(
        event.data,
        EventData::SpecsUpdated("Bootstrap".to_string(), 1767981029384)
    );
}
