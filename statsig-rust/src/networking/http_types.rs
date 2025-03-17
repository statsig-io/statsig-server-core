use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

#[derive(Clone)]
pub struct RequestArgs {
    pub url: String,
    pub body: Option<Vec<u8>>,
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
    #[must_use]
    pub fn new() -> Self {
        RequestArgs {
            url: String::new(),
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
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

#[derive(PartialEq, Clone)]
pub enum HttpMethod {
    GET,
    POST,
}

#[async_trait]
pub trait NetworkProvider: Sync + Send {
    async fn send(&self, method: &HttpMethod, args: &RequestArgs) -> Response;
}
