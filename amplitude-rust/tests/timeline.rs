use amplitude_rust::{
    Delivery, DeliveryResult, DestinationPlugin, Event, EventPlugin, Plugin, Timeline,
};
use std::sync::{Arc, Mutex};

struct Processor(fn(Event) -> Option<Event>);

impl EventPlugin for Processor {
    fn execute(&self, event: Event) -> Option<Event> {
        (self.0)(event)
    }
}

struct Destination {
    code: u16,
    received: Arc<Mutex<Vec<String>>>,
}

impl DestinationPlugin for Destination {
    fn execute(&self, event: Event) -> Result<Delivery, amplitude_rust::AmplitudeError> {
        self.received.lock().unwrap().push(event.event_type.clone());
        Ok(Delivery::Ready(DeliveryResult {
            event,
            code: self.code,
            message: String::new(),
        }))
    }
}

#[tokio::test]
async fn runs_plugins_in_stage_order_and_returns_the_first_destination_result() {
    let timeline = Timeline::new(false);
    let event = Event::new("original");
    let result = timeline.apply(event.clone()).unwrap().wait().await.unwrap();
    assert_eq!(result.code, 100);
    assert_eq!(
        result.message,
        "Event not tracked, no destination plugins on the instance"
    );

    timeline
        .add(Plugin::Enrichment(Arc::new(Processor(|mut event| {
            event.event_type.push_str("+enriched");
            Some(event)
        }))))
        .unwrap();
    timeline
        .add(Plugin::Before(Arc::new(Processor(|mut event| {
            event.event_type.push_str("+before");
            Some(event)
        }))))
        .unwrap();
    let filter = timeline
        .add(Plugin::Before(Arc::new(Processor(|_| None))))
        .unwrap();
    let received = Arc::new(Mutex::new(Vec::new()));
    for code in [200, 400] {
        timeline
            .add(Plugin::Destination(Arc::new(Destination {
                code,
                received: Arc::clone(&received),
            })))
            .unwrap();
    }
    assert_eq!(
        timeline
            .apply(event.clone())
            .unwrap()
            .wait()
            .await
            .unwrap()
            .code,
        0
    );
    assert!(received.lock().unwrap().is_empty());
    assert!(timeline.remove(filter));
    assert_eq!(
        timeline.apply(event).unwrap().wait().await.unwrap().code,
        200
    );
    assert_eq!(*received.lock().unwrap(), ["original+before+enriched"; 2]);
}
