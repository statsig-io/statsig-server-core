use serde::Serialize;
use std::fmt;

use crate::logging_utils::sanitize_secret_key;

type RequestUrl = String;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum NetworkError {
    ShutdownError(RequestUrl),
    DisableNetworkOn(RequestUrl),
    SerializationError(RequestUrl, String),

    RequestFailed(RequestUrl, Option<u16>, String),
    RetriesExhausted(RequestUrl, Option<u16>, u32, String),
    RequestNotRetryable(RequestUrl, Option<u16>, String),
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
            NetworkError::ShutdownError(url) => {
                let url = sanitize_secret_key(url);
                write!(f, "ShutdownError: {url}")
            }
            NetworkError::DisableNetworkOn(url) => {
                let url = sanitize_secret_key(url);
                write!(f, "DisableNetworkOn: {url}")
            }
            NetworkError::SerializationError(url, s) => {
                let url = sanitize_secret_key(url);
                let s = sanitize_secret_key(s);
                write!(f, "SerializationError: {url} {s}")
            }

            NetworkError::RequestFailed(url, status, message) => {
                let url = sanitize_secret_key(url);
                let message = sanitize_secret_key(message);
                let status_display = match status {
                    Some(code) => code.to_string(),
                    None => "None".to_string(),
                };
                write!(f, "RequestFailed: {url} {status_display} {message}")
            }
            NetworkError::RetriesExhausted(url, status, attempts, message) => {
                let url = sanitize_secret_key(url);
                let message = sanitize_secret_key(message);
                let status_display = match status {
                    Some(code) => code.to_string(),
                    None => "None".to_string(),
                };
                write!(
                    f,
                    "RetriesExhausted: {url} status({status_display}) attempts({attempts}) {message}"
                )
            }
            NetworkError::RequestNotRetryable(url, status, message) => {
                let url = sanitize_secret_key(url);
                let message = sanitize_secret_key(message);
                let status_display = match status {
                    Some(code) => code.to_string(),
                    None => "None".to_string(),
                };
                write!(
                    f,
                    "RequestNotRetryable: {url} status({status_display}) {message}"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_secret_key_in_error_display() {
        let err = NetworkError::RetriesExhausted(
            "https://api.statsigcdn.com/v2/download_config_specs/secret-fakeO1234567890.json"
                .to_string(),
            None,
            1,
            "Invalid array length".to_string(),
        );

        let message = err.to_string();
        assert!(message.contains("secret-fakeO*****.json"));
        assert!(!message.contains("secret-fakeO1234567890"));
    }
}
