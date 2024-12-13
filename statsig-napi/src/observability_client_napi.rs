use std::collections::HashMap;
use std::sync::Arc;

use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::{bindgen_prelude::ToNapiValue, JsFunction, JsObject};
use serde_json::json;
use sigstat::{log_e, ObservabilityClient, OpsStatsEventObserver};

const TAG: &str = stringify!(ObservabilityClientNapi);

#[derive(Clone)]
pub struct ObservabilityClientNapi {
  pub init_fn: Option<ThreadsafeFunction<()>>,
  pub increment_fn: Option<ThreadsafeFunction<String>>,
  pub gauge_fn: Option<ThreadsafeFunction<String>>,
  pub dist_fn: Option<ThreadsafeFunction<String>>,
}

impl ObservabilityClientNapi {
  pub fn new(interfaces: JsObject) -> Self {
    ObservabilityClientNapi {
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
      let threadsafe_fn = js_fun.create_threadsafe_function::<_, T, _, _>(
        0,
        |ctx: napi::threadsafe_function::ThreadSafeCallContext<T>| Ok(vec![ctx.value]),
      );

      match threadsafe_fn {
        Ok(threadsafe_fn) => Some(threadsafe_fn),
        Err(e) => {
          log_e!(TAG, "Failed to create threadsafe function: {}", e);
          None
        }
      }
    } else {
      None
    }
  }
}

impl ObservabilityClient for ObservabilityClientNapi {
  fn init(&self) {
    if let Some(func) = self.init_fn.clone() {
      func.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
    }
  }

  fn increment(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
    if let Some(func) = self.increment_fn.clone() {
      let args = json!({
          "metric_name": metric_name,
          "value": value,
          "tags": tags,
      })
      .to_string();
      func.call(Ok(args), ThreadsafeFunctionCallMode::NonBlocking);
    }
  }

  fn gauge(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
    if let Some(func) = self.gauge_fn.clone() {
      let args = json!({
          "metric_name": metric_name,
          "value": value,
          "tags": tags,
      })
      .to_string();
      func.call(Ok(args), ThreadsafeFunctionCallMode::NonBlocking);
    }
  }

  fn dist(&self, metric_name: String, value: f64, tags: Option<HashMap<String, String>>) {
    if let Some(func) = self.dist_fn.clone() {
      let args: String = json!({
          "metric_name": metric_name,
          "value": value,
          "tags": tags,
      })
      .to_string();
      func.call(Ok(args), ThreadsafeFunctionCallMode::NonBlocking);
    }
  }

  fn to_ops_stats_event_observer(self: Arc<Self>) -> Arc<dyn OpsStatsEventObserver> {
    self
  }
}
