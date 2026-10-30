mod utils;

use async_trait::async_trait;
use flate2::read::GzDecoder;
use parking_lot::Mutex;
use serde_json::{json, Value};
use statsig_rust::{
    dyn_value,
    evaluation::{
        evaluator::{Evaluator, SpecType},
        evaluator_context::{EvaluatorContext, IdListResolution},
    },
    experimental::{
        EnrichmentError, ForwardProxyEnrichmentOptions, ForwardProxyUserEnrichmentProvider,
        IDListEnrichmentProvider, IdListLookups, IdListMemberships, StatsigFPUserBuilder,
    },
    gcir::gcir_formatter::GCIRFormatter,
    hashing::HashUtil,
    networking::{NetworkClient, NetworkError, RequestArgs},
    specs_response::spec_types::SpecsResponseFull,
    user::StatsigUserInternal,
    ClientInitResponseOptions, HashAlgorithm, StatsigUser, StatsigUserBuilder,
};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::utils::helpers::load_contents;

fn provider(
    server: &MockServer,
    timeout_ms: u64,
    retries: u32,
) -> ForwardProxyUserEnrichmentProvider {
    ForwardProxyUserEnrichmentProvider::new(
        "secret-test",
        ForwardProxyEnrichmentOptions {
            url: server.uri(),
            timeout_ms: Some(timeout_ms),
            retries: Some(retries),
            proxy_config: None,
        },
    )
    .unwrap()
}

#[tokio::test]
async fn resolves_memberships_with_one_compressed_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/user_enrichment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id_lists": {"lookup-a": ["employees", "beta"]},
            "unsupported_enrichments": [],
            "future_field": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let memberships = provider(&server, 100, 0)
        .resolve_id_list_memberships(&vec!["lookup-a".into(), "lookup-b".into()])
        .await
        .unwrap();

    assert!(memberships["lookup-a"].contains("employees"));
    assert!(memberships["lookup-b"].is_empty());

    let requests = server.received_requests().await.unwrap();
    let request = &requests[0];
    assert_eq!(
        request.headers.get("statsig-api-key").unwrap(),
        "secret-test"
    );
    assert_eq!(request.headers.get("content-encoding").unwrap(), "gzip");

    let mut decoder = GzDecoder::new(request.body.as_slice());
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();
    let body: Value = serde_json::from_slice(&decompressed).unwrap();
    assert_eq!(body, json!({"id_list_lookups": ["lookup-a", "lookup-b"]}));
    assert!(!String::from_utf8(decompressed)
        .unwrap()
        .contains("raw-user"));
}

#[tokio::test]
async fn returns_http_errors_and_honors_retry_count() {
    for status in [408, 500, 503] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/user_enrichment"))
            .respond_with(ResponseTemplate::new(status))
            .expect(2)
            .mount(&server)
            .await;

        let error = provider(&server, 1000, 1)
            .resolve_id_list_memberships(&vec!["lookup".into()])
            .await
            .unwrap_err();
        assert_eq!(error, EnrichmentError::Http(status));
    }
}

#[tokio::test]
async fn non_retryable_http_errors_fail_fast() {
    for status in [400, 401, 403, 404, 405, 413, 422, 429, 501] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/user_enrichment"))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;

        let error = provider(&server, 1000, 3)
            .resolve_id_list_memberships(&vec!["lookup".into()])
            .await
            .unwrap_err();
        assert_eq!(error, EnrichmentError::Http(status));
    }
}

#[tokio::test]
async fn enrichment_retry_policy_does_not_change_other_clients() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401))
        .expect(2)
        .mount(&server)
        .await;

    let result = NetworkClient::new("secret-test", None, None)
        .post(
            RequestArgs {
                url: server.uri(),
                retries: 1,
                timeout_ms: 1000,
                ..RequestArgs::new()
            },
            None,
        )
        .await;
    assert!(matches!(
        result,
        Err(NetworkError::RetriesExhausted(_, Some(401), 2, _))
    ));
}

