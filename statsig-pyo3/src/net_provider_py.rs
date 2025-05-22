use std::collections::HashMap;

use async_trait::async_trait;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use statsig_rust::networking::{HttpMethod, NetworkProvider, RequestArgs, Response};

use crate::safe_gil::SafeGil;

#[derive(FromPyObject)]
struct ResponsePy(
    u16,                             // status code
    Option<Vec<u8>>,                 // data
    Option<String>,                  // error
    Option<HashMap<String, String>>, // headers
);

pub struct NetworkProviderPy {
    pub network_func: PyObject,
}

#[async_trait]
impl NetworkProvider for NetworkProviderPy {
    async fn send(&self, method: &HttpMethod, request_args: &RequestArgs) -> Response {
        let method_str = match method {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
        }
        .to_string();

        let url = request_args.get_fully_qualified_url();
        let headers = request_args.headers.clone().unwrap_or_default();
        let body = request_args.body.clone();
        let proxy_url = try_create_proxy_url(request_args);

        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return create_error_response("Python GIL not available", None),
            };

            let proxy_config_py = proxy_url
                .map(|url| {
                    let dict = PyDict::new(py);
                    dict.set_item("http", &url).ok();
                    dict.set_item("https", &url).ok();
                    dict.into()
                })
                .unwrap_or_else(|| py.None());

            match self
                .network_func
                .call1(py, (method_str, url, headers, body, proxy_config_py))
            {
                Ok(result) => get_response_from_py_result(py, result),
                Err(e) => create_error_response("Request Error", Some(e)),
            }
        })
    }
}

fn get_response_from_py_result(py: Python, result: PyObject) -> Response {
    match result.extract::<ResponsePy>(py) {
        Ok(tuple) => Response {
            status_code: tuple.0,
            data: tuple.1,
            error: tuple.2,
            headers: tuple.3,
        },
        Err(e) => create_error_response("Response Error", Some(e)),
    }
}

fn try_create_proxy_url(args: &RequestArgs) -> Option<String> {
    let proxy = args.proxy_config.as_ref()?;

    let scheme = proxy
        .proxy_protocol
        .clone()
        .unwrap_or_else(|| "http".to_string());

    let host = proxy.proxy_host.as_deref().unwrap_or("");
    let port = proxy.proxy_port;
    let auth_part = proxy
        .proxy_auth
        .as_ref()
        .map(|auth| format!("{}@", auth))
        .unwrap_or_default();

    let url = if let Some(port) = port {
        format!("{}://{}{}:{}", scheme, auth_part, host, port)
    } else {
        format!("{}://{}{}", scheme, auth_part, host)
    };

    Some(url)
}

fn create_error_response(message: &str, e: Option<PyErr>) -> Response {
    let error_message = match e {
        Some(e) => format!("NetworkProviderPy {}: {}", message, e),
        None => format!("NetworkProviderPy {}", message),
    };

    Response {
        status_code: 0,
        data: None,
        error: Some(error_message),
        headers: None,
    }
}
