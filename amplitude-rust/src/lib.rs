//! An independent, asynchronous Amplitude client. Requires a running Tokio runtime.
//! Call `shutdown().await` before dropping the runtime to drain accepted events.

#![doc = include_str!("../README.md")]

use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    sync::{Arc, Mutex, OnceLock, Weak},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use tokio::task::JoinHandle;

pub const API_URL: &str = "https://api2.amplitude.com/2/httpapi";
pub const BATCH_API_URL: &str = "https://api2.amplitude.com/batch";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfiguration(String),
    InvalidEvent(String),
    Http(u16),
    Network,
    Timeout,
    Shutdown,
    WorkerStopped,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Do not include request bodies, credentials, or HTTP response bodies.
        write!(f, "Amplitude error: {self:?}")
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<u64>,
    pub insert_id: String,
    pub event_properties: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingestion_metadata: Option<Value>,
    /// Additional Amplitude fields, such as `user_properties` or `session_id`.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

impl Event {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            user_id: None,
            device_id: None,
            time: None,
            insert_id: uuid::Uuid::new_v4().to_string(),
            event_properties: Map::new(),
            plan: None,
            ingestion_metadata: None,
            fields: Map::new(),
        }
    }

    fn validate(&self) -> Result<(), Error> {
        if self.event_type.trim().is_empty() {
            return Err(Error::InvalidEvent("event_type is required".into()));
        }
        if self.user_id.as_ref().is_none_or(|id| id.is_empty())
            && self.device_id.as_ref().is_none_or(|id| id.is_empty())
        {
            return Err(Error::InvalidEvent(
                "user_id or device_id is required".into(),
            ));
        }
        if self.insert_id.is_empty() {
            return Err(Error::InvalidEvent("insert_id is required".into()));
        }
        for key in self.fields.keys() {
            if matches!(
                key.as_str(),
                "event_type"
                    | "user_id"
                    | "device_id"
                    | "time"
                    | "insert_id"
                    | "event_properties"
                    | "plan"
                    | "ingestion_metadata"
            ) {
                return Err(Error::InvalidEvent(
                    "additional field shadows an event field".into(),
                ));
            }
        }
        Ok(())
    }
}

pub type Callback = Arc<dyn Fn(&Event, &Result<(), Error>) + Send + Sync>;
/// Return false to discard an event. Middleware runs in registration order.
pub type Middleware = Arc<dyn Fn(&mut Event, &Value) -> bool + Send + Sync>;

#[derive(Clone)]
pub struct Options {
    pub server_url: Option<String>,
    pub batch_mode: bool,
    pub upload_threshold: usize,
    pub upload_period: Duration,
    pub max_batch_size: usize,
    pub request_timeout: Duration,
    pub flush_timeout: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub record_throttled_ids: bool,
    /// None uses a direct connection, including when environment proxies are set.
    pub proxy: Option<String>,
    pub request_options: Option<Value>,
    pub plan: Option<Value>,
    pub ingestion_metadata: Option<Value>,
    pub callback: Option<Callback>,
    pub middleware: Vec<Middleware>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            server_url: None,
            batch_mode: false,
            upload_threshold: 100,
            upload_period: Duration::from_secs(10),
            max_batch_size: 1000,
            request_timeout: Duration::from_secs(10),
            flush_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            record_throttled_ids: false,
            proxy: None,
            request_options: None,
            plan: None,
            ingestion_metadata: None,
            callback: None,
            middleware: Vec::new(),
        }
    }
}

impl Options {
    fn client(&self) -> Result<Client, Error> {
        let url = self.server_url.as_deref().unwrap_or(API_URL);
        let valid_url = Url::parse(url).ok().is_some_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        });
        if !valid_url
            || self.upload_threshold == 0
            || self.max_batch_size == 0
            || self.max_batch_size > 1000
            || self.upload_period.is_zero()
            || self.request_timeout.is_zero()
            || self.flush_timeout.is_zero()
            || self.max_retries > 20
        {
            return Err(Error::InvalidConfiguration(
                "invalid URL, limits, or timeout".into(),
            ));
        }
        if self
            .request_options
            .as_ref()
            .is_some_and(|value| !value.is_object())
        {
            return Err(Error::InvalidConfiguration(
                "request_options must be an object".into(),
            ));
        }
        let mut builder = Client::builder().no_proxy().timeout(self.request_timeout);
        if let Some(proxy) = &self.proxy {
            builder = builder.proxy(
                reqwest::Proxy::all(proxy)
                    .map_err(|_| Error::InvalidConfiguration("invalid proxy".into()))?,
            );
        }
        builder
            .build()
            .map_err(|_| Error::InvalidConfiguration("HTTP client setup failed".into()))
    }
}

