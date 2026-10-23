#![doc = include_str!("../AMPLITUDE.md")]
mod event;
mod response;
pub use event::*;
pub use response::*;

use crate::networking::{
    providers::get_network_provider, proxy_config::ProxyConfig, HttpMethod, NetworkProvider,
    RequestArgs,
};
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
    time::Duration,
};
use tokio::{
    runtime::Handle,
    sync::{oneshot, watch, Notify, Semaphore},
};

pub const AMPLITUDE_API_URL: &str = "https://api2.amplitude.com/2/httpapi";
pub const AMPLITUDE_BATCH_API_URL: &str = "https://api2.amplitude.com/batch";
pub const AMPLITUDE_RETRY_TIMEOUTS: [u64; 12] = [
    100, 100, 200, 200, 400, 400, 800, 800, 1600, 1600, 3200, 3200,
];
pub const AMPLITUDE_MAX_CACHED_EVENTS: usize = 16_000;

pub type AmplitudeCallback = Arc<dyn Fn(&AmplitudeEvent, u16, &str) + Send + Sync>;
pub type AmplitudeLogCallback = Arc<dyn Fn(AmplitudeLogMode, &str, &str) + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AmplitudeOptions {
    // Kept for compatibility with the initial demo's init options.
    pub server_url: String,
    pub timeout_ms: u64,
    pub min_id_length: Option<i32>,
    pub headers: HashMap<String, String>,
}
impl Default for AmplitudeOptions {
    fn default() -> Self {
        Self {
            server_url: AMPLITUDE_API_URL.into(),
            timeout_ms: 10_000,
            min_id_length: None,
            headers: HashMap::new(),
        }
    }
}
impl AmplitudeOptions {
    pub fn set_min_id_length(&mut self, value: Option<i32>) -> &mut Self {
        self.min_id_length = value;
        self
    }
    pub fn set_headers(&mut self, value: HashMap<String, String>) -> &mut Self {
        self.headers = value;
        self
    }
    pub fn add_header(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.headers.insert(key.into(), value.into());
        self
    }
    pub fn to_json_object(&self) -> Value {
        self.min_id_length
            .map_or_else(|| json!({}), |value| json!({"min_id_length":value}))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AmplitudeError {
    NotInitialized,
    Shutdown,
    InvalidApiKey,
    InvalidInput(String),
    Network(String),
    Api { status: u16, body: String },
    Middleware(String),
}
impl fmt::Display for AmplitudeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInitialized => f.write_str("Call Amplitude.init before logging events"),
            Self::Shutdown => f.write_str("Amplitude client has shut down"),
            Self::InvalidApiKey => f.write_str("Invalid API key"),
            Self::InvalidInput(message) | Self::Network(message) | Self::Middleware(message) => {
                f.write_str(message)
            }
            Self::Api { status, body } => write!(f, "Amplitude returned HTTP {status}: {body}"),
        }
    }
}
impl std::error::Error for AmplitudeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AmplitudeLogMode {
    Debug,
    Warn,
    Error,
    Off,
}

#[derive(Clone)]
pub struct AmplitudeLogger {
    pub mode: AmplitudeLogMode,
    pub callback: AmplitudeLogCallback,
}
impl Default for AmplitudeLogger {
    fn default() -> Self {
        Self {
            mode: AmplitudeLogMode::Error,
            callback: Arc::new(|mode, tag, message| {
                if mode == AmplitudeLogMode::Error {
                    eprintln!("{tag}: {message}");
                } else {
                    println!("{tag}: {message}");
                }
            }),
        }
    }
}

