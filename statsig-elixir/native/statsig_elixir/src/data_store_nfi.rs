use async_trait::async_trait;
use parking_lot::Mutex;
use rustler::{
    env::OwnedEnv, types::atom::Atom, types::local_pid::LocalPid, Encoder, Env, Error, ResourceArc,
    Term,
};
use statsig_rust::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    log_d, StatsigErr,
};
use std::{cell::RefCell, mem};
use tokio::sync::oneshot;

const TAG: &str = "[DataStore NFI] ";
// Track the active BEAM Env on this scheduler thread so async work can reuse it.
thread_local! {
    static MANAGED_ENVS: RefCell<Vec<Env<'static>>> = const { RefCell::new(Vec::new()) };
}

pub struct ManagedEnvGuard {
    active: bool,
}

impl ManagedEnvGuard {
    pub fn new(env: Env<'_>) -> Self {
        // SAFETY: The Env outlives the guard and is only accessed on this thread.
        let env_static = unsafe { mem::transmute::<Env<'_>, Env<'static>>(env) };
        MANAGED_ENVS.with(|stack| stack.borrow_mut().push(env_static));
        ManagedEnvGuard { active: true }
    }
}

impl Drop for ManagedEnvGuard {
    fn drop(&mut self) {
        if self.active {
            MANAGED_ENVS.with(|stack| {
                stack.borrow_mut().pop();
            });
            self.active = false;
        }
    }
}

fn current_managed_env() -> Option<Env<'static>> {
    MANAGED_ENVS.with(|stack| stack.borrow().last().copied())
}

mod atoms {
    rustler::atoms! {
        data_store_request = "statsig_data_store_request",
        initialize,
        shutdown,
        get,
        set,
        support_polling_updates_for,
        no_payload
    }
}

#[derive(rustler::NifStruct)]
#[module = "Statsig.DataStore.Reference"]
pub struct StatsigDataStoreReference {
    pub pid: LocalPid,
}

#[derive(rustler::NifStruct)]
#[module = "Statsig.DataStore.Response"]
pub struct StatsigDataStoreResponse {
    pub result: Option<String>,
    pub time: Option<u64>,
}

pub struct ElixirDataStore {
    pid: LocalPid,
}

impl ElixirDataStore {
    pub fn new(pid: LocalPid) -> Self {
        Self { pid }
    }

    async fn request_unit(&self, request: RequestKind) -> Result<(), StatsigErr> {
        match self.send_request(request).await? {
            ResponsePayload::Unit => Ok(()),
            _ => Err(StatsigErr::DataStoreFailure(
                "Unexpected reply from Elixir data store".to_string(),
            )),
        }
    }

    async fn request_bool(&self, request: RequestKind) -> Result<bool, StatsigErr> {
        match self.send_request(request).await? {
            ResponsePayload::Bool(value) => Ok(value),
            _ => Err(StatsigErr::DataStoreFailure(
                "Unexpected reply from Elixir data store".to_string(),
            )),
        }
    }

    async fn request_data(&self, request: RequestKind) -> Result<DataStoreResponse, StatsigErr> {
        match self.send_request(request).await? {
            ResponsePayload::Data(value) => Ok(value),
            _ => Err(StatsigErr::DataStoreFailure(
                "Unexpected reply from Elixir data store".to_string(),
            )),
        }
    }

    async fn send_request(&self, request: RequestKind) -> Result<ResponsePayload, StatsigErr> {
        log_d!(TAG, "Sending data store request to Elixir");
        let (sender, receiver) = oneshot::channel();
        let resource = ResourceArc::new(DataStoreRequestResource::new(
            request.response_kind(),
            sender,
        ));

        if let Some(env) = current_managed_env() {
            env.send(
                &self.pid,
                (
                    atoms::data_store_request(),
                    resource.clone(),
                    request.atom(),
                    request.encode_payload(env),
                ),
            )
            .map_err(|_| {
                StatsigErr::DataStoreFailure("Failed to message Elixir data store".to_string())
            })?;
        } else {
            let mut env = OwnedEnv::new();
            env.send_and_clear(&self.pid, |env| {
                (
                    atoms::data_store_request(),
                    resource.clone(),
                    request.atom(),
                    request.encode_payload(env),
                )
            })
            .map_err(|_| {
                StatsigErr::DataStoreFailure("Failed to message Elixir data store".to_string())
            })?;
        }

        receiver.await.map_err(|_| {
            StatsigErr::DataStoreFailure("Elixir data store did not reply to request".to_string())
        })?
    }
}

