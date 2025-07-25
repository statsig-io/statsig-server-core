use crate::event_emitter::{SdkEvent, SdkEventEmitter};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn sub(event_emitter: &mut SdkEventEmitter, event_name: &str) -> (String, Arc<AtomicUsize>) {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    let id: String = event_emitter.subscribe(event_name, move |_| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    (id, counter)
}

fn emit(event_emitter: &mut SdkEventEmitter, event_name: &str) {
    match event_name {
        SdkEvent::RULESETS_UPDATED => {
            event_emitter.emit(SdkEvent::RulesetsUpdated {
                lcut: 0,
                raw_value: "".to_string(),
            });
        }
        SdkEvent::INIT_SUCCESS => {
            event_emitter.emit(SdkEvent::InitSuccess { duration: 1.0 });
        }
        _ => {
            panic!("Unsupported event: {event_name}");
        }
    }
}

#[test]
fn test_unsub_by_event() {
    let mut event_emitter = SdkEventEmitter::default();

    let (_, first_counter) = sub(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    let (_, second_counter) = sub(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    let (_, third_counter) = sub(&mut event_emitter, SdkEvent::INIT_SUCCESS);

    emit(&mut event_emitter, SdkEvent::RULESETS_UPDATED);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 0);

    emit(&mut event_emitter, SdkEvent::INIT_SUCCESS);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);

    event_emitter.unsubscribe(SdkEvent::RULESETS_UPDATED);
    emit(&mut event_emitter, SdkEvent::RULESETS_UPDATED);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);

    emit(&mut event_emitter, SdkEvent::INIT_SUCCESS);
    assert_eq!(third_counter.load(Ordering::SeqCst), 2);
}

#[test]
fn test_unsub_by_event_and_id() {
    let mut event_emitter = SdkEventEmitter::default();

    let (first_id, first_counter) = sub(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    let (_, second_counter) = sub(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    let (_, third_counter) = sub(&mut event_emitter, SdkEvent::INIT_SUCCESS);

    emit(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    emit(&mut event_emitter, SdkEvent::INIT_SUCCESS);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);

    event_emitter.unsubscribe_by_id(SdkEvent::RULESETS_UPDATED, &first_id);
    emit(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    emit(&mut event_emitter, SdkEvent::INIT_SUCCESS);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 2);
    assert_eq!(third_counter.load(Ordering::SeqCst), 2);
}

#[test]
fn test_unsub_all() {
    let mut event_emitter = SdkEventEmitter::default();

    let (_, first_counter) = sub(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    let (_, second_counter) = sub(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    let (_, third_counter) = sub(&mut event_emitter, SdkEvent::INIT_SUCCESS);

    emit(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    emit(&mut event_emitter, SdkEvent::INIT_SUCCESS);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);

    event_emitter.unsubscribe_all();

    emit(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    emit(&mut event_emitter, SdkEvent::INIT_SUCCESS);

    assert_eq!(first_counter.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    assert_eq!(third_counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_sub_all() {
    let mut event_emitter = SdkEventEmitter::default();
    let (_, counter) = sub(&mut event_emitter, SdkEvent::ALL);

    emit(&mut event_emitter, SdkEvent::RULESETS_UPDATED);
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    emit(&mut event_emitter, SdkEvent::INIT_SUCCESS);
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}
