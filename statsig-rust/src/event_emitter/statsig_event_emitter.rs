use crate::{event_emitter::SdkEvent, log_e};
use dashmap::DashMap;

const TAG: &str = "StatsigEventEmitter";

struct Listener {
    id: String,
    callback: Box<dyn Fn(&SdkEvent) + Send + Sync>,
}

#[derive(Default)]
pub struct StatsigEventEmitter {
    listeners: DashMap<usize, Vec<Listener>>,
}

impl StatsigEventEmitter {
    pub fn subscribe<F>(&self, event: &str, callback: F) -> String
    where
        F: Fn(&SdkEvent) + Send + Sync + 'static,
    {
        let code = SdkEvent::get_code_from_name(event);
        if code == 0 {
            log_e!(TAG, "Invalid event name: {}", event);
            return "ERROR".to_string();
        }

        let id = uuid::Uuid::new_v4().to_string();

        self.listeners.entry(code).or_default().push(Listener {
            id: id.clone(),
            callback: Box::new(callback),
        });

        id
    }

    pub fn unsubscribe(&self, event: &str) {
        let code = SdkEvent::get_code_from_name(event);
        self.listeners.remove(&code);
    }

    pub fn unsubscribe_by_id(&self, event: &str, id: String) {
        let code = SdkEvent::get_code_from_name(event);
        let mut listeners = match self.listeners.get_mut(&code) {
            Some(listeners) => listeners,
            None => return,
        };

        listeners.retain(|listener| listener.id != id);
    }

    pub fn unsubscribe_all(&self) {
        self.listeners.clear();
    }

    #[allow(dead_code)] // temp: used and removed in next PR
    pub(crate) fn emit(&self, event: SdkEvent) {
        self.emit_to_listeners(
            &event,
            self.listeners
                .get(&SdkEvent::get_code_from_name(SdkEvent::ALL))
                .as_deref(),
        );

        self.emit_to_listeners(&event, self.listeners.get(&event.get_code()).as_deref());
    }

    fn emit_to_listeners(&self, event: &SdkEvent, listeners: Option<&Vec<Listener>>) {
        let listeners = match listeners {
            Some(listeners) => listeners,
            None => return,
        };

        listeners
            .iter()
            .for_each(|listener| (listener.callback)(event));
    }
}
