use std::collections::HashMap;
use std::sync::Arc;

use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::{bindgen_prelude::ToNapiValue, JsFunction, JsObject};
use serde_json::json;
use sigstat::{IObservabilityClient, IOpsStatsEventObserver};

#[derive(Clone)]
pub struct ObservabilityClient {
  pub init_fn: Option<ThreadsafeFunction<()>>,
  pub increment_fn: Option<ThreadsafeFunction<String>>,
  pub gauge_fn: Option<ThreadsafeFunction<String>>,
  pub dist_fn: Option<ThreadsafeFunction<String>>,
}

impl ObservabilityClient {
  pub fn new(interfaces: JsObject) -> Self {
    ObservabilityClient {
      init_fn: Self::get_and_wrap::<()>(&interfaces, "init"),
      increment_fn: Self::get_and_wrap::<String>(&interfaces, "increment"),
      gauge_fn: Self::get_and_wrap::<String>(&interfaces, "gauge"),
      dist_fn: Self::get_and_wrap::<String>(&interfaces, "dist"),
    }
  }

  fn get_and_wrap<T: ToNapiValue>(
    interfaces: &JsObject,
    func_name: &str,
  ) -> Option<ThreadsafeFunction<T>> {
    if let Ok(Some(js_fun)) = interfaces.get::<_, JsFunction>(func_name) {
      Some(
        js_fun
          .create_threadsafe_function::<_, T, _, _>(
            0,
            |ctx: napi::threadsafe_function::ThreadSafeCallContext<T>| Ok(vec![ctx.value]),
          )
          .unwrap(),
      )
    } else {
      None
    }
  }
}

impl IObservabilityClient for ObservabilityClient {
  fn init(&self) {
    match self.init_fn.clone() {
      Some(func) => {
        func.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
      }
      None => {}
    }
  }

  fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
    match self.increment_fn.clone() {
      Some(func) => {
        let args = json!({
            "metric_name": metric_name,
            "value": value,
            "tags": tags,
        })
        .to_string();
        func.call(Ok(args), ThreadsafeFunctionCallMode::NonBlocking);
      }
      None => {}
    }
  }

  fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
    match self.gauge_fn.clone() {
      Some(func) => {
        let args = json!({
            "metric_name": metric_name,
            "value": value,
            "tags": tags,
        })
        .to_string();
        func.call(Ok(args), ThreadsafeFunctionCallMode::NonBlocking);
      }
      None => {}
    }
  }

  fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
    match self.dist_fn.clone() {
      Some(func) => {
        let args: String = json!({
            "metric_name": metric_name,
            "value": value,
            "tags": tags,
        })
        .to_string();
        func.call(Ok(args), ThreadsafeFunctionCallMode::NonBlocking);
      }
      None => {}
    }
  }

  fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn IOpsStatsEventObserver> {
    self
  }
}
