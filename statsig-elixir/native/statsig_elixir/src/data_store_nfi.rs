use async_trait::async_trait;
use parking_lot::Mutex;
use rustler::{
    env::OwnedEnv,
    types::atom::Atom,
    types::binary::{Binary, OwnedBinary},
    types::local_pid::LocalPid,
    Encoder, Env, Error, ResourceArc, Term,
};
use statsig_rust::{
    data_store_interface::{
        DataStoreBytesResponse, DataStoreResponse, DataStoreTrait, RequestPath,
    },
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
        get_bytes,
        set,
        set_bytes,
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

#[derive(rustler::NifStruct)]
#[module = "Statsig.DataStore.BytesResponse"]
pub struct StatsigDataStoreBytesResponse<'a> {
    pub result: Option<Binary<'a>>,
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

    async fn request_bytes_data(
        &self,
        request: RequestKind,
    ) -> Result<DataStoreBytesResponse, StatsigErr> {
        match self.send_request(request).await? {
            ResponsePayload::BytesData(value) => Ok(value),
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
            request.is_bytes_request(),
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

    async fn get_bytes(&self, key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
        self.request_bytes_data(RequestKind::GetBytes(key.to_string()))
            .await
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

    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        time: Option<u64>,
    ) -> Result<(), StatsigErr> {
        log_d!(TAG, "Setting bytes key in data store");
        self.request_unit(RequestKind::SetBytes {
            key: key.to_string(),
            value: value.to_vec(),
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
    GetBytes(String),
    Set {
        key: String,
        value: String,
        time: Option<u64>,
    },
    SetBytes {
        key: String,
        value: Vec<u8>,
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
            RequestKind::GetBytes(_) => atoms::get_bytes(),
            RequestKind::Set { .. } => atoms::set(),
            RequestKind::SetBytes { .. } => atoms::set_bytes(),
            RequestKind::SupportPolling(_) => atoms::support_polling_updates_for(),
        }
    }

    fn encode_payload<'a>(&self, env: Env<'a>) -> Term<'a> {
        match self {
            RequestKind::Initialize | RequestKind::Shutdown => atoms::no_payload().encode(env),
            RequestKind::Get(key) => key.encode(env),
            RequestKind::GetBytes(key) => key.encode(env),
            RequestKind::Set { key, value, time } => {
                (key.clone(), value.clone(), *time).encode(env)
            }
            RequestKind::SetBytes { key, value, time } => {
                let mut binary =
                    OwnedBinary::new(value.len()).expect("failed to allocate Elixir binary");
                binary.copy_from_slice(value);
                let binary = Binary::from_owned(binary, env);

                (key.clone(), binary, *time).encode(env)
            }
            RequestKind::SupportPolling(path) => path.encode(env),
        }
    }

    fn response_kind(&self) -> ResponseKind {
        match self {
            RequestKind::Initialize
            | RequestKind::Shutdown
            | RequestKind::Set { .. }
            | RequestKind::SetBytes { .. } => ResponseKind::Unit,
            RequestKind::Get(_) => ResponseKind::Data,
            RequestKind::GetBytes(_) => ResponseKind::BytesData,
            RequestKind::SupportPolling(_) => ResponseKind::Bool,
        }
    }

    fn is_bytes_request(&self) -> bool {
        matches!(
            self,
            RequestKind::GetBytes(_) | RequestKind::SetBytes { .. }
        )
    }
}

#[derive(Clone, Copy)]
pub enum ResponseKind {
    Unit,
    Data,
    BytesData,
    Bool,
}

pub enum ResponsePayload {
    Unit,
    Data(DataStoreResponse),
    BytesData(DataStoreBytesResponse),
    Bool(bool),
}

pub struct DataStoreRequestResource {
    response_kind: ResponseKind,
    is_bytes_request: bool,
    sender: Mutex<Option<oneshot::Sender<Result<ResponsePayload, StatsigErr>>>>,
}

impl DataStoreRequestResource {
    pub fn new(
        response_kind: ResponseKind,
        is_bytes_request: bool,
        sender: oneshot::Sender<Result<ResponsePayload, StatsigErr>>,
    ) -> Self {
        Self {
            response_kind,
            is_bytes_request,
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
            ResponseKind::BytesData => payload
                .decode::<StatsigDataStoreBytesResponse>()
                .map(|resp| ResponsePayload::BytesData(resp.into()))
                .map_err(|err| {
                    self.send_err(format!("Failed to decode bytes data payload: {err:?}"));
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
            let error = if self.is_bytes_request && reason == "BytesNotImplemented" {
                StatsigErr::BytesNotImplemented
            } else {
                StatsigErr::DataStoreFailure(reason)
            };

            let _ = sender.send(Err(error));
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

impl From<StatsigDataStoreBytesResponse<'_>> for DataStoreBytesResponse {
    fn from(value: StatsigDataStoreBytesResponse) -> Self {
        DataStoreBytesResponse {
            result: value.result.map(|result| result.as_slice().to_vec()),
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