struct PendingEvent {
    event: Event,
    callback: Option<Callback>,
}

type Reply = oneshot::Sender<Result<(), Error>>;
type Update = Box<dyn FnOnce(&mut Options) + Send>;

enum Command {
    Log(Box<PendingEvent>, Value, Reply),
    Send(Vec<Event>, Reply),
    Flush(Reply),
    Configure(Update, Reply),
    Shutdown(Reply),
}

type Registry = Mutex<HashMap<String, Weak<Amplitude>>>;
static INSTANCES: OnceLock<Registry> = OnceLock::new();

struct Pressure {
    pending: usize,
    threshold: usize,
    retrying: usize,
    throttled_users: HashSet<String>,
    throttled_devices: HashSet<String>,
}

pub struct Amplitude {
    sender: AsyncMutex<Option<mpsc::Sender<Command>>>,
    worker: AsyncMutex<Option<JoinHandle<()>>>,
    shutdown_result: AsyncMutex<Option<Result<(), Error>>>,
    pressure: Arc<Mutex<Pressure>>,
}

impl Amplitude {
    pub fn new(api_key: impl Into<String>, options: Options) -> Result<Arc<Self>, Error> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(Error::InvalidConfiguration("API key is required".into()));
        }
        let client = options.client()?;
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| Error::InvalidConfiguration("a Tokio runtime is required".into()))?;
        let pressure = Arc::new(Mutex::new(Pressure {
            pending: 0,
            threshold: options.upload_threshold,
            retrying: 0,
            throttled_users: HashSet::new(),
            throttled_devices: HashSet::new(),
        }));
        let (sender, receiver) = mpsc::channel(1024);
        let worker_pressure = Arc::clone(&pressure);
        let worker = runtime.spawn(run(api_key, options, client, receiver, worker_pressure));
        Ok(Arc::new(Self {
            sender: AsyncMutex::new(Some(sender)),
            worker: AsyncMutex::new(Some(worker)),
            shutdown_result: AsyncMutex::new(None),
            pressure,
        }))
    }

    /// The first call for a name determines its API key and initial options.
    /// Use an empty name for the default instance. Shut-down instances are replaced.
    pub fn get_instance(
        name: impl Into<String>,
        api_key: impl Into<String>,
        options: Options,
    ) -> Result<Arc<Self>, Error> {
        let mut registry = INSTANCES
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap();
        registry.retain(|_, instance| instance.strong_count() > 0);
        let name = name.into();
        if let Some(instance) = registry.get(&name).and_then(Weak::upgrade) {
            return Ok(instance);
        }
        let instance = Self::new(api_key, options)?;
        registry.insert(name, Arc::downgrade(&instance));
        Ok(instance)
    }

    async fn execute(&self, make_command: impl FnOnce(Reply) -> Command) -> Result<(), Error> {
        let (reply, response) = oneshot::channel();
        {
            // Serialize enqueueing with shutdown, including sends waiting for capacity.
            let sender = self.sender.lock().await;
            sender
                .as_ref()
                .ok_or(Error::Shutdown)?
                .send(make_command(reply))
                .await
                .map_err(|_| Error::WorkerStopped)?;
        }
        response.await.map_err(|_| Error::WorkerStopped)?
    }

    pub async fn log_event(&self, event: Event) -> Result<(), Error> {
        self.log_event_with(event, None, Value::Null).await
    }

    /// Acknowledges enqueueing; delivery is reported through callbacks or `flush_events`.
    pub async fn log_event_with(
        &self,
        event: Event,
        callback: Option<Callback>,
        extra: Value,
    ) -> Result<(), Error> {
        self.execute(|reply| Command::Log(Box::new(PendingEvent { event, callback }), extra, reply))
            .await
    }

    /// Wait for delivery of exactly these events. Used by adapters that own their queue.
    /// Keep insert IDs stable when retrying this operation.
    pub async fn send_events(&self, events: Vec<Event>) -> Result<(), Error> {
        self.execute(|reply| Command::Send(events, reply)).await
    }

    pub async fn flush_events(&self) -> Result<(), Error> {
        self.execute(Command::Flush).await
    }

    /// Atomically validate and replace configuration before subsequent queued commands.
    pub async fn configure(
        &self,
        update: impl FnOnce(&mut Options) + Send + 'static,
    ) -> Result<(), Error> {
        self.execute(|reply| Command::Configure(Box::new(update), reply))
            .await
    }

    pub fn should_wait(&self, event: &Event) -> bool {
        let pressure = self.pressure.lock().unwrap();
        pressure.pending > pressure.threshold
            || pressure.retrying > 16_000
            || event
                .user_id
                .as_ref()
                .is_some_and(|id| pressure.throttled_users.contains(id))
            || event
                .device_id
                .as_ref()
                .is_some_and(|id| pressure.throttled_devices.contains(id))
    }

    /// Stop accepting events, drain earlier commands and buffered events, and join the worker.
    pub async fn shutdown(&self) -> Result<(), Error> {
        let mut completed = self.shutdown_result.lock().await;
        if let Some(result) = &*completed {
            return result.clone();
        }
        let (reply, response) = oneshot::channel();
        let result = {
            let mut sender = self.sender.lock().await;
            match sender.take() {
                Some(sender) => sender
                    .send(Command::Shutdown(reply))
                    .await
                    .map_err(|_| Error::WorkerStopped),
                None => Err(Error::Shutdown),
            }
        };
        let result = match result {
            Ok(()) => response.await.unwrap_or(Err(Error::WorkerStopped)),
            Err(error) => Err(error),
        };
        if let Some(worker) = self.worker.lock().await.take() {
            if worker.await.is_err() {
                *completed = Some(Err(Error::WorkerStopped));
                return Err(Error::WorkerStopped);
            }
        }
        if let Some(registry) = INSTANCES.get() {
            registry
                .lock()
                .unwrap()
                .retain(|_, instance| !std::ptr::eq(instance.as_ptr(), self));
        }
        *completed = Some(result.clone());
        result
    }
}