/// Java ExecutorService equivalent: a Tokio runtime handle and a concurrency limit.
#[derive(Clone)]
pub struct AmplitudeThreadPool {
    handle: Handle,
    semaphore: Arc<Semaphore>,
}
impl AmplitudeThreadPool {
    pub fn new(concurrency: usize) -> Result<Self, AmplitudeError> {
        let handle = crate::StatsigRuntime::get_runtime()
            .get_handle()
            .map_err(|error| AmplitudeError::Network(error.to_string()))?;
        Self::with_handle(handle, concurrency)
    }
    pub fn with_handle(handle: Handle, concurrency: usize) -> Result<Self, AmplitudeError> {
        if concurrency == 0 {
            return Err(AmplitudeError::InvalidInput(
                "Thread pool concurrency must be positive".into(),
            ));
        }
        Ok(Self {
            handle,
            semaphore: Arc::new(Semaphore::new(concurrency)),
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmplitudeMiddlewarePayload {
    pub event: AmplitudeEvent,
    pub extra: Value,
}

#[allow(clippy::double_must_use)]
#[async_trait]
pub trait AmplitudeMiddleware: Send + Sync {
    async fn run(
        &self,
        payload: AmplitudeMiddlewarePayload,
        next: AmplitudeMiddlewareNext,
    ) -> Result<Option<AmplitudeMiddlewarePayload>, AmplitudeError>;
}

pub struct AmplitudeMiddlewareNext {
    middleware: Arc<Vec<Arc<dyn AmplitudeMiddleware>>>,
    position: usize,
}
impl AmplitudeMiddlewareNext {
    pub fn run(
        self,
        payload: AmplitudeMiddlewarePayload,
    ) -> Pin<
        Box<dyn Future<Output = Result<Option<AmplitudeMiddlewarePayload>, AmplitudeError>> + Send>,
    > {
        Box::pin(async move {
            if let Some(middleware) = self.middleware.get(self.position).cloned() {
                middleware
                    .run(
                        payload,
                        Self {
                            middleware: self.middleware,
                            position: self.position + 1,
                        },
                    )
                    .await
            } else {
                Ok(Some(payload))
            }
        })
    }
}

struct State {
    api_key: Option<String>,
    server_url: Option<String>,
    batch: bool,
    options: Option<AmplitudeOptions>,
    timeout_ms: u64,
    flush_timeout_ms: u64,
    threshold: usize,
    period_ms: u64,
    plan: Option<AmplitudePlan>,
    ingestion_metadata: Option<AmplitudeIngestionMetadata>,
    logger: AmplitudeLogger,
    callback: Option<AmplitudeCallback>,
    middleware: Vec<Arc<dyn AmplitudeMiddleware>>,
    proxy: Option<ProxyConfig>,
    queue: Vec<AmplitudeEvent>,
    scheduled: bool,
    closed: bool,
    record_throttled: bool,
    send_pool: AmplitudeThreadPool,
    retry_pool: AmplitudeThreadPool,
}
struct Inner {
    name: Option<String>,
    state: Mutex<State>,
    network: RwLock<Option<Arc<dyn NetworkProvider>>>,
    active: AtomicUsize,
    retry_count: AtomicUsize,
    idle: Notify,
    stop: watch::Sender<bool>,
    throttled: Mutex<HashMap<(bool, String), usize>>,
}
#[derive(Clone)]
pub struct Amplitude {
    inner: Arc<Inner>,
}
static INSTANCES: OnceLock<Mutex<HashMap<String, Amplitude>>> = OnceLock::new();

impl Default for Amplitude {
    fn default() -> Self {
        Self::new()
    }
}
impl Amplitude {
    pub fn new() -> Self {
        Self::create(None)
    }
    fn create(name: Option<String>) -> Self {
        let (stop, _) = watch::channel(false);
        Self {
            inner: Arc::new(Inner {
                name,
                state: Mutex::new(State {
                    api_key: None,
                    server_url: None,
                    batch: false,
                    options: None,
                    timeout_ms: 10_000,
                    flush_timeout_ms: 0,
                    threshold: 10,
                    period_ms: 10_000,
                    plan: None,
                    ingestion_metadata: None,
                    logger: AmplitudeLogger::default(),
                    callback: None,
                    middleware: Vec::new(),
                    proxy: None,
                    queue: Vec::new(),
                    scheduled: false,
                    closed: false,
                    record_throttled: false,
                    send_pool: AmplitudeThreadPool::new(20).expect("Tokio runtime creation failed"),
                    retry_pool: AmplitudeThreadPool::new(10)
                        .expect("Tokio runtime creation failed"),
                }),
                network: RwLock::new(None),
                active: AtomicUsize::new(0),
                retry_count: AtomicUsize::new(0),
                idle: Notify::new(),
                stop,
                throttled: Mutex::new(HashMap::new()),
            }),
        }
    }
    pub fn get_instance(name: Option<&str>) -> Self {
        let name = name.unwrap_or("").to_owned();
        INSTANCES
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .entry(name.clone())
            .or_insert_with(|| Self::create(Some(name)))
            .clone()
    }
    pub fn is_same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
    pub fn init(
        &self,
        api_key: &str,
        options: Option<AmplitudeOptions>,
    ) -> Result<(), AmplitudeError> {
        if api_key.trim().is_empty() {
            return Err(AmplitudeError::InvalidInput(
                "api_key must not be empty".into(),
            ));
        }
        let network = get_network_provider()
            .upgrade()
            .ok_or_else(|| AmplitudeError::Network("No network provider available".into()))?;
        let mut state = self.inner.state.lock();
        if state.closed {
            return Err(AmplitudeError::Shutdown);
        }
        if let Some(options) = options {
            if !(options.server_url.starts_with("http://")
                || options.server_url.starts_with("https://"))
                || options.timeout_ms == 0
            {
                return Err(AmplitudeError::InvalidInput(
                    "server_url must use HTTP(S) and timeout_ms must be positive".into(),
                ));
            }
            state.server_url = Some(options.server_url.clone());
            state.timeout_ms = options.timeout_ms;
            state.options = Some(options);
        }
        state.api_key = Some(api_key.into());
        state.batch = false;
        *self.inner.network.write() = Some(network);
        Ok(())
    }
    pub fn set_server_url(&self, url: &str) -> &Self {
        if url.starts_with("http://") || url.starts_with("https://") {
            self.inner.state.lock().server_url = Some(url.into());
        }
        self
    }
    pub fn server_url(&self) -> String {
        let state = self.inner.state.lock();
        state.server_url.clone().unwrap_or_else(|| {
            if state.batch {
                AMPLITUDE_BATCH_API_URL
            } else {
                AMPLITUDE_API_URL
            }
            .into()
        })
    }
    pub fn set_options(&self, options: Option<AmplitudeOptions>) -> &Self {
        self.inner.state.lock().options = options;
        self
    }
    pub fn use_batch_mode(&self, batch: bool) -> &Self {
        self.inner.state.lock().batch = batch;
        self
    }
    pub fn set_log_mode(&self, mode: AmplitudeLogMode) -> &Self {
        self.inner.state.lock().logger.mode = mode;
        self
    }
    pub fn set_logger(&self, logger: AmplitudeLogger) -> &Self {
        self.inner.state.lock().logger = logger;
        self
    }
    pub fn set_event_upload_threshold(&self, threshold: usize) -> &Self {
        self.inner.state.lock().threshold = threshold;
        self
    }
    pub fn set_event_upload_period_millis(&self, period: u64) -> &Self {
        self.inner.state.lock().period_ms = period;
        self
    }
    pub fn set_callbacks(&self, callback: Option<AmplitudeCallback>) -> &Self {
        self.inner.state.lock().callback = callback;
        self
    }
    pub fn set_plan(&self, plan: Option<AmplitudePlan>) -> &Self {
        self.inner.state.lock().plan = plan;
        self
    }
    pub fn set_ingestion_metadata(&self, metadata: Option<AmplitudeIngestionMetadata>) -> &Self {
        self.inner.state.lock().ingestion_metadata = metadata;
        self
    }
    pub fn set_proxy(&self, proxy: Option<ProxyConfig>) -> &Self {
        self.inner.state.lock().proxy = proxy;
        self
    }
    pub fn set_flush_timeout(&self, timeout: u64) -> &Self {
        self.inner.state.lock().flush_timeout_ms = timeout;
        self
    }
    pub fn set_send_thread_pool(&self, pool: AmplitudeThreadPool) -> &Self {
        self.inner.state.lock().send_pool = pool;
        self
    }
    pub fn set_retry_thread_pool(&self, pool: AmplitudeThreadPool) -> &Self {
        self.inner.state.lock().retry_pool = pool;
        self
    }
    pub fn set_record_throttled_id(&self, record: bool) -> &Self {
        self.inner.state.lock().record_throttled = record;
        self
    }
    pub fn add_event_middleware(&self, middleware: Arc<dyn AmplitudeMiddleware>) -> &Self {
        self.inner.state.lock().middleware.push(middleware);
        self
    }
    pub fn should_wait(&self, event: &AmplitudeEvent) -> bool {
        let state = self.inner.state.lock();
        if state.queue.len() > state.threshold
            || self.inner.retry_count.load(Ordering::SeqCst) >= AMPLITUDE_MAX_CACHED_EVENTS
        {
            return true;
        }
        state.record_throttled
            && [(true, &event.user_id), (false, &event.device_id)]
                .iter()
                .any(|(user, id)| {
                    id.as_ref().is_some_and(|id| {
                        self.inner
                            .throttled
                            .lock()
                            .contains_key(&(*user, id.clone()))
                    })
                })
    }
    async fn prepare(
        &self,
        mut event: AmplitudeEvent,
        extra: Value,
        callback: Option<AmplitudeCallback>,
    ) -> Result<Option<AmplitudeEvent>, AmplitudeError> {
        event.timestamp = Some(
            event
                .timestamp
                .or(event.time)
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
        );
        let middleware = {
            let state = self.inner.state.lock();
            if state.closed {
                return Err(AmplitudeError::Shutdown);
            }
            if state.api_key.is_none() {
                return Err(AmplitudeError::NotInitialized);
            }
            if event.plan.is_none() {
                event.plan = state.plan.clone();
            }
            if event.ingestion_metadata.is_none() {
                event.ingestion_metadata = state.ingestion_metadata.clone();
            }
            state.middleware.clone()
        };
        // Preserve callbacks across language middleware serialization.
        let callback = callback.or_else(|| event.callback.take());
        let result = AmplitudeMiddlewareNext {
            middleware: Arc::new(middleware),
            position: 0,
        }
        .run(AmplitudeMiddlewarePayload { event, extra })
        .await?;
        let Some(mut payload) = result else {
            return Ok(None);
        };
        if payload.event.user_id.is_none() && payload.event.device_id.is_none() {
            return Err(AmplitudeError::InvalidInput(
                "event_type and either user_id or device_id are required".into(),
            ));
        }
        payload.event.callback = callback;
        // Java truncates its mutable JSON properties during serialization. Retain
        // those values on the accepted event so callbacks observe the sent data.
        for object in [
            &mut payload.event.event_properties,
            &mut payload.event.user_properties,
            &mut payload.event.groups,
            &mut payload.event.group_properties,
        ]
        .into_iter()
        .flatten()
        {
            let value = truncate_properties(&Value::Object(object.clone()))?;
            *object = value.as_object().cloned().unwrap_or_default();
        }
        payload.event.to_json_object()?;
        Ok(Some(payload.event))
    }
    pub async fn log_event(&self, event: AmplitudeEvent) -> Result<(), AmplitudeError> {
        self.log_event_with(event, None, Value::Null).await
    }
    pub async fn log_event_with(
        &self,
        event: AmplitudeEvent,
        callback: Option<AmplitudeCallback>,
        extra: Value,
    ) -> Result<(), AmplitudeError> {
        let Some(event) = self.prepare(event, extra, callback).await? else {
            return Ok(());
        };
        let (events, schedule) = {
            let mut state = self.inner.state.lock();
            if state.closed {
                return Err(AmplitudeError::Shutdown);
            }
            state.queue.push(event);
            if state.queue.len() >= state.threshold {
                // Register sends while holding the queue lock so shutdown cannot miss them.
                self.inner.active.fetch_add(1, Ordering::SeqCst);
                (Some(std::mem::take(&mut state.queue)), None)
            } else if !state.scheduled {
                state.scheduled = true;
                (
                    None,
                    Some((state.period_ms, state.send_pool.handle.clone())),
                )
            } else {
                (None, None)
            }
        };
        if let Some(events) = events {
            self.inner.spawn_send(events);
        }
        if let Some((period, handle)) = schedule {
            let weak = Arc::downgrade(&self.inner);
            let mut stop = self.inner.stop.subscribe();
            handle.spawn(async move {
                if *stop.borrow() { return; }
                tokio::select! { _ = tokio::time::sleep(Duration::from_millis(period)) => {}, _ = stop.changed() => { return; } }
                if let Some(inner) = weak.upgrade() {
                    let events = { let mut state = inner.state.lock(); state.scheduled = false;
                        if state.closed || state.queue.is_empty() { None } else { inner.active.fetch_add(1, Ordering::SeqCst); Some(std::mem::take(&mut state.queue)) }
                    };
                    if let Some(events) = events { inner.spawn_send(events); }
                }
            });
        }
        Ok(())
    }
    /// Java-compatible fire-and-forget flush. Use flush_events_and_wait for a barrier.
    pub fn flush_events(&self) -> Result<(), AmplitudeError> {
        let events = {
            let mut state = self.inner.state.lock();
            if state.closed {
                return Err(AmplitudeError::Shutdown);
            }
            if state.queue.is_empty() {
                return Ok(());
            }
            self.inner.active.fetch_add(1, Ordering::SeqCst);
            std::mem::take(&mut state.queue)
        };
        self.inner.spawn_send(events);
        Ok(())
    }
    pub async fn flush_events_and_wait(&self) -> Result<(), AmplitudeError> {
        self.flush_events()?;
        self.inner.await_idle().await;
        Ok(())
    }
    /// Compatibility with the initial demo: send immediately and await the final callback.
    pub async fn track(&self, event: AmplitudeEvent) -> Result<(), AmplitudeError> {
        let (sender, receiver) = oneshot::channel();
        let sender = Mutex::new(Some(sender));
        let existing = event.callback.clone();
        let callback: AmplitudeCallback = Arc::new(move |event, status, message| {
            if let Some(callback) = &existing {
                callback(event, status, message);
            }
            if let Some(sender) = sender.lock().take() {
                let result = if (200..300).contains(&status) {
                    Ok(())
                } else if message.contains("Invalid API key") {
                    Err(AmplitudeError::InvalidApiKey)
                } else {
                    Err(AmplitudeError::Api {
                        status,
                        body: message.into(),
                    })
                };
                let _ = sender.send(result);
            }
        });
        let Some(event) = self.prepare(event, Value::Null, Some(callback)).await? else {
            return Ok(());
        };
        {
            let state = self.inner.state.lock();
            if state.closed {
                return Err(AmplitudeError::Shutdown);
            }
            self.inner.active.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.spawn_send(vec![event]);
        receiver
            .await
            .map_err(|_| AmplitudeError::Network("Amplitude worker stopped".into()))?
    }
    pub async fn shutdown(&self) -> Result<(), AmplitudeError> {
        let pending = {
            let mut state = self.inner.state.lock();
            state.closed = true;
            state.scheduled = false;
            std::mem::take(&mut state.queue)
        };
        self.inner.stop.send_replace(true);
        self.inner
            .callbacks(&pending, 0, "Client shutdown. Events not retry.");
        self.inner.await_idle().await;
        {
            let mut state = self.inner.state.lock();
            state.callback = None;
            state.middleware.clear();
            state.logger = AmplitudeLogger::default();
        }
        if let Some(name) = &self.inner.name {
            if let Some(registry) = INSTANCES.get() {
                let mut registry = registry.lock();
                if registry
                    .get(name)
                    .is_some_and(|current| self.is_same_instance(current))
                {
                    registry.remove(name);
                }
            }
        }
        Ok(())
    }
}

struct TaskGuard {
    inner: Arc<Inner>,
    retry_count: usize,
    ids: HashSet<(bool, String)>,
}
impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.inner
            .retry_count
            .fetch_sub(self.retry_count, Ordering::SeqCst);
        let mut throttled = self.inner.throttled.lock();
        for key in &self.ids {
            if let Some(count) = throttled.get_mut(key) {
                *count -= 1;
                if *count == 0 {
                    throttled.remove(key);
                }
            }
        }
        drop(throttled);
        self.inner.active.fetch_sub(1, Ordering::SeqCst);
        self.inner.idle.notify_waiters();
    }
}
impl Inner {
    fn log(&self, mode: AmplitudeLogMode, tag: &str, message: &str) {
        let logger = self.state.lock().logger.clone();
        if mode >= logger.mode && mode != AmplitudeLogMode::Off {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (logger.callback)(mode, tag, message)
            }));
        }
    }
    async fn await_idle(&self) {
        loop {
            let notified = self.idle.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.active.load(Ordering::SeqCst) == 0 {
                return;
            }
            notified.await;
        }
    }
    fn callbacks(&self, events: &[AmplitudeEvent], code: u16, message: &str) {
        let (callback, logger) = {
            let state = self.state.lock();
            (state.callback.clone(), state.logger.clone())
        };
        for event in events {
            for callback in [callback.as_ref(), event.callback.as_ref()]
                .into_iter()
                .flatten()
            {
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(event, code, message)
                }))
                .is_err()
                    && logger.mode <= AmplitudeLogMode::Error
                {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (logger.callback)(
                            AmplitudeLogMode::Error,
                            "Amplitude",
                            "Event callback panicked",
                        )
                    }));
                }
            }
        }
    }
    async fn request(
        &self,
        events: &[AmplitudeEvent],
        initial: bool,
    ) -> Result<AmplitudeResponse, AmplitudeError> {
        let (args, flush_timeout) = {
            let state = self.state.lock();
            let mut body = json!({"api_key": state.api_key, "events": events.iter().map(AmplitudeEvent::to_json_object).collect::<Result<Vec<_>,_>>()?});
            let mut headers = HashMap::from([
                ("Content-Type".into(), "application/json".into()),
                ("Accept".into(), "application/json".into()),
            ]);
            if let Some(options) = &state.options {
                body["options"] = options.to_json_object();
                headers.extend(options.headers.clone());
            }
            let url = state.server_url.clone().unwrap_or_else(|| {
                if state.batch {
                    AMPLITUDE_BATCH_API_URL
                } else {
                    AMPLITUDE_API_URL
                }
                .into()
            });
            (
                RequestArgs {
                    url,
                    body: Some(
                        serde_json::to_vec(&body)
                            .map_err(|error| AmplitudeError::InvalidInput(error.to_string()))?,
                    ),
                    headers: Some(headers),
                    timeout_ms: state.timeout_ms,
                    proxy_config: state.proxy.clone(),
                    disable_system_proxy: true,
                    disable_file_streaming: Some(true),
                    ..RequestArgs::new()
                },
                state.flush_timeout_ms,
            )
        };
        let network = self
            .network
            .read()
            .clone()
            .ok_or(AmplitudeError::NotInitialized)?;
        let future = network.send(&HttpMethod::POST, &args);
        let mut response = if initial && flush_timeout > 0 {
            tokio::time::timeout(Duration::from_millis(flush_timeout), future)
                .await
                .map_err(|_| AmplitudeError::Network("Flush execution timeout".into()))?
        } else {
            future.await
        };
        if response.error.is_some() {
            return Ok(AmplitudeResponse::from_http_status(408, ""));
        }
        let status = response.status_code.unwrap_or(408);
        let body = response
            .data
            .as_mut()
            .and_then(|data| data.read_to_string().ok())
            .unwrap_or_default();
        let result = AmplitudeResponse::parse(status, &body);
        self.log(
            AmplitudeLogMode::Debug,
            if initial { "SEND" } else { "RETRY" },
            &format!("Events count {}. HTTP {status}", events.len()),
        );
        result
    }
    fn spawn_send(self: &Arc<Self>, events: Vec<AmplitudeEvent>) {
        let inner = self.clone();
        let pool = self.state.lock().send_pool.clone();
        let guard = TaskGuard {
            inner: inner.clone(),
            retry_count: 0,
            ids: HashSet::new(),
        };
        pool.handle.spawn(async move {
            let _guard = guard;
            let _permit = pool.semaphore.acquire_owned().await;
            if *inner.stop.borrow() {
                inner.callbacks(&events, 0, "Client shutdown. Events not retry.");
                return;
            }
            match inner.request(&events, true).await {
                Err(error) => {
                    inner.log(
                        AmplitudeLogMode::Error,
                        "Flush Thread Error",
                        &error.to_string(),
                    );
                    inner.callbacks(
                        &events,
                        0,
                        &format!("Error sending events due to the exception: {error}"),
                    );
                }
                Ok(response) if response.status == AmplitudeStatus::Success => {
                    inner.callbacks(&events, response.code, "Event sent success.")
                }
                Ok(response) if !response.status.should_retry() => {
                    inner.callbacks(&events, response.code, "Unknown response status.")
                }
                Ok(response) => inner.enqueue_retries(events, response),
            }
        });
    }
    fn enqueue_retries(self: &Arc<Self>, events: Vec<AmplitudeEvent>, response: AmplitudeResponse) {
        let mut groups: HashMap<(String, String), Vec<AmplitudeEvent>> = HashMap::new();
        let invalid = response.invalid_event_indices();
        let missing_field = response
            .body
            .get("missing_field")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty());
        let count = events.len();
        for (index, event) in events.into_iter().enumerate() {
            let drop = (response.status == AmplitudeStatus::Invalid
                && (missing_field || count == 1 || invalid.contains(&index)))
                || (response.status == AmplitudeStatus::Ratelimit
                    && response.exceeds_quota(&event));
            if drop {
                self.callbacks(&[event], response.code, &response.error);
            } else {
                groups
                    .entry((
                        event.user_id.clone().unwrap_or_default(),
                        event.device_id.clone().unwrap_or_default(),
                    ))
                    .or_default()
                    .push(event);
            }
        }
        for (_, events) in groups {
            if events.is_empty() {
                continue;
            }
            let reserved = events.len();
            let admitted = self
                .retry_count
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    (current < AMPLITUDE_MAX_CACHED_EVENTS).then_some(current + reserved)
                })
                .is_ok();
            if !admitted {
                self.callbacks(
                    &events,
                    response.code,
                    "Retry buffer is full, events dropped.",
                );
                continue;
            }
            let mut ids = HashSet::new();
            if response.status == AmplitudeStatus::Ratelimit && self.state.lock().record_throttled {
                for event in &events {
                    for (user, key, id) in [
                        (true, "throttled_users", &event.user_id),
                        (false, "throttled_devices", &event.device_id),
                    ] {
                        if let Some(id) = id {
                            if response
                                .body
                                .get(key)
                                .and_then(Value::as_object)
                                .is_some_and(|map| map.contains_key(id))
                            {
                                ids.insert((user, id.clone()));
                            }
                        }
                    }
                }
                let mut throttled = self.throttled.lock();
                for id in &ids {
                    *throttled.entry(id.clone()).or_default() += 1;
                }
            }
            self.active.fetch_add(1, Ordering::SeqCst);
            let inner = self.clone();
            let pool = self.state.lock().retry_pool.clone();
            let guard = TaskGuard {
                inner: inner.clone(),
                retry_count: reserved,
                ids,
            };
            pool.handle.spawn(async move {
                let _guard = guard;
                let mut stop = inner.stop.subscribe();
                if *stop.borrow() { inner.callbacks(&events, 0, "Client shutdown. Events not retry."); return; }
                let _permit = tokio::select! { permit = pool.semaphore.acquire_owned() => permit, _ = stop.changed() => { inner.callbacks(&events, 0, "Client shutdown. Events not retry."); return; } };
                inner.retry_loop(events, stop).await;
            });
        }
    }
    async fn retry_loop(&self, mut events: Vec<AmplitudeEvent>, mut stop: watch::Receiver<bool>) {
        for (attempt, delay) in AMPLITUDE_RETRY_TIMEOUTS.iter().enumerate() {
            if events.is_empty() {
                return;
            }
            if *stop.borrow() {
                self.callbacks(&events, 0, "Client shutdown. Events not retry.");
                return;
            }
            tokio::select! { _ = tokio::time::sleep(Duration::from_millis(*delay)) => {}, _ = stop.changed() => { self.callbacks(&events, 0, "Client shutdown. Events not retry."); return; } }
            let response = match self.request(&events, false).await {
                Ok(response) => response,
                Err(error) => {
                    self.callbacks(&events, 0, &format!("Retry threads Exception: {error}"));
                    return;
                }
            };
            match response.status {
                AmplitudeStatus::Success => {
                    self.callbacks(&events, response.code, "Events sent success.");
                    return;
                }
                AmplitudeStatus::Ratelimit if response.exceeds_quota(&events[0]) => {
                    self.callbacks(&events, response.code, &response.error);
                    return;
                }
                AmplitudeStatus::Invalid if events.len() == 1 => {
                    self.callbacks(&events, response.code, &response.error);
                    return;
                }
                AmplitudeStatus::Unknown | AmplitudeStatus::Skipped => {
                    self.callbacks(&events, response.code, "Unknown response status.");
                    return;
                }
                _ => {}
            }
            if attempt == AMPLITUDE_RETRY_TIMEOUTS.len() - 1 {
                self.callbacks(&events, response.code, "Event retries exhausted.");
                return;
            }
            if response.status == AmplitudeStatus::Invalid {
                let invalid = response.invalid_event_indices();
                let (dropped, keep): (Vec<_>, Vec<_>) = events
                    .into_iter()
                    .enumerate()
                    .partition(|(index, _)| invalid.contains(index));
                self.callbacks(
                    &dropped
                        .into_iter()
                        .map(|(_, event)| event)
                        .collect::<Vec<_>>(),
                    response.code,
                    "Invalid events.",
                );
                events = keep.into_iter().map(|(_, event)| event).collect();
            } else if response.status == AmplitudeStatus::PayloadTooLarge {
                let dropped = events.split_off(events.len() / 2);
                self.callbacks(&dropped, response.code, "Event dropped for retry");
            }
        }
    }
}