#[tokio::test]
async fn malformed_responses_are_decode_errors() {
    for body in [
        "not-json",
        "{}",
        r#"{"unrelated":true}"#,
        r#"{"id_lists":null}"#,
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/user_enrichment"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;

        let error = provider(&server, 1000, 3)
            .resolve_id_list_memberships(&vec!["lookup".into()])
            .await
            .unwrap_err();
        assert!(
            matches!(error, EnrichmentError::Decode(_)),
            "{body}: {error:?}"
        );
    }
}

#[tokio::test]
async fn explicit_empty_response_is_authoritative() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/user_enrichment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id_lists": {}})))
        .mount(&server)
        .await;

    let memberships = provider(&server, 1000, 0)
        .resolve_id_list_memberships(&vec!["lookup".into()])
        .await
        .unwrap();
    assert_eq!(
        memberships,
        HashMap::from([("lookup".into(), HashSet::new())])
    );
}

// Send successful headers, then either close early or stall mid-body. MockServer
// normally buffers a complete response, so these failures need a raw connection.
async fn incomplete_response_error(stall: bool) -> EnrichmentError {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            assert_ne!(stream.read_buf(&mut request).await.unwrap(), 0);
            if let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"id_lists\":")
            .await
            .unwrap();
        if stall {
            std::future::pending::<()>().await;
        }
    });
    let provider = ForwardProxyUserEnrichmentProvider::new(
        "secret-test",
        ForwardProxyEnrichmentOptions {
            url: format!("http://{address}"),
            timeout_ms: Some(200),
            retries: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        provider.resolve_id_list_memberships(&vec!["lookup".into()]),
    )
    .await;
    if stall {
        server.abort();
    } else {
        server.await.unwrap();
    }
    result.unwrap().unwrap_err()
}

#[tokio::test]
async fn truncated_response_body_is_a_transport_error() {
    assert!(matches!(
        incomplete_response_error(false).await,
        EnrichmentError::Transport(_)
    ));
}

#[tokio::test]
async fn stalled_response_body_is_a_timeout() {
    assert_eq!(
        incomplete_response_error(true).await,
        EnrichmentError::Timeout(Duration::from_millis(200))
    );
}

#[tokio::test]
async fn request_timeout_is_reported_without_fabricating_memberships() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/user_enrichment"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(50))
                .set_body_json(json!({"id_lists": {}})),
        )
        .mount(&server)
        .await;

    let error = provider(&server, 5, 0)
        .resolve_id_list_memberships(&vec!["lookup".into()])
        .await
        .unwrap_err();
    assert_eq!(error, EnrichmentError::Timeout(Duration::from_millis(5)));
}

#[tokio::test]
async fn timeout_text_in_url_does_not_change_transport_classification() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let provider = ForwardProxyUserEnrichmentProvider::new(
        "secret-test",
        ForwardProxyEnrichmentOptions {
            url: format!("http://{address}/timeout-proxy"),
            timeout_ms: Some(1000),
            retries: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        provider
            .resolve_id_list_memberships(&vec!["lookup".into()])
            .await,
        Err(EnrichmentError::Transport(_))
    ));
}

