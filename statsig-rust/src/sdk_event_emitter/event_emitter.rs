use crate::{
    log_e,
    sdk_event_emitter::{SdkEvent, SdkEventCode},
    statsig_types::{DynamicConfig, Experiment, Layer},
    Statsig,
};
use dashmap::DashMap;
use std::{borrow::Cow, ops::Deref};

const TAG: &str = "SdkEventEmitter";

struct Listener {
    sub_id_value: String,
    callback: Box<dyn Fn(SdkEvent) + Send + Sync>,
}

#[derive(Clone)]
pub struct SubscriptionID {
    value: String,
    event: String,
}

impl SubscriptionID {
    pub fn new(event: &str) -> Self {
        Self {
            value: uuid::Uuid::new_v4().to_string(),
            event: event.to_string(),
        }
    }

    pub fn error() -> Self {
        Self {
            value: "ERROR".to_string(),
            event: "ERROR".to_string(),
        }
    }

    pub fn decode(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('@').collect();
        if parts.len() != 2 {
            return None;
        }

        Some(Self {
            value: parts[0].to_string(),
            event: parts[1].to_string(),
        })
    }

    pub fn encode(self) -> String {
        let mut encoded = self.value;
        encoded.push('@');
        encoded.push_str(&self.event);
        encoded
    }
}

#[derive(Default)]
pub struct SdkEventEmitter {
    listeners: DashMap<u8, Vec<Listener>>,
}

impl SdkEventEmitter {
    pub fn subscribe<F>(&self, event: &str, callback: F) -> SubscriptionID
    where
        F: Fn(SdkEvent) + Send + Sync + 'static,
    {
        let code = SdkEventCode::from_name(event).as_raw();
        if code == 0 {
            log_e!(TAG, "Invalid event name: {}", event);
            return SubscriptionID::error();
        }

        let sub_id = SubscriptionID::new(event);

        self.listeners.entry(code).or_default().push(Listener {
            sub_id_value: sub_id.value.clone(),
            callback: Box::new(callback),
        });

        sub_id
    }

    pub fn unsubscribe(&self, event: &str) {
        let code = SdkEventCode::from_name(event).as_raw();
        self.listeners.remove(&code);
    }

    pub fn unsubscribe_by_id(&self, subscription_id: &SubscriptionID) {
        let code = SdkEventCode::from_name(&subscription_id.event).as_raw();
        let mut listeners = match self.listeners.get_mut(&code) {
            Some(listeners) => listeners,
            None => return,
        };

        listeners.retain(|listener| listener.sub_id_value != subscription_id.value);
    }

    pub fn unsubscribe_all(&self) {
        self.listeners.clear();
    }

    pub(crate) fn emit(&self, event: SdkEvent) {
        let all_code = SdkEventCode::from_name(SdkEvent::ALL).as_raw();
        self.emit_to_listeners(&event, self.listeners.get(&all_code).as_deref());

        let event_code = event.get_code().as_raw();
        self.emit_to_listeners(&event, self.listeners.get(&event_code).as_deref());
    }

    fn emit_to_listeners(&self, event: &SdkEvent, listeners: Option<&Vec<Listener>>) {
        let listeners = match listeners {
            Some(listeners) => listeners,
            None => return,
        };

        listeners
            .iter()
            .for_each(|listener| (listener.callback)(event.clone()));
    }
}

impl Deref for Statsig {
    type Target = SdkEventEmitter;

    fn deref(&self) -> &Self::Target {
        &self.event_emitter
    }
}

impl Statsig {
    pub(crate) fn emit_gate_evaluated(
        &self,
        gate_name: &str,
        rule_id: &str,
        value: bool,
        reason: &str,
    ) {
        self.emit(SdkEvent::GateEvaluated {
            gate_name: gate_name.into(),
            rule_id: rule_id.into(),
            value,
            reason: reason.into(),
        });
    }

    pub(crate) fn emit_dynamic_config_evaluated(&self, config: &DynamicConfig) {
        self.emit(SdkEvent::DynamicConfigEvaluated {
            dynamic_config: Cow::Borrowed(config),
        });
    }

    pub(crate) fn emit_experiment_evaluated(&self, experiment: &Experiment) {
        self.emit(SdkEvent::ExperimentEvaluated {
            experiment: Cow::Borrowed(experiment),
        });
    }

    pub(crate) fn emit_layer_evaluated(&self, layer: &Layer) {
        self.emit(SdkEvent::LayerEvaluated {
            layer: Cow::Borrowed(layer),
        });
    }
}
