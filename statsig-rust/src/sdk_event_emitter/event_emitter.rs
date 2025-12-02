use crate::{
    log_e,
    sdk_event_emitter::{SdkEvent, SdkEventCode},
    statsig_types::{DynamicConfig, Experiment, Layer},
    Statsig,
};
use dashmap::DashMap;
use std::ops::Deref;

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
            gate_name,
            rule_id,
            value,
            reason,
        });
    }

    #[cfg(feature = "ffi-support")]
    pub(crate) fn emit_gate_evaluated_parts(
        &self,
        gate_name: &str,
        reason: &str,
        eval_result: Option<&crate::evaluation::evaluator_result::EvaluatorResult>,
    ) {
        let mut rule_id = None;
        let mut value = false;

        if let Some(eval) = eval_result {
            rule_id = eval.rule_id.as_ref().map(|r| r.as_str());
            value = eval.bool_value;
        }

        self.emit(SdkEvent::GateEvaluated {
            gate_name,
            rule_id: rule_id.unwrap_or_default(),
            value,
            reason,
        });
    }

    pub(crate) fn emit_dynamic_config_evaluated(&self, config: &DynamicConfig) {
        self.emit(SdkEvent::DynamicConfigEvaluated {
            config_name: config.name.as_str(),
            reason: config.details.reason.as_str(),
            rule_id: Some(config.rule_id.as_str()),
            value: config.__evaluation.as_ref().map(|e| &e.value),
        });
    }

    #[cfg(feature = "ffi-support")]
    pub(crate) fn emit_dynamic_config_evaluated_parts(
        &self,
        config_name: &str,
        reason: &str,
        eval_result: Option<&crate::evaluation::evaluator_result::EvaluatorResult>,
    ) {
        let mut rule_id = None;
        let mut value = None;

        if let Some(eval) = eval_result {
            rule_id = eval.rule_id.as_ref().map(|r| r.as_str());
            value = eval.json_value.as_ref();
        }

        self.emit(SdkEvent::DynamicConfigEvaluated {
            config_name,
            reason,
            rule_id,
            value,
        });
    }

    pub(crate) fn emit_experiment_evaluated(&self, experiment: &Experiment) {
        self.emit(SdkEvent::ExperimentEvaluated {
            experiment_name: experiment.name.as_str(),
            reason: experiment.details.reason.as_str(),
            rule_id: Some(experiment.rule_id.as_str()),
            value: experiment.__evaluation.as_ref().map(|e| &e.value),
            group_name: experiment.group_name.as_deref(),
        });
    }

    #[cfg(feature = "ffi-support")]
    pub(crate) fn emit_experiment_evaluated_parts(
        &self,
        experiment_name: &str,
        reason: &str,
        eval_result: Option<&crate::evaluation::evaluator_result::EvaluatorResult>,
    ) {
        let mut rule_id = None;
        let mut value = None;
        let mut group_name = None;

        if let Some(eval) = eval_result {
            rule_id = eval.rule_id.as_ref().map(|r| r.as_str());
            value = eval.json_value.as_ref();
            group_name = eval.group_name.as_ref().map(|g| g.as_str());
        }

        self.emit(SdkEvent::ExperimentEvaluated {
            experiment_name,
            reason,
            rule_id,
            value,
            group_name,
        });
    }

    pub(crate) fn emit_layer_evaluated(&self, layer: &Layer) {
        self.emit(SdkEvent::LayerEvaluated {
            layer_name: layer.name.as_str(),
            reason: layer.details.reason.as_str(),
            rule_id: Some(layer.rule_id.as_str()),
        });
    }

    #[cfg(feature = "ffi-support")]
    pub(crate) fn emit_layer_evaluated_parts(
        &self,
        layer_name: &str,
        reason: &str,
        eval_result: Option<&crate::evaluation::evaluator_result::EvaluatorResult>,
    ) {
        let mut rule_id = None;

        if let Some(eval) = eval_result {
            rule_id = eval.rule_id.as_ref().map(|r| r.as_str());
        }

        self.emit(SdkEvent::LayerEvaluated {
            layer_name,
            reason,
            rule_id,
        });
    }
}
