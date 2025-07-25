use crate::{
    event_emitter::SdkEvent,
    log_e,
    statsig_types::{DynamicConfig, Experiment, Layer},
    Statsig,
};
use dashmap::DashMap;
use std::{borrow::Cow, ops::Deref};

const TAG: &str = "StatsigEventEmitter";

struct Listener {
    id: String,
    callback: Box<dyn Fn(SdkEvent) + Send + Sync>,
}

#[derive(Default)]
pub struct StatsigEventEmitter {
    listeners: DashMap<usize, Vec<Listener>>,
}

impl StatsigEventEmitter {
    pub fn subscribe<F>(&self, event: &str, callback: F) -> String
    where
        F: Fn(SdkEvent) + Send + Sync + 'static,
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
            .for_each(|listener| (listener.callback)(event.clone()));
    }
}

impl Deref for Statsig {
    type Target = StatsigEventEmitter;

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
