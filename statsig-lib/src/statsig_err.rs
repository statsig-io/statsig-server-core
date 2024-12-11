use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum StatsigErr {
    CustomError(String),

    LockFailure(String),

    UnstartedAdapter(String),

    NetworkError(String),

    JsonParseError(String, String),

    FileError(String),

    ThreadFailure(String),

    StackOverflowError,

    // DataStore
    DataStoreFailure(String),
    DataStoreSkipPoll,

    // ID Lists Adapter
    IdListsAdapterFailedToInsertIdList,

    GrpcError(String),

    ShutdownTimeout,

    // Task Scheduler
    ScheduleFailure(String),
    ShutdownFailure,
}

impl Display for StatsigErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatsigErr::CustomError(msg) => write!(f, "{}", msg),

            StatsigErr::LockFailure(msg) => write!(f, "Failed to acquire lock: {}", msg),

            StatsigErr::UnstartedAdapter(msg) => write!(f, "Adapter not started: {}", msg),

            StatsigErr::NetworkError(msg) => write!(f, "Network error: {}", msg),

            StatsigErr::JsonParseError(type_name, err_msg) => {
                write!(f, "Failed to parse {} - {}", type_name, err_msg)
            }

            StatsigErr::FileError(msg) => write!(f, "File write error: {}", msg),

            StatsigErr::ThreadFailure(msg) => write!(f, "Thread failure: {}", msg),

            StatsigErr::StackOverflowError => write!(f, "Statsig Evaluation Depth Exceeded"),

            // ID Lists
            StatsigErr::IdListsAdapterFailedToInsertIdList => {
                write!(f, "Failed to insert new Id List")
            }

            StatsigErr::GrpcError(e) => write!(f, "{}", e),

            StatsigErr::ShutdownTimeout => write!(f, "Shutdown timed out"),

            StatsigErr::ScheduleFailure(e) => write!(f, "Failed to schedule task: {}", e),

            StatsigErr::ShutdownFailure => write!(f, "Failed to shutdown task scheduler"),
            StatsigErr::DataStoreFailure(message) => write!(f, "DataStore Error: {}", message),
            StatsigErr::DataStoreSkipPoll => write!(f, "DataStore stops polling"),
        }
    }
}
