use crate::statsig_napi::StatsigNapiInternal;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use statsig_rust::{log_e, sdk_event_emitter::SubscriptionID};
use std::sync::Arc;

type EmitFnArgs = String;
type EmitFn = ThreadsafeFunction<EmitFnArgs, (), EmitFnArgs, false, true>;

const TAG: &str = "SdkEventEmitterNapi";

#[napi]
impl StatsigNapiInternal {
    #[napi(js_name = "__INTERNAL_subscribe")]
    pub fn __internal_subscribe(
        &self,
        #[napi(ts_arg_type = "SdkEvent")] event_name: String,
        #[napi(ts_arg_type = "(raw: string) => void")] callback: Arc<EmitFn>,
    ) -> String {
        let sub_id = self
            .inner
            .event_emitter
            .subscribe(event_name.as_str(), move |event| {
                if let Some(value) = event.to_raw_json_string() {
                    callback.call(value, ThreadsafeFunctionCallMode::Blocking);
                }
            });

        sub_id.encode()
    }

    #[napi]
    pub fn unsubscribe(&self, #[napi(ts_arg_type = "SdkEvent")] event_name: String) {
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