#[async_trait]
impl DataStoreTrait for ElixirDataStore {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        self.request_unit(RequestKind::Initialize).await
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        self.request_unit(RequestKind::Shutdown).await
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        self.request_data(RequestKind::Get(key.to_string())).await
    }

    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
        log_d!(TAG, "Setting key in data store");
        self.request_unit(RequestKind::Set {
            key: key.to_string(),
            value: value.to_string(),
            time,
        })
        .await
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        self.request_bool(RequestKind::SupportPolling(path.to_string()))
            .await
            .unwrap_or(false)
    }
}

enum RequestKind {
    Initialize,
    Shutdown,
    Get(String),
    Set {
        key: String,
        value: String,
        time: Option<u64>,
    },
    SupportPolling(String),
}

impl RequestKind {
    fn atom(&self) -> Atom {
        match self {
            RequestKind::Initialize => atoms::initialize(),
            RequestKind::Shutdown => atoms::shutdown(),
            RequestKind::Get(_) => atoms::get(),
            RequestKind::Set { .. } => atoms::set(),
            RequestKind::SupportPolling(_) => atoms::support_polling_updates_for(),
        }
    }

    fn encode_payload<'a>(&self, env: Env<'a>) -> Term<'a> {
        match self {
            RequestKind::Initialize | RequestKind::Shutdown => atoms::no_payload().encode(env),
            RequestKind::Get(key) => key.encode(env),
            RequestKind::Set { key, value, time } => {
                (key.clone(), value.clone(), *time).encode(env)
            }
            RequestKind::SupportPolling(path) => path.encode(env),
        }
    }

    fn response_kind(&self) -> ResponseKind {
        match self {
            RequestKind::Initialize | RequestKind::Shutdown | RequestKind::Set { .. } => {
                ResponseKind::Unit
            }
            RequestKind::Get(_) => ResponseKind::Data,
            RequestKind::SupportPolling(_) => ResponseKind::Bool,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ResponseKind {
    Unit,
    Data,
    Bool,
}

pub enum ResponsePayload {
    Unit,
    Data(DataStoreResponse),
    Bool(bool),
}

pub struct DataStoreRequestResource {
    response_kind: ResponseKind,
    sender: Mutex<Option<oneshot::Sender<Result<ResponsePayload, StatsigErr>>>>,
}

impl DataStoreRequestResource {
    pub fn new(
        response_kind: ResponseKind,
        sender: oneshot::Sender<Result<ResponsePayload, StatsigErr>>,
    ) -> Self {
        Self {
            response_kind,
            sender: Mutex::new(Some(sender)),
        }
    }

    fn fulfill_ok(&self, payload: Term) -> Result<(), Error> {
        let decoded: Result<ResponsePayload, Error> = match self.response_kind {
            ResponseKind::Unit => Ok(ResponsePayload::Unit),
            ResponseKind::Bool => {
                payload
                    .decode::<bool>()
                    .map(ResponsePayload::Bool)
                    .map_err(|err| {
                        self.send_err(format!("Failed to decode boolean payload: {err:?}"));
                        err
                    })
            }
            ResponseKind::Data => payload
                .decode::<StatsigDataStoreResponse>()
                .map(|resp| ResponsePayload::Data(resp.into()))
                .map_err(|err| {
                    self.send_err(format!("Failed to decode data payload: {err:?}"));
                    err
                }),
        };

        match decoded {
            Ok(value) => {
                self.send_ok(value);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn fulfill_err(&self, reason: String) -> Result<(), Error> {
        self.send_err(reason);
        Ok(())
    }

    fn send_ok(&self, payload: ResponsePayload) {
        if let Some(sender) = self.sender.lock().take() {
            let _ = sender.send(Ok(payload));
        }
    }

    fn send_err(&self, reason: String) {
        if let Some(sender) = self.sender.lock().take() {
            let _ = sender.send(Err(StatsigErr::DataStoreFailure(reason)));
        }
    }
}

impl From<StatsigDataStoreResponse> for DataStoreResponse {
    fn from(value: StatsigDataStoreResponse) -> Self {
        DataStoreResponse {
            result: value.result,
            time: value.time,
        }
    }
}

#[rustler::nif]
pub fn data_store_reply(
    request: ResourceArc<DataStoreRequestResource>,
    payload: Term,
) -> Result<(), Error> {
    request.fulfill_ok(payload)
}

#[rustler::nif]
pub fn data_store_reply_error(
    request: ResourceArc<DataStoreRequestResource>,
    reason: String,
) -> Result<(), Error> {
    request.fulfill_err(reason)
}