#[tokio::test]
async fn retries_truncated_body_and_recovers_within_attempt_budget() {
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    assert_ne!(stream.read_buf(&mut request).await.unwrap(), 0);
                    if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let len: usize = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse().unwrap())
                            })
                            .unwrap();
                        if request.len() >= end + 4 + len {
                            break;
                        }
                    }
                }
                let body = if attempt == 0 {
                    "{\"id_lists\":"
                } else {
                    "{\"id_lists\":{}}"
                };
                let len = if attempt == 0 { 100 } else { body.len() };
                stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}").as_bytes()).await.unwrap();
            }
        });
        let provider = ForwardProxyUserEnrichmentProvider::new(
            "secret-test",
            ForwardProxyEnrichmentOptions {
                url: format!("http://{address}"),
                timeout_ms: Some(1000),
                retries: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        let memberships = tokio::time::timeout(
            Duration::from_secs(5),
            provider.resolve_id_list_memberships(&vec!["lookup".into()]),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(memberships["lookup"].is_empty());
        server.await.unwrap();
    }
}

struct RecordingProvider {
    calls: AtomicUsize,
    lookups: Mutex<Vec<IdListLookups>>,
    result: Result<IdListMemberships, EnrichmentError>,
}

impl RecordingProvider {
    fn success(memberships: IdListMemberships) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            lookups: Mutex::new(Vec::new()),
            result: Ok(memberships),
        }
    }

    fn failure(error: EnrichmentError) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            lookups: Mutex::new(Vec::new()),
            result: Err(error),
        }
    }
}

#[async_trait]
impl IDListEnrichmentProvider for RecordingProvider {
    async fn resolve_id_list_memberships(
        &self,
        lookups: &IdListLookups,
    ) -> Result<IdListMemberships, EnrichmentError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.lookups.lock().push(lookups.clone());
        self.result.clone()
    }

    fn provider_name(&self) -> &'static str {
        "recording"
    }

    fn endpoint(&self) -> &str {
        "mock://enrichment"
    }
}

#[tokio::test]
async fn fp_builder_enriches_once_and_preserves_core_user_shape() {
    let memberships = HashMap::from([(
        "pmWkWSBC".to_string(),
        HashSet::from(["company_id_list".to_string()]),
    )]);
    let provider = Arc::new(RecordingProvider::success(memberships.clone()));

    let fp_user = StatsigFPUserBuilder::new_with_enricher(provider.clone())
        .user_id(Some("dan_1"))
        .custom_ids(Some([("companyID", "123")]))
        .email(Some("dan@example.com".to_string()))
        .custom(Some(
            [("plan".to_string(), dyn_value!("enterprise"))]
                .into_iter()
                .collect(),
        ))
        .build()
        .await
        .unwrap();

    assert_eq!(provider.calls.load(Ordering::Relaxed), 1);
    let sent_lookups = &provider.lookups.lock()[0];
    assert_eq!(sent_lookups.len(), 2);
    assert!(sent_lookups.contains(&"pmWkWSBC".to_string()));
    assert_eq!(fp_user.data.id_list_memberships, Some(memberships));

    let core_user = StatsigUserBuilder::new_with_user_id("dan_1")
        .custom_ids(Some([("companyID", "123")]))
        .email(Some("dan@example.com".to_string()))
        .custom(Some(
            [("plan".to_string(), dyn_value!("enterprise"))]
                .into_iter()
                .collect(),
        ))
        .build();
    assert_eq!(
        serde_json::to_value(fp_user.data.as_ref()).unwrap(),
        serde_json::to_value(core_user.data.as_ref()).unwrap()
    );

    fn accepts_core_user(_: &StatsigUser) {}
    accepts_core_user(&fp_user);
}

#[tokio::test]
async fn fp_builder_returns_enrichment_error_on_failure() {
    let provider = Arc::new(RecordingProvider::failure(EnrichmentError::Http(503)));
    let error = match StatsigFPUserBuilder::new_with_enricher(provider.clone())
        .user_id(Some("user"))
        .build()
        .await
    {
        Ok(_) => panic!("expected enrichment error"),
        Err(error) => error,
    };

    assert_eq!(provider.calls.load(Ordering::Relaxed), 1);
    assert_eq!(error, EnrichmentError::Http(503));
}

#[tokio::test]
async fn successful_empty_enrichment_is_authoritative() {
    let provider = Arc::new(RecordingProvider::success(HashMap::new()));
    let fp_user = StatsigFPUserBuilder::new_with_enricher(provider)
        .user_id(Some("user"))
        .build()
        .await
        .unwrap();

    assert_eq!(fp_user.data.id_list_memberships, Some(HashMap::new()));
}

