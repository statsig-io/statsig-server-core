use statsig_rust::{sdk_event_emitter::SdkEvent, Statsig, StatsigUser};
use std::{sync::mpsc, time::Duration};

struct ReceivedEvent {
    event_name: String,
    spec_name: String,
    eval_reason: String,
}

fn setup(sdk_key: &str) -> (Statsig, StatsigUser, mpsc::Receiver<ReceivedEvent>) {
    let statsig = Statsig::new(sdk_key, None);
    let (tx, rx) = mpsc::channel::<ReceivedEvent>();
    let user = StatsigUser::with_user_id("a_user".to_string());

    statsig.subscribe(SdkEvent::ALL, move |event| {
        let mut result = ReceivedEvent {
            event_name: event.get_name().to_string(),
            spec_name: String::new(),
            eval_reason: String::new(),
        };

        match event {
            SdkEvent::GateEvaluated {
                gate_name, reason, ..
            } => {
                result.spec_name = gate_name.to_string();
                result.eval_reason = reason.to_string();
            }
            SdkEvent::DynamicConfigEvaluated { dynamic_config } => {
                result.spec_name = dynamic_config.name.to_string();
                result.eval_reason = dynamic_config.details.reason.to_string();
            }
            SdkEvent::ExperimentEvaluated { experiment } => {
                result.spec_name = experiment.name.to_string();
                result.eval_reason = experiment.details.reason.to_string();
            }
            SdkEvent::LayerEvaluated { layer } => {
                result.spec_name = layer.name.to_string();
                result.eval_reason = layer.details.reason.to_string();
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
    assert_eq!(event.spec_name, "test_gate");
    assert_eq!(event.eval_reason, "Uninitialized");
}

#[tokio::test]
async fn test_gate_evaluated_event_for_get_feature_gate() {
    let (statsig, user, rx) = setup("secret-get_feature_gate");

    statsig.get_feature_gate(&user, "test_gate");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "gate_evaluated");
    assert_eq!(event.spec_name, "test_gate");
    assert_eq!(event.eval_reason, "Uninitialized");
}

#[tokio::test]
async fn test_dynamic_config_evaluated_event() {
    let (statsig, user, rx) = setup("secret-get_dynamic_config");

    statsig.get_dynamic_config(&user, "test_config");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "dynamic_config_evaluated");
    assert_eq!(event.spec_name, "test_config");
    assert_eq!(event.eval_reason, "Uninitialized");
}

#[tokio::test]
async fn test_experiment_evaluated_event() {
    let (statsig, user, rx) = setup("secret-get_experiment");

    statsig.get_experiment(&user, "test_experiment");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "experiment_evaluated");
    assert_eq!(event.spec_name, "test_experiment");
    assert_eq!(event.eval_reason, "Uninitialized");
}

#[tokio::test]
async fn test_layer_evaluated_event() {
    let (statsig, user, rx) = setup("secret-get_layer");

    statsig.get_layer(&user, "test_layer");

    let event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(event.event_name, "layer_evaluated");
    assert_eq!(event.spec_name, "test_layer");
    assert_eq!(event.eval_reason, "Uninitialized");
}