fn prepare(event: &mut Event, extra: &Value, options: &Options) -> Result<bool, Error> {
    if event.plan.is_none() {
        event.plan.clone_from(&options.plan);
    }
    if event.ingestion_metadata.is_none() {
        event
            .ingestion_metadata
            .clone_from(&options.ingestion_metadata);
    }
    for middleware in &options.middleware {
        if !middleware(event, extra) {
            return Ok(false);
        }
    }
    event.validate()?;
    Ok(true)
}

async fn run(
    api_key: String,
    mut options: Options,
    mut client: Client,
    mut receiver: mpsc::Receiver<Command>,
    pressure: Arc<Mutex<Pressure>>,
) {
    let mut pending = Vec::new();
    let mut deadline = tokio::time::Instant::now() + options.upload_period;
    loop {
        let command = tokio::select! {
            command = receiver.recv() => command,
            _ = tokio::time::sleep_until(deadline), if !pending.is_empty() => {
                if let Err(error) = deliver(&api_key, &options, &client, &pressure, std::mem::take(&mut pending)).await {
                    log::warn!("{error}");
                }
                pressure.lock().unwrap().pending = 0;
                continue;
            }
        };
        let Some(command) = command else {
            let _ = deliver(&api_key, &options, &client, &pressure, pending).await;
            return;
        };
        match command {
            Command::Log(mut item, extra, reply) => {
                match prepare(&mut item.event, &extra, &options) {
                    Ok(true) => {
                        if pending.is_empty() {
                            deadline = tokio::time::Instant::now() + options.upload_period;
                        }
                        pending.push(*item);
                        pressure.lock().unwrap().pending = pending.len();
                        let _ = reply.send(Ok(()));
                        if pending.len() >= options.upload_threshold {
                            if let Err(error) = deliver(
                                &api_key,
                                &options,
                                &client,
                                &pressure,
                                std::mem::take(&mut pending),
                            )
                            .await
                            {
                                log::warn!("{error}");
                            }
                            pressure.lock().unwrap().pending = 0;
                        }
                    }
                    Ok(false) => {
                        let _ = reply.send(Ok(()));
                    }
                    Err(error) => {
                        let _ = reply.send(Err(error));
                    }
                }
            }
            Command::Send(events, reply) => {
                let prepared = events
                    .into_iter()
                    .try_fold(Vec::new(), |mut items, mut event| {
                        if prepare(&mut event, &Value::Null, &options)? {
                            items.push(PendingEvent {
                                event,
                                callback: None,
                            });
                        }
                        Ok::<_, Error>(items)
                    });
                let result = match prepared {
                    Ok(items) => deliver(&api_key, &options, &client, &pressure, items).await,
                    Err(error) => Err(error),
                };
                let _ = reply.send(result);
            }
            Command::Flush(reply) => {
                let result = deliver(
                    &api_key,
                    &options,
                    &client,
                    &pressure,
                    std::mem::take(&mut pending),
                )
                .await;
                pressure.lock().unwrap().pending = 0;
                let _ = reply.send(result);
            }
            Command::Configure(update, reply) => {
                let mut next = options.clone();
                update(&mut next);
                let result = next.client().map(|next_client| {
                    options = next;
                    client = next_client;
                    pressure.lock().unwrap().threshold = options.upload_threshold;
                    deadline = tokio::time::Instant::now() + options.upload_period;
                });
                let _ = reply.send(result);
            }
            Command::Shutdown(reply) => {
                let result = deliver(&api_key, &options, &client, &pressure, pending).await;
                pressure.lock().unwrap().pending = 0;
                let _ = reply.send(result);
                return;
            }
        }
    }
}

