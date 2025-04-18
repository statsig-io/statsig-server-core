use async_trait::async_trait;

use crate::networking::{HttpMethod, NetworkProvider, RequestArgs, Response};

pub struct NetworkProviderNoop;

#[async_trait]
impl NetworkProvider for NetworkProviderNoop {
    async fn send(&self, _method: &HttpMethod, _request_args: &RequestArgs) -> Response {
        Response {
            status_code: 0,
            data: None,
            error: Some("No Network Provider Set".to_string()),
            headers: None,
        }
    }
}
