use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};
use bytes::Bytes;

#[derive(Clone)]
pub struct RequestArgs {
    pub url: String,
    pub body: Option<Bytes>,
    pub retries: u32,
    pub headers: Option<HashMap<String, String>>,
    pub query_params: Option<HashMap<String, String>>,
    pub accept_gzip_response: bool,
    pub timeout_ms: u64,
    pub is_shutdown: Option<Arc<AtomicBool>>,
}

impl Default for RequestArgs {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestArgs {
    pub fn new() -> Self {
        RequestArgs {
            url: "".to_string(),
            body: None,
            retries: 0,
            headers: None,
            query_params: None,
            accept_gzip_response: false,
            timeout_ms: 0,
            is_shutdown: None,
        }
    }
}

pub struct Response {
    pub status_code: u16,
    pub data: Option<String>,
    pub error: Option<String>,
}

#[derive(PartialEq, Clone)]
pub enum HttpMethod {
    GET,
    POST,
}
