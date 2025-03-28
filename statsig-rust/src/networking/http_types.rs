use async_trait::async_trait;
use chrono::Utc;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use crate::sdk_diagnostics::marker::KeyType;
use crate::StatsigErr;

pub type RequestArgs = RequestArgsTyped<String>;

impl Default for RequestArgs {
    fn default() -> Self {
        Self::new(|data| {
            String::from_utf8(data.unwrap_or_default()).map_err(|_| {
                StatsigErr::SerializationError(
                    "Failed to deserialize response as a string".to_string(),
                )
            })
        })
    }
}

#[derive(Clone)]
pub struct RequestArgsTyped<T>
where
    T: Clone,
{
    pub url: String,
    pub body: Option<Vec<u8>>,
    pub retries: u32,
    pub headers: Option<HashMap<String, String>>,
    pub query_params: Option<HashMap<String, String>>,
    pub accept_gzip_response: bool,
    pub timeout_ms: u64,
    pub is_shutdown: Option<Arc<AtomicBool>>,
    pub diagnostics_key: Option<KeyType>,
    pub response_deserializer: fn(Option<Vec<u8>>) -> Result<T, StatsigErr>,
}

impl<T> AsRef<RequestArgsTyped<T>> for RequestArgsTyped<T>
where
    T: Clone,
{
    fn as_ref(&self) -> &RequestArgsTyped<T> {
        self
    }
}

#[derive(Clone)]
pub struct NetProviderRequestArgs {
    pub url: String,
    pub body: Option<Vec<u8>>,
    pub retries: u32,
    pub headers: Option<HashMap<String, String>>,
    pub query_params: Option<HashMap<String, String>>,
    pub accept_gzip_response: bool,
    pub timeout_ms: u64,
    pub is_shutdown: Option<Arc<AtomicBool>>,
    pub diagnostics_key: Option<KeyType>,
}

impl<T> From<RequestArgsTyped<T>> for NetProviderRequestArgs
where
    T: Clone,
{
    fn from(val: RequestArgsTyped<T>) -> Self {
        NetProviderRequestArgs {
            url: val.url,
            body: val.body,
            retries: val.retries,
            headers: val.headers,
            query_params: val.query_params,
            accept_gzip_response: val.accept_gzip_response,
            timeout_ms: val.timeout_ms,
            is_shutdown: val.is_shutdown,
            diagnostics_key: val.diagnostics_key,
        }
    }
}

impl NetProviderRequestArgs {
    pub fn get_fully_qualified_url(&self) -> String {
        let mut url = self.url.clone();
        let query_params = match &self.query_params {
            Some(params) => params,
            None => return url,
        };

        let query_params_str = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        if !query_params_str.is_empty() {
            url.push_str(&format!("?{}", query_params_str));
        }

        url
    }
}

impl<T> RequestArgsTyped<T>
where
    T: Clone,
{
    #[must_use]
    pub fn new(deserializer: fn(Option<Vec<u8>>) -> Result<T, StatsigErr>) -> Self {
        RequestArgsTyped {
            url: String::new(),
            body: None,
            retries: 0,
            headers: None,
            query_params: None,
            accept_gzip_response: false,
            timeout_ms: 0,
            is_shutdown: None,
            diagnostics_key: None,
            response_deserializer: deserializer,
        }
    }

    pub fn populate_headers(&mut self, extra_headers: HashMap<String, String>) {
        let mut headers = HashMap::new();
        headers.extend(extra_headers);

        headers.insert(
            "STATSIG-CLIENT-TIME".into(),
            Utc::now().timestamp_millis().to_string(),
        );

        if let Some(my_headers) = &mut self.headers {
            my_headers.extend(headers);
        } else {
            self.headers = Some(headers);
        }
    }
}

pub struct Response {
    pub status_code: u16,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(PartialEq, Clone)]
pub enum HttpMethod {
    GET,
    POST,
}

#[async_trait]
pub trait NetworkProvider: Sync + Send {
    async fn send(&self, method: &HttpMethod, args: &NetProviderRequestArgs) -> Response;
    async fn shutdown(&self) -> Result<(), StatsigErr>;
}
