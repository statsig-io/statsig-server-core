use std::{
    fmt::{Display, Formatter},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use wiremock::{
    http::Method as WiremockMethod,
    matchers::{method, path, path_regex},
    Mock, MockBuilder, MockServer, Request, ResponseTemplate,
};

#[allow(clippy::upper_case_acronyms)]
pub enum Method {
    GET,
    POST,
}

impl From<Method> for WiremockMethod {
    fn from(val: Method) -> Self {
        match val {
            Method::GET => WiremockMethod::GET,
            Method::POST => WiremockMethod::POST,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum Endpoint {
    LogEvent,
    DownloadConfigSpecs,
    GetIdLists,
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Endpoint::LogEvent => write!(f, "/v1/log_event"),
            Endpoint::DownloadConfigSpecs => write!(f, "/v2/download_config_specs"),
            Endpoint::GetIdLists => write!(f, "/v1/get_id_lists"),
        }
    }
}

pub struct EndpointStub {
    pub endpoint: Endpoint,
    pub response: String,
    pub status: u16,
    pub delay_ms: u64,
    pub method: Method,
}

impl EndpointStub {
    pub fn with_endpoint(endpoint: Endpoint) -> EndpointStub {
        EndpointStub {
            endpoint,
            response: "".to_string(),
            status: 200,
            delay_ms: 0,
            method: Method::GET,
        }
    }
}

pub struct MockScrapi {
    mock_server: MockServer,
    requests: Arc<Mutex<Vec<Request>>>,
    logged_events: Arc<AtomicU64>,
}

impl MockScrapi {
    pub async fn new() -> MockScrapi {
        let mock_server = MockServer::start().await;

        MockScrapi {
            mock_server,
            requests: Arc::new(Mutex::new(Vec::new())),
            logged_events: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn reset(&self) {
        self.mock_server.reset().await;
    }

    pub async fn stub(&self, stub: EndpointStub) {
        let logged_events = self.logged_events.clone();
        let reqs = self.requests.clone();

        let mut builder = Mock::given(method(stub.method));
        builder = set_endpoint_matcher(builder, &stub.endpoint);

        builder
            .respond_with(move |req: &Request| {
                let response_template = ResponseTemplate::new(stub.status)
                    .set_body_string(stub.response.clone())
                    .set_delay(Duration::from_millis(stub.delay_ms));

                reqs.lock().unwrap().push(req.clone());

                if req.url.as_str().contains("/v1/log_event") {
                    let count = req.headers["statsig-event-count"]
                        .to_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap();

                    let local_logged_events_ptr = logged_events.clone();
                    tokio::task::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(stub.delay_ms)).await;
                        local_logged_events_ptr.fetch_add(count, Ordering::SeqCst);
                    });
                }

                response_template
            })
            .mount(&self.mock_server)
            .await;
    }

    pub fn url_for_endpoint(&self, endpoint: Endpoint) -> String {
        format!("{}{}", self.mock_server.uri(), endpoint)
    }

    pub fn times_called_for_endpoint(&self, endpoint: Endpoint) -> u32 {
        let requests = self.requests.lock().unwrap();

        let filtered_requests: Vec<_> = requests
            .iter()
            .filter(|req| req.url.as_str().contains(&endpoint.to_string()))
            .collect();

        filtered_requests.len() as u32
    }

    pub fn get_logged_event_count(&self) -> u64 {
        self.logged_events.load(Ordering::SeqCst)
    }

    pub fn get_requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    pub fn get_requests_for_endpoint(&self, endpoint: Endpoint) -> Vec<Request> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|req| req.url.as_str().contains(&endpoint.to_string()))
            .cloned()
            .collect()
    }
}

fn set_endpoint_matcher(builder: MockBuilder, endpoint: &Endpoint) -> MockBuilder {
    match endpoint {
        Endpoint::DownloadConfigSpecs => builder.and(path_regex("^/v2/download_config_specs")),
        _ => builder.and(path(endpoint.to_string())),
    }
}
