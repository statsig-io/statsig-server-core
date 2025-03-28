use async_trait::async_trait;

use crate::{
    networking::{HttpMethod, NetProviderRequestArgs, NetworkProvider, Response},
    StatsigErr,
};

pub struct NetworkProviderNoop;

#[async_trait]
impl NetworkProvider for NetworkProviderNoop {
    async fn send(&self, _method: &HttpMethod, _request_args: &NetProviderRequestArgs) -> Response {
        Response {
            status_code: 0,
            data: None,
            error: Some("No Network Provider Set".to_string()),
            headers: None,
        }
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Ok(())
    }
}