#[tokio::test]
async fn fp_enriched_user_matches_stock_id_list_evaluation_and_gcir() {
    let specs: SpecsResponseFull =
        serde_json::from_str(&load_contents("eval_proj_dcs.json")).unwrap();
    let memberships = HashMap::from([(
        "pmWkWSBC".to_string(),
        HashSet::from(["company_id_list".to_string()]),
    )]);
    let fp_user =
        StatsigFPUserBuilder::new_with_enricher(Arc::new(RecordingProvider::success(memberships)))
            .user_id(Some("dan_1"))
            .custom_ids(Some([("companyID", "123")]))
            .build()
            .await
            .unwrap();
    let mut stock_user = StatsigUser::with_user_id("dan_1");
    stock_user.set_custom_ids(HashMap::from([("companyID", "123")]));

    let stock_callback =
        |list: &str, lookup: &str| list == "company_id_list" && lookup == "pmWkWSBC";
    let empty_lists = HashMap::new();

    assert_eq!(
        evaluate_gate(&specs, &fp_user, IdListResolution::MapLookup(&empty_lists)),
        evaluate_gate(
            &specs,
            &stock_user,
            IdListResolution::Callback(&stock_callback)
        )
    );

    let enriched_gcir = generate_gcir(&specs, &fp_user, IdListResolution::MapLookup(&empty_lists));
    let stock_gcir = generate_gcir(
        &specs,
        &stock_user,
        IdListResolution::Callback(&stock_callback),
    );
    assert_eq!(
        enriched_gcir.feature_gates["test_id_list"].value,
        stock_gcir.feature_gates["test_id_list"].value
    );
}

fn evaluate_gate(
    specs: &SpecsResponseFull,
    user: &StatsigUser,
    resolver: IdListResolution<'_>,
) -> bool {
    let internal = StatsigUserInternal {
        user_ref: user,
        statsig_instance: None,
    };
    let hashing = HashUtil::new();
    let mut context = EvaluatorContext::new(
        &internal, specs, resolver, &hashing, None, None, false, None, true,
    );
    Evaluator::evaluate(&mut context, "test_id_list", &SpecType::Gate).unwrap();
    context.result.bool_value
}

fn generate_gcir(
    specs: &SpecsResponseFull,
    user: &StatsigUser,
    resolver: IdListResolution<'_>,
) -> statsig_rust::InitializeResponse {
    let internal = StatsigUserInternal {
        user_ref: user,
        statsig_instance: None,
    };
    let hashing = HashUtil::new();
    let mut context = EvaluatorContext::new(
        &internal, specs, resolver, &hashing, None, None, false, None, true,
    );
    GCIRFormatter::generate_v1_format(
        &mut context,
        &ClientInitResponseOptions {
            hash_algorithm: Some(HashAlgorithm::None),
            ..Default::default()
        },
    )
    .unwrap()
}

#[tokio::test]
async fn binding_seed_preserves_numeric_and_string_ids() {
    let enricher = Arc::new(RecordingProvider {
        calls: AtomicUsize::new(0),
        lookups: Mutex::new(Vec::new()),
        result: Ok(HashMap::new()),
    });
    for user_id in [json!(123), json!(12.5), json!("00123")] {
        let data: statsig_rust::StatsigUserData = serde_json::from_value(json!({
            "userID": user_id, "customIDs": {"number": 123, "float": 12.5, "string": "00123"}
        }))
        .unwrap();
        let expected = serde_json::to_value(&data).unwrap();
        let user = StatsigFPUserBuilder::from_user_data(enricher.clone(), data)
            .build()
            .await
            .unwrap();
        assert_eq!(serde_json::to_value(&*user.data).unwrap(), expected);
    }
}
