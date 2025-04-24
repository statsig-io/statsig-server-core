use std::fmt::{Display, Formatter};

use serde::Serialize;

use crate::networking::NetworkError;

#[derive(Debug, Clone, Serialize)]
pub enum StatsigErr {
    // Custom
    CustomError(String),

    // System / Concurrency
    LockFailure(String),
    ThreadFailure(String),
    StackOverflowError,
    SharedInstanceFailure(String),
    ObservabilityClientFailure(String),

    // Adapter
    UnstartedAdapter(String),
    IdListsAdapterFailedToInsertIdList,
    SpecsAdapterSkipPoll(String),
    DataStoreFailure(String),

    // Network
    NetworkError(NetworkError, Option<String>),
    GrpcError(String),

    // Data Format / Serialization / Parsing
    SerializationError(String),
    JsonParseError(String, String),

    // Compression
    ZstdDictCompressionError(String),
    GzipError(String),
    ZstdError(String),

    // Filesystem
    FileError(String),

    // Logging
    LogEventError(String),

    // Evaluation
    EvaluationError(String),

    // Initialization / Shutdown
    InitializationError(String),
    ShutdownFailure(String),

    // Task Scheduler
    ScheduleFailure(String),
    TaskShutdownFailure,
}

impl Display for StatsigErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatsigErr::CustomError(msg) => write!(f, "{msg}"),

            StatsigErr::LockFailure(msg) => write!(f, "Failed to acquire lock: {msg}"),
            StatsigErr::ThreadFailure(msg) => write!(f, "Thread failure: {msg}"),
            StatsigErr::StackOverflowError => write!(f, "Statsig Evaluation Depth Exceeded"),
            StatsigErr::SharedInstanceFailure(message) => {
                write!(f, "SharedInstance Error: {message}")
            }
            StatsigErr::ObservabilityClientFailure(message) => {
                write!(f, "ObservabilityClient Error: {message}")
            }

            StatsigErr::UnstartedAdapter(msg) => write!(f, "Adapter not started: {msg}"),
            StatsigErr::IdListsAdapterFailedToInsertIdList => {
                write!(f, "Failed to insert new Id List")
            }
            StatsigErr::SpecsAdapterSkipPoll(adapter_name) => {
                write!(f, "{adapter_name} skips scheduling polling")
            }
            StatsigErr::DataStoreFailure(message) => write!(f, "DataStore Error: {message}"),

            StatsigErr::NetworkError(error, msg) => write!(f, "Network error {error}: {msg:?}"),
            StatsigErr::GrpcError(e) => write!(f, "{e}"),

            StatsigErr::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            StatsigErr::JsonParseError(type_name, err_msg) => {
                write!(f, "Failed to parse {type_name} - {err_msg}")
            }

            StatsigErr::ZstdDictCompressionError(msg) => {
                write!(f, "Zstd dictionary compression error: {msg}")
            }
            StatsigErr::GzipError(msg) => write!(f, "Gzip error: {msg}"),
            StatsigErr::ZstdError(msg) => write!(f, "Zstd error: {msg}"),

            StatsigErr::FileError(msg) => write!(f, "File write error: {msg}"),

            StatsigErr::LogEventError(msg) => write!(f, "Log event error: {msg}"),

            StatsigErr::EvaluationError(message) => {
                write!(f, "Evaluation Error: {message}")
            }

            StatsigErr::InitializationError(message) => {
                write!(f, "Initialization Error: {message}")
            }
            StatsigErr::ShutdownFailure(e) => write!(f, "Shutdown failure: {e}"),

            StatsigErr::ScheduleFailure(e) => write!(f, "Failed to schedule task: {e}"),
            StatsigErr::TaskShutdownFailure => write!(f, "Failed to shutdown task scheduler"),
        }
    }
}

impl StatsigErr {
    pub fn name(&self) -> &'static str {
        match self {
            StatsigErr::CustomError(_) => "CustomError",

            StatsigErr::LockFailure(_) => "LockFailure",
            StatsigErr::ThreadFailure(_) => "ThreadFailure",
            StatsigErr::StackOverflowError => "StackOverflowError",
            StatsigErr::SharedInstanceFailure(_) => "SharedInstanceFailure",
            StatsigErr::ObservabilityClientFailure(_) => "ObservabilityClientFailure",

            StatsigErr::UnstartedAdapter(_) => "UnstartedAdapter",
            StatsigErr::IdListsAdapterFailedToInsertIdList => "IdListsAdapterFailedToInsertIdList",
            StatsigErr::SpecsAdapterSkipPoll(_) => "SpecsAdapterSkipPoll",
            StatsigErr::DataStoreFailure(_) => "DataStoreFailure",

            StatsigErr::NetworkError(_, _) => "NetworkError",
            StatsigErr::GrpcError(_) => "GrpcError",

            StatsigErr::SerializationError(_) => "SerializationError",
            StatsigErr::JsonParseError(_, _) => "JsonParseError",

            StatsigErr::ZstdDictCompressionError(_) => "ZstdDictCompressionError",
            StatsigErr::GzipError(_) => "GzipError",
            StatsigErr::ZstdError(_) => "ZstdError",

            StatsigErr::FileError(_) => "FileError",

            StatsigErr::LogEventError(_) => "LogEventError",

            StatsigErr::EvaluationError(_) => "EvaluationError",

            StatsigErr::InitializationError(_) => "InitializationError",
            StatsigErr::ShutdownFailure(_) => "ShutdownFailure",

            StatsigErr::ScheduleFailure(_) => "ScheduleFailure",
            StatsigErr::TaskShutdownFailure => "TaskShutdownFailure",
        }
    }
}
