use serde_json::json;

use crate::{
    sdk_event_emitter::{SdkEvent, SdkEventEmitter, SubscriptionID},
    DynamicReturnable,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn sub(
    event_emitter: &mut SdkEventEmitter,
    event_name: &str,
) -> (SubscriptionID, Arc<AtomicUsize>) {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    let id = event_emitter.subscribe(event_name, move |_| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    (id, counter)
}

fn emit(event_emitter: &mut SdkEventEmitter, event_name: &str) {
    match event_name {
        SdkEvent::GATE_EVALUATED => {
            event_emitter.emit(SdkEvent::GateEvaluated {
                gate_name: "test_gate",
                rule_id: "test_rule_id",
                value: true,
                reason: "test_reason",
            });
        }
        SdkEvent::DYNAMIC_CONFIG_EVALUATED => {
            event_emitter.emit(SdkEvent::DynamicConfigEvaluated {
                config_name: "test_dynamic_config",
                reason: "test_reason",
                rule_id: Some("test_rule_id"),
                value: Some(&DynamicReturnable::from_map(
                    std::collections::HashMap::from([(
                        "test_param".to_string(),
                        json!("test_value"),
                    )]),
                )),
            });
        }
        _ => {
            panic!("Unsupported event: {event_name}");
        }
    }
}

#[test]
fn test_unsub_by_event() {
    let mut event_emitter = SdkEventEmitter::default();

    let (_, first_counter) = sub(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    let (_, second_counter) = sub(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    let (_, third_counter) = sub(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);

    emit(&mut event_emitter, SdkEvent::GATE_EVALUATED);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 0);

    emit(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);

    event_emitter.unsubscribe(SdkEvent::GATE_EVALUATED);
    emit(&mut event_emitter, SdkEvent::GATE_EVALUATED);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);

    emit(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);
    assert_eq!(third_counter.load(Ordering::SeqCst), 2);
}

#[test]
fn test_unsub_by_event_and_id() {
    let mut event_emitter = SdkEventEmitter::default();

    let (first_id, first_counter) = sub(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    let (_, second_counter) = sub(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    let (_, third_counter) = sub(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);

    emit(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    emit(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);

    event_emitter.unsubscribe_by_id(&first_id);
    emit(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    emit(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 2);
    assert_eq!(third_counter.load(Ordering::SeqCst), 2);
}

#[test]
fn test_unsub_all() {
    let mut event_emitter = SdkEventEmitter::default();

    let (_, first_counter) = sub(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    let (_, second_counter) = sub(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    let (_, third_counter) = sub(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);

    emit(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    emit(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);

    event_emitter.unsubscribe_all();

    emit(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    emit(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_sub_all() {
    let mut event_emitter = SdkEventEmitter::default();
    let (_, counter) = sub(&mut event_emitter, SdkEvent::ALL);

    emit(&mut event_emitter, SdkEvent::GATE_EVALUATED);
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    emit(&mut event_emitter, SdkEvent::DYNAMIC_CONFIG_EVALUATED);
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}