fn callbacks(options: &Options, items: &[PendingEvent], result: &Result<(), Error>) {
    for item in items {
        for callback in [options.callback.as_ref(), item.callback.as_ref()]
            .into_iter()
            .flatten()
        {
            // User callbacks must not strand the queue if they panic.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                callback(&item.event, result)
            }));
        }
    }
}

async fn deliver(
    api_key: &str,
    options: &Options,
    client: &Client,
    pressure: &Mutex<Pressure>,
    items: Vec<PendingEvent>,
) -> Result<(), Error> {
    let mut batches = VecDeque::new();
    let mut iter = items.into_iter();
    loop {
        let batch: Vec<_> = iter.by_ref().take(options.max_batch_size).collect();
        if batch.is_empty() {
            break;
        }
        batches.push_back(batch);
    }
    let mut first_error = None;
    while let Some(mut batch) = batches.pop_front() {
        let result = tokio::time::timeout(
            options.flush_timeout,
            upload(api_key, options, client, pressure, &batch),
        )
        .await
        .unwrap_or(Err(Error::Timeout));
        if result == Err(Error::Http(413)) && batch.len() > 1 {
            let second = batch.split_off(batch.len() / 2);
            batches.push_front(second);
            batches.push_front(batch);
            continue;
        }
        {
            let mut pressure = pressure.lock().unwrap();
            pressure.retrying = 0;
            pressure.throttled_users.clear();
            pressure.throttled_devices.clear();
        }
        callbacks(options, &batch, &result);
        if let Err(error) = result {
            first_error.get_or_insert(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

async fn upload(
    api_key: &str,
    options: &Options,
    client: &Client,
    pressure: &Mutex<Pressure>,
    items: &[PendingEvent],
) -> Result<(), Error> {
    let url = options
        .server_url
        .as_deref()
        .unwrap_or(if options.batch_mode {
            BATCH_API_URL
        } else {
            API_URL
        });
    let events: Vec<_> = items.iter().map(|item| &item.event).collect();
    let mut body = json!({ "api_key": api_key, "events": events });
    if let Some(request_options) = &options.request_options {
        body["options"] = request_options.clone();
    }
    for attempt in 0..=options.max_retries {
        let response = client.post(url).json(&body).send().await;
        let (error, retryable, retry_after) = match response {
            Ok(response) => {
                let status = response.status();
                if status == StatusCode::OK {
                    return Ok(());
                }
                let retry_after = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(Duration::from_secs);
                (
                    Error::Http(status.as_u16()),
                    status == StatusCode::TOO_MANY_REQUESTS
                        || status == StatusCode::REQUEST_TIMEOUT
                        || status.is_server_error(),
                    retry_after,
                )
            }
            Err(_) => (Error::Network, true, None),
        };
        if !retryable || attempt == options.max_retries {
            return Err(error);
        }
        {
            let mut pressure = pressure.lock().unwrap();
            pressure.retrying = items.len();
            if error == Error::Http(429) && options.record_throttled_ids {
                for item in items {
                    if let Some(id) = &item.event.user_id {
                        pressure.throttled_users.insert(id.clone());
                    }
                    if let Some(id) = &item.event.device_id {
                        pressure.throttled_devices.insert(id.clone());
                    }
                }
            }
        }
        let backoff = options
            .retry_delay
            .saturating_mul(1 << attempt)
            .min(Duration::from_secs(60));
        tokio::time::sleep(retry_after.unwrap_or(backoff)).await;
    }
    unreachable!()
}
