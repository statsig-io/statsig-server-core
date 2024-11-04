use std::fmt::{Display, Formatter};

use tonic::Status;

#[derive(Debug)]
pub enum StatsigGrpcErr {
    FailedToGetLock,
    FailedToConnect(String),
    ErrorGrpcStatus(Status),
    CustomErr(String),
}

impl Display for StatsigGrpcErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatsigGrpcErr::CustomErr(msg) => write!(f, "{}", msg),
            StatsigGrpcErr::FailedToConnect(msg) => write!(f, "Failed to connect to GRPC Servers {}", msg),
            StatsigGrpcErr::FailedToGetLock => {
              write!(f, "Failed to acquire lock")
            }
            StatsigGrpcErr::ErrorGrpcStatus(s) => write!(f, "GRPC error status {}", s.message()),
        }
    }
}
