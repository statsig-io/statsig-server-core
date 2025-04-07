use std::collections::HashMap;

use async_trait::async_trait;
use napi::{
    bindgen_prelude::{FnArgs, Promise},
    threadsafe_function::ThreadsafeFunction,
};

use napi_derive::napi;
use statsig_rust::{
    networking::{HttpMethod, NetworkProvider, RequestArgs, Response},
    StatsigErr,
};

type NapiNetworkFuncArgs = FnArgs<(String, String, HashMap<String, String>, Option<Vec<u8>>)>;

pub type NapiNetworkFunc = ThreadsafeFunction<
    NapiNetworkFuncArgs,
    Promise<NapiNetworkFuncResult>,
    NapiNetworkFuncArgs,
    false,
>;

#[napi(object)]
pub struct NapiNetworkFuncResult {
    pub status: u32,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

pub struct NetworkProviderNapi {
    pub network_func: NapiNetworkFunc,
}

#[async_trait]
impl NetworkProvider for NetworkProviderNapi {
    async fn send(&self, method: &HttpMethod, request_args: &RequestArgs) -> Response {
        let method_str = match method {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
        }
        .to_string();

        let url = request_args.get_fully_qualified_url();
        let headers = request_args.headers.clone().unwrap_or_default();

        let js_promise = match self
            .network_func
            .call_async((method_str, url, headers, request_args.body.clone()).into())
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return Response {
                    status_code: 0,
                    data: None,
                    error: Some(format!("NapiFetchFnInvocationError: {}", e)),
                    headers: None,
                };
            }
        };

        let result = match js_promise.await {
            Ok(result) => result,
            Err(e) => {
                return Response {
                    status_code: 0,
                    data: None,
                    error: Some(format!("NapiFetchFnPromiseRejection: {}", e)),
                    headers: None,
                };
            }
        };

        Response {
            status_code: result.status as u16,
            data: result.data,
            error: result.error,
            headers: None,
        }
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        // noop
        Ok(())
    }
}
