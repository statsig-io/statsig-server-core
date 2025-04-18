use std::collections::HashMap;

use async_trait::async_trait;

use pyo3::prelude::*;
use statsig_rust::networking::{HttpMethod, NetworkProvider, RequestArgs, Response};

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

        Python::with_gil(|py| {
            match self
                .network_func
                .call1(py, (method_str, url, headers, body))
            {
                Ok(result) => get_response_from_py_result(py, result),
                Err(e) => Response {
                    status_code: 0,
                    data: None,
                    error: Some(format!("NetworkProviderPy Request Error: {}", e)),
                    headers: None,
                },
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
        Err(e) => Response {
            status_code: 0,
            data: None,
            error: Some(format!("NetworkProviderPy Response Error: {}", e)),
            headers: None,
        },
    }
}
