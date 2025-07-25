use crate::statsig_napi::StatsigNapiInternal;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use statsig_rust::{log_e, sdk_event_emitter::SubscriptionID};
use std::sync::Arc;

type MetricFnArgs = serde_json::Map<String, serde_json::Value>;
type MetricFn = ThreadsafeFunction<MetricFnArgs, (), MetricFnArgs, false, true>;

const TAG: &str = "SdkEventEmitterNapi";

#[napi]
impl StatsigNapiInternal {
    #[napi]
    pub fn subscribe(
        &self,
        #[napi(
            ts_arg_type = "'*' | 'gate_evaluated' | 'dynamic_config_evaluated' | 'experiment_evaluated' | 'layer_evaluated'"
        )]
        event_name: String,
        #[napi(ts_arg_type = "(event: any) => void")] callback: Arc<MetricFn>,
    ) -> String {
        let sub_id = self
            .inner
            .event_emitter
            .subscribe(event_name.as_str(), move |event| {
                callback.call(event.to_json_map(), ThreadsafeFunctionCallMode::Blocking);
            });

        sub_id.encode()
    }

    #[napi]
    pub fn unsubscribe(
        &self,
        #[napi(
            ts_arg_type = "'*' | 'gate_evaluated' | 'dynamic_config_evaluated' | 'experiment_evaluated' | 'layer_evaluated'"
        )]
        event_name: String,
    ) {
        self.inner.event_emitter.unsubscribe(event_name.as_str());
    }

    #[napi]
    pub fn unsubscribe_by_id(&self, subscription_id: String) {
        let sub_id = match SubscriptionID::decode(&subscription_id) {
            Some(sub_id) => sub_id,
            None => {
                log_e!(TAG, "Invalid subscription ID: {}", subscription_id);
                return;
            }
        };
        self.inner.event_emitter.unsubscribe_by_id(&sub_id);
    }

    #[napi]
    pub fn unsubscribe_all(&self) {
        self.inner.event_emitter.unsubscribe_all();
    }
}
