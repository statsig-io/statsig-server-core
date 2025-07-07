mod utils;

use std::{sync::Arc, time::Duration};

use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use tokio::time;
use utils::{
    helpers::load_contents,
    mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi},
};

// Context see this pr: https://github.com/statsig-io/private-statsig-server-core/pull/1006
// Test will hang if there is a deadlock happen

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_experiment() {
    let statsig = setup_and_init_statsig().await;
    let user = StatsigUser::with_user_id("a_user_id");

    let task = tokio::spawn(async move {
        for _ in 1..100000 {
            statsig.get_experiment(&user, "test_experiment_with_targeting");
        }
    });

    if let Err(_e) = time::timeout(Duration::from_secs(10), task).await {
        panic!("Timeout on get experiment, potential deadlock!!");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_check_gate() {
    let statsig = setup_and_init_statsig().await;
    let user = StatsigUser::with_user_id("a_user_id");

    let task = tokio::spawn(async move {
        for _ in 1..10000 {
            statsig.get_feature_gate(&user, "test_experiment_with_targeting");
        }
    });

    if let Err(_e) = time::timeout(Duration::from_secs(10), task).await {
        panic!("Timeout on get gate, potential deadlock");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_layer() {
    let statsig = setup_and_init_statsig().await;
    let user = StatsigUser::with_user_id("a_user_id");

    let task = tokio::spawn(async move {
        for _ in 1..10000 {
            statsig.get_layer(&user, "test_experiment_with_targeting");
        }
    });

    if let Err(_e) = time::timeout(Duration::from_secs(10), task).await {
        panic!("Timeout on get layer, potential deadlock");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_gcir() {
    let statsig = setup_and_init_statsig().await;
    let user = StatsigUser::with_user_id("a_user_id");

    let task = tokio::spawn(async move {
        for _ in 1..100 {
            statsig.get_client_init_response(&user);
        }
    });

    if let Err(_e) = time::timeout(Duration::from_secs(10), task).await {
        panic!("Timeout on gcir, potential deadlock");
    }
}

async fn setup_and_init_statsig() -> Statsig {
    let scrapi = setup_scrapi().await;
    let options = Arc::new(
        StatsigOptions::builder()
            .specs_url(Some(scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)))
            .specs_sync_interval_ms(Some(30))
            .build(),
    );
    let statsig = Statsig::new("secret", Some(options));
    statsig.initialize().await.unwrap();
    statsig
}

async fn setup_scrapi() -> MockScrapi {
    let mock_scrapi: MockScrapi = MockScrapi::new().await;
    let dcs = load_contents("eval_proj_dcs.json");

    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: dcs,
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    mock_scrapi
}
