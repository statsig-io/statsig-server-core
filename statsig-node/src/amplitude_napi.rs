use napi::{
    bindgen_prelude::{FnArgs, Promise},
    threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use napi_derive::napi;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use statsig_rust::{Amplitude as RustAmplitude, AmplitudeEvent as RustEvent};
use std::{collections::HashMap, sync::Arc};

#[napi(object)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmplitudeOptions {
    pub server_url: Option<String>,
    pub timeout_ms: Option<u32>,
    pub min_id_length: Option<i32>,
    pub headers: Option<HashMap<String, String>>,
}
impl AmplitudeOptions {
    fn into_rust(self) -> statsig_rust::AmplitudeOptions {
        statsig_rust::AmplitudeOptions {
            server_url: self
                .server_url
                .unwrap_or_else(|| statsig_rust::AMPLITUDE_API_URL.into()),
            timeout_ms: self.timeout_ms.map(u64::from).unwrap_or(10_000),
            min_id_length: self.min_id_length,
            headers: self.headers.unwrap_or_default(),
        }
    }
}

#[napi(object)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmplitudePlan {
    pub branch: Option<String>,
    pub source: Option<String>,
    pub version: Option<String>,
    pub version_id: Option<String>,
}
#[napi(object)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmplitudeIngestionMetadata {
    pub source_name: Option<String>,
    pub source_version: Option<String>,
}

#[napi(object)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmplitudeEvent {
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dma: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idfa: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idfv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_lng: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_properties: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_properties: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_properties: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<AmplitudePlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingestion_metadata: Option<AmplitudeIngestionMetadata>,
}
impl AmplitudeEvent {
    fn into_rust(self) -> napi::Result<RustEvent> {
        let mut event: RustEvent =
            serde_json::from_value(serde_json::to_value(self).map_err(napi_error)?)
                .map_err(napi_error)?;
        event
            .library
            .get_or_insert_with(|| format!("amplitude-node-core/{}", env!("CARGO_PKG_VERSION")));
        Ok(event)
    }
}
fn middleware_payload(mut value: Value) -> napi::Result<statsig_rust::AmplitudeMiddlewarePayload> {
    // N-API's JSON bridge represents JS numbers as f64, including timestamps.
    // Reconstruct Java's integer fields without truncating fractional values.
    if let Some(event) = value.get_mut("event").and_then(Value::as_object_mut) {
        for key in ["time", "timestamp", "sessionId", "eventId", "quantity"] {
            if let Some(number) = event.get(key).and_then(Value::as_f64) {
                if number.fract() != 0.0 || number.abs() > 9_007_199_254_740_991.0 {
                    return Err(napi_error(
                        "Middleware integer fields must be safe integers",
                    ));
                }
                event.insert(key.into(), serde_json::json!(number as i64));
            }
        }
    }
    serde_json::from_value(value).map_err(napi_error)
}

