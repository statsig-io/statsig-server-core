use std::fmt::{Display, Formatter};

use crate::networking::NetworkError;

#[derive(Debug, Clone)]
pub enum StatsigErr {
    CustomError(String),

    LockFailure(String),

    UnstartedAdapter(String),

    NetworkError(NetworkError, Option<String>),

    SerializationError(String),

    ZstdDictCompressionError(String),

    GzipError(String),

    ZstdError(String),

    JsonParseError(String, String),

    FileError(String),

    ThreadFailure(String),

    StackOverflowError,

    // DataStore
    DataStoreFailure(String),

    // Skip poll
    SpecsAdapterSkipPoll(String),

    ObservabilityClientFailure(String),

    // ID Lists Adapter
    IdListsAdapterFailedToInsertIdList,

    GrpcError(String),

    ShutdownTimeout,

    // Task Scheduler
    ScheduleFailure(String),
    ShutdownFailure,

    SharedInstanceFailure(String),
}

impl Display for StatsigErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatsigErr::CustomError(msg) => write!(f, "{msg}"),

            StatsigErr::LockFailure(msg) => write!(f, "Failed to acquire lock: {msg}"),

            StatsigErr::UnstartedAdapter(msg) => write!(f, "Adapter not started: {msg}"),

            StatsigErr::NetworkError(error, msg) => write!(f, "Network error {error}: {msg:?}"),

            StatsigErr::SerializationError(msg) => write!(f, "Serialization error: {msg}"),

            StatsigErr::ZstdDictCompressionError(msg) => {
                write!(f, "Zstd dictionary compression error: {msg}")
            }

            StatsigErr::GzipError(msg) => write!(f, "Gzip error: {msg}"),

            StatsigErr::ZstdError(msg) => write!(f, "Zstd error: {msg}"),

            StatsigErr::JsonParseError(type_name, err_msg) => {
                write!(f, "Failed to parse {type_name} - {err_msg}")
            }

            StatsigErr::FileError(msg) => write!(f, "File write error: {msg}"),

            StatsigErr::ThreadFailure(msg) => write!(f, "Thread failure: {msg}"),

            StatsigErr::StackOverflowError => write!(f, "Statsig Evaluation Depth Exceeded"),

            // ID Lists
            StatsigErr::IdListsAdapterFailedToInsertIdList => {
                write!(f, "Failed to insert new Id List")
            }

            StatsigErr::GrpcError(e) => write!(f, "{e}"),

            StatsigErr::ShutdownTimeout => write!(f, "Shutdown timed out"),

            StatsigErr::ScheduleFailure(e) => write!(f, "Failed to schedule task: {e}"),

            StatsigErr::ShutdownFailure => write!(f, "Failed to shutdown task scheduler"),
            StatsigErr::DataStoreFailure(message) => write!(f, "DataStore Error: {message}"),
            StatsigErr::SpecsAdapterSkipPoll(adapter_name) => {
                write!(f, "{adapter_name} skips scheduling polling")
            }
            StatsigErr::ObservabilityClientFailure(message) => {
                write!(f, "ObservabilityClient Error: {message}")
            }
            StatsigErr::SharedInstanceFailure(message) => {
                write!(f, "SharedInstance Error: {message}")
            }
        }
    }
}
