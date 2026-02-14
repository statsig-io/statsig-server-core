use crate::networking::proxy_config::ProxyConfig;
use crate::sdk_diagnostics::marker::KeyType;
use crate::StatsigErr;
use async_trait::async_trait;
use chrono::Utc;
use serde::de::DeserializeOwned;
use std::io::Cursor;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

#[derive(Clone)]
pub struct RequestArgs {
    pub url: String,
    pub body: Option<Vec<u8>>,
    pub retries: u32, // 1 initial + N 'retries'
    pub headers: Option<HashMap<String, String>>,
    pub query_params: Option<HashMap<String, String>>,
    pub deltas_enabled: bool,
    pub accept_gzip_response: bool,
    pub timeout_ms: u64,
    pub is_shutdown: Option<Arc<AtomicBool>>,
    pub diagnostics_key: Option<KeyType>,
    pub proxy_config: Option<ProxyConfig>,
    pub disable_file_streaming: Option<bool>,
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
            deltas_enabled: false,
            accept_gzip_response: false,
            timeout_ms: 0,
            is_shutdown: None,
            diagnostics_key: None,
            proxy_config: None,
            disable_file_streaming: None,
        }
    }

    pub fn get_fully_qualified_url(&self) -> String {
        let mut url = self.url.clone();
        let query_params = match &self.query_params {
            Some(params) => params,
            None => return url,
        };

        let query_params_str = query_params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");

        if !query_params_str.is_empty() {
            url.push_str(&format!("?{query_params_str}"));
        }

        url
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
    pub status_code: Option<u16>,
    pub data: Option<ResponseData>,
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

pub trait ResponseDataStream:
    std::io::Read + std::io::Seek + std::fmt::Debug + Send + Sync
{
}

impl<T: std::io::Read + std::io::Seek + std::fmt::Debug + Send + Sync> ResponseDataStream for T {}

pub struct ResponseData {
    stream: Box<dyn ResponseDataStream>,
    headers: Option<HashMap<String, String>>,
}

const TAG: &str = "ResponseData";

impl ResponseData {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_bytes_with_headers(bytes, None)
    }

    pub fn from_bytes_with_headers(
        bytes: Vec<u8>,
        headers: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            stream: Box::new(Cursor::new(bytes)),
            headers,
        }
    }

    pub fn from_stream(stream: Box<dyn ResponseDataStream>) -> Self {
        Self::from_stream_with_headers(stream, None)
    }

    pub fn from_stream_with_headers(
        stream: Box<dyn ResponseDataStream>,
        headers: Option<HashMap<String, String>>,
    ) -> Self {
        Self { stream, headers }
    }

    pub fn get_stream_ref(&self) -> &dyn ResponseDataStream {
        &self.stream
    }

    pub fn get_stream_mut(&mut self) -> &mut dyn ResponseDataStream {
        &mut self.stream
    }

    pub fn get_header_ref(&self, key: &str) -> Option<&String> {
        self.headers.as_ref().and_then(|h| h.get(key))
    }

    pub fn deserialize_into<T: DeserializeOwned>(&mut self) -> Result<T, StatsigErr> {
        self.rewind()?;

        let result = serde_json::from_reader(self.stream.as_mut())
            .map_err(|e| StatsigErr::JsonParseError(TAG.to_string(), e.to_string()))?;

        Ok(result)
    }

    pub fn deserialize_in_place<T: DeserializeOwned>(
        &mut self,
        place: &mut T,
    ) -> Result<(), StatsigErr> {
        self.rewind()?;

        let mut deserializer = serde_json::Deserializer::from_reader(self.stream.as_mut());

        T::deserialize_in_place(&mut deserializer, place)
            .map_err(|e| StatsigErr::JsonParseError(TAG.to_string(), e.to_string()))
    }

    pub fn read_to_string(&mut self) -> Result<String, StatsigErr> {
        self.rewind()?;

        let mut buf = Vec::new();

        self.stream
            .read_to_end(&mut buf)
            .map_err(|e| StatsigErr::SerializationError(e.to_string()))?;

        String::from_utf8(buf)
            .map_err(|e| StatsigErr::JsonParseError(TAG.to_string(), e.to_string()))
    }

    fn rewind(&mut self) -> Result<(), StatsigErr> {
        self.stream
            .rewind()
            .map_err(|e| StatsigErr::SerializationError(e.to_string()))
    }
}
