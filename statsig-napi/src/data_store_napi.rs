use async_trait::async_trait;
use napi::{
  bindgen_prelude::{FromNapiValue, Promise, ToNapiValue},
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
  tokio::sync::oneshot,
  JsFunction, JsObject,
};
use napi_derive::napi;
use serde_json::json;
use sigstat::{
  data_store_interface::{DataAdapterResponse, DataStoreTrait, RequestPath},
  StatsigErr,
};

#[napi(object, js_name = "AdapterResponse")]
pub struct DataAdapterResponseNapi {
  pub result: Option<String>,
  pub time: Option<i64>,
}

impl Into<DataAdapterResponse> for DataAdapterResponseNapi {
  fn into(self) -> DataAdapterResponse {
    DataAdapterResponse {
      result: self.result,
      time: self.time.map(|time| time as u64),
    }
  }
}

pub struct DataStoreNapi {
  pub init_fn: Option<ThreadsafeFunction<()>>,
  pub get_fn: Option<ThreadsafeFunction<String>>,
  pub set_fn: Option<ThreadsafeFunction<String>>,
  pub shutdown_fn: Option<ThreadsafeFunction<()>>,
  pub support_polling_updates_for_fn: Option<ThreadsafeFunction<String>>,
}

impl DataStoreNapi {
  pub fn new(interface: JsObject) -> Self {
    DataStoreNapi {
      init_fn: Self::get_and_wrap::<()>(&interface, "initialize"),
      get_fn: Self::get_and_wrap::<String>(&interface, "get"),
      set_fn: Self::get_and_wrap::<String>(&interface, "set"),
      shutdown_fn: Self::get_and_wrap::<()>(&interface, "shutdown"),
      support_polling_updates_for_fn: Self::get_and_wrap::<String>(
        &interface,
        "supportsPollingUpdatesFor",
      ),
    }
  }

  fn get_and_wrap<T: ToNapiValue>(
    interface: &JsObject,
    func_name: &str,
  ) -> Option<ThreadsafeFunction<T>> {
    if let Ok(Some(js_fun)) = interface.get::<_, JsFunction>(func_name) {
      let maybe_func = js_fun.create_threadsafe_function::<_, T, _, _>(
        0,
        |ctx: napi::threadsafe_function::ThreadSafeCallContext<T>| Ok(vec![ctx.value]),
      );
      match maybe_func {
        Ok(func) => Some(func),
        Err(_) => None,
      }
    } else {
      None
    }
  }

  async fn call_and_await_on_response<T: FromNapiValue, V: FromNapiValue + 'static>(
    &self,
    tsfn: ThreadsafeFunction<T>,
    arg: T,
  ) -> Result<V, StatsigErr> {
    let (tx, rx) = oneshot::channel();
    tsfn.call_with_return_value(
      Ok(arg),
      ThreadsafeFunctionCallMode::Blocking,
      move |value: V| {
        let _res = tx.send(value);
        Ok(())
      },
    );
    rx.await
      .map_err(|e| StatsigErr::DataStoreFailure(e.to_string()))
  }
}

#[async_trait]
impl DataStoreTrait for DataStoreNapi {
  async fn initialize(&self) -> Result<(), StatsigErr> {
    match self.init_fn.clone() {
      Some(func) => {
        let call_res = self
          .call_and_await_on_response::<(), Promise<()>>(func, ())
          .await?;
        call_res
          .await
          .map_err(|e| StatsigErr::CustomError(e.reason))
      }
      None => Err(StatsigErr::DataStoreFailure(
        "No init function found".to_string(),
      )),
    }
  }

  async fn shutdown(&self) -> Result<(), StatsigErr> {
    match self.shutdown_fn.clone() {
      Some(func) => {
        let call_res = self
          .call_and_await_on_response::<(), Promise<()>>(func, ())
          .await?;
        call_res
          .await
          .map_err(|e| StatsigErr::CustomError(e.reason))
      }
      None => Err(StatsigErr::DataStoreFailure(
        "No shutdown function found".to_string(),
      )),
    }
  }

  async fn get(&self, key: &str) -> Result<DataAdapterResponse, StatsigErr> {
    match self.get_fn.clone() {
      Some(func) => {
        let call_res = self
          .call_and_await_on_response::<String, Promise<DataAdapterResponseNapi>>(
            func,
            key.to_string(),
          )
          .await?;
        call_res
          .await
          .map(|response| response.into())
          .map_err(|e| StatsigErr::CustomError(e.reason))
      }
      None => Err(StatsigErr::DataStoreFailure(
        "No init function found".to_string(),
      )),
    }
  }

  //TODO: properly implement this one
  async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
    match self.set_fn.clone() {
      Some(func) => {
        let call_res = self
          .call_and_await_on_response::<String, Promise<()>>(
            func,
            json!({"key": key,"value": value,"time": time}).to_string(),
          )
          .await?;
        call_res
          .await
          .map_err(|e| StatsigErr::CustomError(e.reason))
      }
      None => Err(StatsigErr::DataStoreFailure(
        "No shutdown function found".to_string(),
      )),
    }
  }

  async fn support_polling_updates_for(&self, key: RequestPath) -> bool {
    match self.support_polling_updates_for_fn.clone() {
      Some(func) => self
        .call_and_await_on_response::<String, bool>(func, key.to_string())
        .await
        .unwrap_or(false),
      None => false,
    }
  }
}
