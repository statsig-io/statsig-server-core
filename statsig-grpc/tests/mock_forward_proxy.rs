use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tonic::{Request, Response, Status};
use tonic::codegen::tokio_stream;
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use statsig_grpc::{ConfigSpecRequest, ConfigSpecResponse, StatsigForwardProxy, StatsigForwardProxyServer};

pub async fn wait_one_ms() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}
pub struct MockForwardProxyVerification {
    pub proxy_address: String,
    pub stubbed_get_config_spec_response: Mutex<ConfigSpecResponse>,

    stream_tx: Mutex<Sender<Result<ConfigSpecResponse, Status>>>,
    stream_rx: Mutex<Option<Receiver<Result<ConfigSpecResponse, Status>>>>,
}

impl MockForwardProxyVerification {
    pub async fn send_stream_update(&self, update: Result<ConfigSpecResponse, Status>) {
        let tx = self.stream_tx.lock().unwrap();
        tx.send(update).await.unwrap();
    }
}

pub struct MockForwardProxy {
    pub verification: Arc<MockForwardProxyVerification>,
}

impl MockForwardProxy {
    pub async fn spawn() -> Arc<MockForwardProxyVerification> {
        let mock_addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        let mock_verification = Arc::new(MockForwardProxyVerification {
            proxy_address: format!("http://{}", mock_addr.to_string()),
            stubbed_get_config_spec_response: Mutex::new(ConfigSpecResponse {
                spec: "NOT STUBBED".to_string(),
                last_updated: 0,
            }),
            stream_tx: Mutex::new(tx),
            stream_rx: Mutex::new(Some(rx))
        });

        let mock_service = MockForwardProxy {
            verification: mock_verification.clone()
        };

        let server = Server::builder()
            .add_service(StatsigForwardProxyServer::new(mock_service))
            .serve(mock_addr);

        tokio::spawn(server);
        wait_one_ms().await;

        mock_verification
    }
}

#[tonic::async_trait]
impl StatsigForwardProxy for MockForwardProxy {
    async fn get_config_spec(
        &self,
        _request: Request<ConfigSpecRequest>,
    ) -> Result<Response<ConfigSpecResponse>, Status> {
        let response = self.verification.stubbed_get_config_spec_response.lock().unwrap().clone();
        Ok(Response::new(response))
    }

    type StreamConfigSpecStream = tokio_stream::wrappers::ReceiverStream<Result<ConfigSpecResponse, Status>>;

    async fn stream_config_spec(
        &self,
        _request: Request<ConfigSpecRequest>,
    ) -> Result<Response<Self::StreamConfigSpecStream>, Status> {
        let rx = self.verification.stream_rx.lock().unwrap().take().unwrap();

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(stream))
    }
}
