mod utils;

use crate::utils::mock_specs_adapter::MockSpecsAdapter;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi};

async fn setup() -> (Arc<Statsig>, StatsigUser, MockScrapi) {
    let mock_scrapi = MockScrapi::new().await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: "{}".to_string(),
            status: 500,
            ..EndpointStub::with_endpoint(Endpoint::LogEvent)
        })
        .await;

    let user = StatsigUser::with_user_id("a_user".to_string());
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(Arc::new(MockSpecsAdapter::with_data(
                "tests/data/eval_proj_dcs.json",
            ))),
            // output_log_level: Some(LogLevel::Debug),
            log_event_url: Some(mock_scrapi.url_for_endpoint(Endpoint::LogEvent)),
            environment: Some("development".to_string()),
            disable_country_lookup: Some(true),
            disable_user_agent_parsing: Some(true),
            event_logging_max_pending_batch_queue_size: Some(2),
            event_logging_max_queue_size: Some(1000),
            ..StatsigOptions::new()
        })),
    );

    statsig.initialize().await.unwrap();

    (Arc::new(statsig), user, mock_scrapi)
}

#[tokio::test]
async fn test_check_gate_overrides() {
    let (statsig, user, mock_scrapi) = setup().await;

    for _ in 0..10 {
        let statsig = statsig.clone();
        let user = user.clone();

        let (tx, rx) = mpsc::channel::<()>();
        let overall_start = Instant::now();
        std::thread::spawn(move || {
            for _ in 0..100_000 {
                let start = Instant::now();
                statsig.log_event(&user, "test_event", None, None);
                let duration = start.elapsed();
                if duration > Duration::from_millis(100) {
                    println!("Time taken: {duration:?}");
                }
            }
            tx.send(()).unwrap();
        });
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let overall_duration = overall_start.elapsed();
        println!("Overall time taken: {overall_duration:?}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let _ = statsig.shutdown().await;

    let count = mock_scrapi.get_logged_event_count();
    assert_eq!(count, 0);
}
