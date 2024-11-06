use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum StatsigErr {
    CustomError(String),

    LockFailure(String),

    UnstartedAdapter(String),

    NetworkError(String),

    JsonParseError(String, String),

    FileError(String),

    // Specs Adapter
    SpecsLocalFileReadFailure(String),

    // ID Lists Adapter
    IdListsAdapterFailedToInsertIdList,

    GrpcError(String),
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

            // Specs
            StatsigErr::SpecsLocalFileReadFailure(e) => {
                write!(f, "Specs adapter failed to read local file, {}", e)
            }

            // ID Lists
            StatsigErr::IdListsAdapterFailedToInsertIdList => {
                write!(f, "Failed to insert new Id List")
            }

            StatsigErr::GrpcError(e) => write!(f, "{}", e),
        }
    }
}
