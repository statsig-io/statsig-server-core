use serde::Serialize;
use std::fmt;

type RequestUrl = String;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum NetworkError {
    ShutdownError(RequestUrl),
    DisableNetworkOn(RequestUrl),
    SerializationError(RequestUrl, String),

    RequestFailed(RequestUrl, u16, String),
    RetriesExhausted(RequestUrl, u16, u32, String),
    RequestNotRetryable(RequestUrl, u16, String),
}

impl NetworkError {
    pub fn name(&self) -> &'static str {
        match self {
            NetworkError::ShutdownError(_) => "ShutdownError",
            NetworkError::DisableNetworkOn(_) => "DisableNetworkOn",
            NetworkError::SerializationError(_, _) => "SerializationError",
            NetworkError::RequestFailed(_, _, _) => "RequestFailed",
            NetworkError::RetriesExhausted(_, _, _, _) => "RetriesExhausted",
            NetworkError::RequestNotRetryable(_, _, _) => "RequestNotRetryable",
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ShutdownError(url) => write!(f, "ShutdownError: {url}"),
            NetworkError::DisableNetworkOn(url) => write!(f, "DisableNetworkOn: {url}"),
            NetworkError::SerializationError(url, s) => write!(f, "SerializationError: {url} {s}"),

            NetworkError::RequestFailed(url, status, message) => {
                write!(f, "RequestFailed: {url} {status} {message}")
            }
            NetworkError::RetriesExhausted(url, status, attempts, message) => {
                write!(
                    f,
                    "RetriesExhausted: {url} status({status}) attempts({attempts}) {message}"
                )
            }
            NetworkError::RequestNotRetryable(url, status, message) => {
                write!(f, "RequestNotRetryable: {url} status({status}) {message}")
            }
        }
    }
}