fn napi_error(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

type CallbackArgs = FnArgs<(Value, u32, String)>;
type CallbackFn = ThreadsafeFunction<CallbackArgs, (), CallbackArgs, false, true>;
fn callback(callback: Arc<CallbackFn>) -> statsig_rust::AmplitudeCallback {
    Arc::new(move |event, status, message| {
        if let Ok(event) = serde_json::to_value(event) {
            callback.call(
                (event, u32::from(status), message.to_owned()).into(),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
    })
}

type LoggerArgs = FnArgs<(String, String)>;
type LoggerFn = ThreadsafeFunction<LoggerArgs, (), LoggerArgs, false, true>;
#[napi(object, object_to_js = false)]
pub struct AmplitudeLogger {
    #[napi(ts_type = "(tag: string, message: string) => void")]
    pub debug: Option<Arc<LoggerFn>>,
    #[napi(ts_type = "(tag: string, message: string) => void")]
    pub warn: Option<Arc<LoggerFn>>,
    #[napi(ts_type = "(tag: string, message: string) => void")]
    pub error: Option<Arc<LoggerFn>>,
}

#[napi]
pub struct AmplitudeMiddlewareNext {
    inner: Mutex<Option<statsig_rust::AmplitudeMiddlewareNext>>,
}
#[napi]
impl AmplitudeMiddlewareNext {
    #[napi]
    pub async fn run(&self, payload: Value) -> napi::Result<Value> {
        let next = self
            .inner
            .lock()
            .take()
            .ok_or_else(|| napi_error("Middleware next can only be called once"))?;
        let payload = middleware_payload(payload)?;
        serde_json::to_value(next.run(payload).await.map_err(napi_error)?).map_err(napi_error)
    }
}
type MiddlewareArgs = FnArgs<(Value, AmplitudeMiddlewareNext)>;
type MiddlewareFn = ThreadsafeFunction<MiddlewareArgs, Promise<Value>, MiddlewareArgs, false, true>;
struct NodeMiddleware {
    callback: Arc<MiddlewareFn>,
}
#[allow(clippy::double_must_use)]
#[async_trait::async_trait]
impl statsig_rust::AmplitudeMiddleware for NodeMiddleware {
    async fn run(
        &self,
        payload: statsig_rust::AmplitudeMiddlewarePayload,
        next: statsig_rust::AmplitudeMiddlewareNext,
    ) -> Result<Option<statsig_rust::AmplitudeMiddlewarePayload>, statsig_rust::AmplitudeError>
    {
        let error =
            |error: napi::Error| statsig_rust::AmplitudeError::Middleware(error.to_string());
        let payload = serde_json::to_value(payload)
            .map_err(|e| statsig_rust::AmplitudeError::Middleware(e.to_string()))?;
        let result = self
            .callback
            .call_async(
                (
                    payload,
                    AmplitudeMiddlewareNext {
                        inner: Mutex::new(Some(next)),
                    },
                )
                    .into(),
            )
            .await
            .map_err(error)?
            .await
            .map_err(error)?;
        if result.is_null() {
            Ok(None)
        } else {
            middleware_payload(result)
                .map(Some)
                .map_err(|e| statsig_rust::AmplitudeError::Middleware(e.to_string()))
        }
    }
}

#[napi]
#[derive(Default)]
pub struct AmplitudeNapi {
    inner: RustAmplitude,
}
#[napi]
impl AmplitudeNapi {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustAmplitude::new(),
        }
    }
    #[napi(factory)]
    pub fn get_instance(name: Option<String>) -> Self {
        Self {
            inner: RustAmplitude::get_instance(name.as_deref()),
        }
    }
    #[napi]
    pub fn init(&self, api_key: String, options: Option<AmplitudeOptions>) -> napi::Result<()> {
        self.inner
            .init(&api_key, options.map(AmplitudeOptions::into_rust))
            .map_err(napi_error)
    }
    #[napi]
    pub fn set_server_url(&self, url: String) {
        self.inner.set_server_url(&url);
    }
    #[napi]
    pub fn set_options(&self, options: Option<AmplitudeOptions>) {
        self.inner
            .set_options(options.map(AmplitudeOptions::into_rust));
    }
    #[napi]
    pub fn use_batch_mode(&self, batch: bool) {
        self.inner.use_batch_mode(batch);
    }
    #[napi]
    pub fn set_event_upload_threshold(&self, threshold: u32) {
        self.inner.set_event_upload_threshold(threshold as usize);
    }
    #[napi]
    pub fn set_event_upload_period_millis(&self, period: u32) {
        self.inner.set_event_upload_period_millis(u64::from(period));
    }
    #[napi]
    pub fn set_flush_timeout(&self, timeout: u32) {
        self.inner.set_flush_timeout(u64::from(timeout));
    }
    #[napi]
    pub fn set_record_throttled_id(&self, record: bool) {
        self.inner.set_record_throttled_id(record);
    }
    #[napi]
    pub fn set_send_thread_pool(&self, concurrency: u32) -> napi::Result<()> {
        self.inner.set_send_thread_pool(
            statsig_rust::AmplitudeThreadPool::new(concurrency as usize).map_err(napi_error)?,
        );
        Ok(())
    }
    #[napi]
    pub fn set_retry_thread_pool(&self, concurrency: u32) -> napi::Result<()> {
        self.inner.set_retry_thread_pool(
            statsig_rust::AmplitudeThreadPool::new(concurrency as usize).map_err(napi_error)?,
        );
        Ok(())
    }
    #[napi]
    pub fn set_plan(&self, plan: Option<AmplitudePlan>) -> napi::Result<()> {
        let plan = plan
            .map(|v| serde_json::from_value(serde_json::to_value(v)?))
            .transpose()
            .map_err(napi_error)?;
        self.inner.set_plan(plan);
        Ok(())
    }
    #[napi]
    pub fn set_ingestion_metadata(
        &self,
        metadata: Option<AmplitudeIngestionMetadata>,
    ) -> napi::Result<()> {
        let metadata = metadata
            .map(|v| serde_json::from_value(serde_json::to_value(v)?))
            .transpose()
            .map_err(napi_error)?;
        self.inner.set_ingestion_metadata(metadata);
        Ok(())
    }
    #[napi]
    pub fn set_proxy(&self, proxy: Option<crate::statsig_options_napi::ProxyConfig>) {
        self.inner.set_proxy(proxy.map(Into::into));
    }

    #[napi]
    pub fn set_log_mode(&self, mode: String) -> napi::Result<()> {
        let mode = match mode.to_uppercase().as_str() {
            "DEBUG" => statsig_rust::AmplitudeLogMode::Debug,
            "WARN" => statsig_rust::AmplitudeLogMode::Warn,
            "ERROR" => statsig_rust::AmplitudeLogMode::Error,
            "OFF" => statsig_rust::AmplitudeLogMode::Off,
            _ => return Err(napi_error("Invalid Amplitude log mode")),
        };
        self.inner.set_log_mode(mode);
        Ok(())
    }
    #[napi]
    pub fn set_logger(&self, logger: AmplitudeLogger) {
        self.inner.set_logger(statsig_rust::AmplitudeLogger {
            mode: statsig_rust::AmplitudeLogMode::Error,
            callback: Arc::new(move |mode, tag, message| {
                let callback_fn = match mode {
                    statsig_rust::AmplitudeLogMode::Debug => &logger.debug,
                    statsig_rust::AmplitudeLogMode::Warn => &logger.warn,
                    statsig_rust::AmplitudeLogMode::Error => &logger.error,
                    statsig_rust::AmplitudeLogMode::Off => return,
                };
                if let Some(callback_fn) = callback_fn {
                    callback_fn.call(
                        (tag.to_owned(), message.to_owned()).into(),
                        ThreadsafeFunctionCallMode::NonBlocking,
                    );
                }
            }),
        });
    }
    #[napi]
    pub fn set_callbacks(
        &self,
        #[napi(
            ts_arg_type = "((event: AmplitudeEvent, status: number, message: string) => void) | null"
        )]
        callback_fn: Option<Arc<CallbackFn>>,
    ) {
        self.inner.set_callbacks(callback_fn.map(callback));
    }
    #[napi]
    pub fn add_event_middleware(
        &self,
        #[napi(
            ts_arg_type = "(payload: AmplitudeMiddlewarePayload, next: AmplitudeMiddlewareNext) => Promise<AmplitudeMiddlewarePayload | null>"
        )]
        callback_fn: Arc<MiddlewareFn>,
    ) {
        self.inner.add_event_middleware(Arc::new(NodeMiddleware {
            callback: callback_fn,
        }));
    }
    #[napi]
    pub async fn log_event(
        &self,
        event: AmplitudeEvent,
        #[napi(
            ts_arg_type = "((event: AmplitudeEvent, status: number, message: string) => void) | null"
        )]
        callback_fn: Option<Arc<CallbackFn>>,
        extra: Option<Value>,
    ) -> napi::Result<()> {
        self.inner
            .log_event_with(
                event.into_rust()?,
                callback_fn.map(callback),
                extra.unwrap_or(Value::Null),
            )
            .await
            .map_err(napi_error)
    }
    #[napi]
    pub async fn track(&self, event: AmplitudeEvent) -> napi::Result<()> {
        self.inner
            .track(event.into_rust()?)
            .await
            .map_err(napi_error)
    }
    #[napi]
    pub fn should_wait(&self, event: AmplitudeEvent) -> napi::Result<bool> {
        Ok(self.inner.should_wait(&event.into_rust()?))
    }
    #[napi]
    pub async fn flush_events(&self) -> napi::Result<()> {
        self.inner.flush_events_and_wait().await.map_err(napi_error)
    }
    #[napi]
    pub async fn shutdown(&self) -> napi::Result<()> {
        self.inner.shutdown().await.map_err(napi_error)
    }
}
