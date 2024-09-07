use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use lazy_static::lazy_static;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tonic::{Request, Response, Status};
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use sigstat::log_e;
use statsig_grpc::{ConfigSpecRequest, ConfigSpecResponse, StatsigForwardProxy, StatsigForwardProxyServer};

lazy_static! {
    static ref PORT_ID: AtomicI32 = AtomicI32::new(50051);
}

pub async fn wait_one_ms() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}
pub struct MockForwardProxy {
    pub proxy_address: SocketAddr,
    pub stubbed_get_config_spec_response: Mutex<ConfigSpecResponse>,

    shutdown_notifier: Arc<Notify>,
    server_handle: Mutex<Option<JoinHandle<()>>>,

    stream_tx: Mutex<Option<Sender<Result<ConfigSpecResponse, Status>>>>,
    stream_rx: Mutex<Option<Receiver<Result<ConfigSpecResponse, Status>>>>,
}

impl MockForwardProxy {
    pub async fn spawn() -> Arc<MockForwardProxy> {
        let port = PORT_ID.fetch_add(1, Ordering::SeqCst);
        let proxy_address: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();


        let forward_proxy = Arc::new(MockForwardProxy {
            proxy_address,
            stubbed_get_config_spec_response: Mutex::new(ConfigSpecResponse {
                spec: "NOT STUBBED".to_string(),
                last_updated: 0,
            }),

            shutdown_notifier: Arc::new(Notify::new()),
            server_handle: Mutex::new(None),

            stream_tx: Mutex::new(None),
            stream_rx: Mutex::new(None),
        });

        forward_proxy.clone().restart().await;
        forward_proxy
    }

    pub async fn send_stream_update(&self, update: Result<ConfigSpecResponse, Status>) {
        match self.stream_tx.lock().unwrap().as_ref().unwrap().send(update).await {
            Err(err) => {
                log_e!("Failed to send update {}", err)
            }
            _ => {}
        }
    }

    pub async fn stop(&self) {
        let handle = self.server_handle.lock().unwrap().take();
        if let Some(handle) = handle {
            self.send_stream_update(Err(Status::unavailable("Connection Lost"))).await;
            self.shutdown_notifier.notify_one();
            wait_one_ms().await;

            let _ = handle.await;
        }
    }

    pub async fn restart(self: Arc<Self>) {
        self.stop().await;

        let mock_service = MockForwardProxyService {
            proxy: self.clone()
        };

        let shutdown_notify = self.shutdown_notifier.clone();
        let address = self.proxy_address.clone();

        let server_handle = tokio::spawn(async move {
            let _ = Server::builder()
                .add_service(StatsigForwardProxyServer::new(mock_service))
                .serve_with_shutdown(address, async {
                    shutdown_notify.notified().await;
                }).await;
        });

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        *self.stream_tx.lock().unwrap() = Some(tx);
        *self.stream_rx.lock().unwrap() = Some(rx);
        *self.server_handle.lock().unwrap() = Some(server_handle);

        wait_one_ms().await; // wait for the update to be applied
    }
}

struct MockForwardProxyService {
    pub proxy: Arc<MockForwardProxy>,
}

#[tonic::async_trait]
impl StatsigForwardProxy for MockForwardProxyService {
    async fn get_config_spec(
        &self,
        _request: Request<ConfigSpecRequest>,
    ) -> Result<Response<ConfigSpecResponse>, Status> {
        let response = self.proxy.stubbed_get_config_spec_response.lock().unwrap().clone();
        Ok(Response::new(response))
    }

    type StreamConfigSpecStream = ReceiverStream<Result<ConfigSpecResponse, Status>>;

    async fn stream_config_spec(
        &self,
        _request: Request<ConfigSpecRequest>,
    ) -> Result<Response<Self::StreamConfigSpecStream>, Status> {
        let rx = self.proxy.stream_rx.lock().unwrap().take().unwrap();

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(stream))
    }
}
