use crate::statsig_napi::StatsigNapiInternal;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::sync::Arc;

type MetricFnArgs = serde_json::Map<String, serde_json::Value>;
type MetricFn = ThreadsafeFunction<MetricFnArgs, (), MetricFnArgs, false, true>;

#[napi]
impl StatsigNapiInternal {
    #[napi]
    pub fn subscribe(
        &self,
        #[napi(
            ts_arg_type = "'*' | 'init_success' | 'init_failure' | 'rulesets_updated' | 'gate_evaluated' | 'dynamic_config_evaluated' | 'experiment_evaluated' | 'layer_evaluated'"
        )]
        event_name: String,
        #[napi(ts_arg_type = "(event: any) => void")] callback: Arc<MetricFn>,
    ) -> String {
        self.inner
            .event_emitter
            .subscribe(event_name.as_str(), move |event| {
                callback.call(event.to_json_map(), ThreadsafeFunctionCallMode::Blocking);
            })
    }

    #[napi]
    pub fn unsubscribe(
        &self,
        #[napi(
            ts_arg_type = "'*' | 'init_success' | 'init_failure' | 'rulesets_updated' | 'gate_evaluated' | 'dynamic_config_evaluated' | 'experiment_evaluated' | 'layer_evaluated'"
        )]
        event_name: String,
    ) {
        self.inner.event_emitter.unsubscribe(event_name.as_str());
    }

    #[napi]
    pub fn unsubscribe_by_id(
        &self,
        #[napi(
            ts_arg_type = "'*' | 'init_success' | 'init_failure' | 'rulesets_updated' | 'gate_evaluated' | 'dynamic_config_evaluated' | 'experiment_evaluated' | 'layer_evaluated'"
        )]
        event_name: String,
        subscription_id: String,
    ) {
        self.inner
            .event_emitter
            .unsubscribe_by_id(event_name.as_str(), &subscription_id);
    }

    #[napi]
    pub fn unsubscribe_all(&self) {
        self.inner.event_emitter.unsubscribe_all();
    }
}
