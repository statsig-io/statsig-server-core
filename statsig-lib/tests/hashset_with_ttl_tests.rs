use tokio::time::{Duration, sleep};
use sigstat::hashset_with_ttl::HashSetWithTTL;
use sigstat::StatsigRuntime;
use sigstat::output_logger::{initialize_simple_output_logger, LogLevel};

#[tokio::test]
async fn test_add_and_contains() {
    let statsig_runtime = StatsigRuntime::get_runtime();
    let hashset_with_ttl = HashSetWithTTL::new(&statsig_runtime, Duration::from_secs(10));

    hashset_with_ttl.add("test_key".to_string()).unwrap();
    assert!(hashset_with_ttl.contains("test_key").unwrap());
    assert!(!hashset_with_ttl.contains("non_existent_key").unwrap());
}

#[tokio::test]
async fn test_reset() {
    let statsig_runtime = StatsigRuntime::get_runtime();
    let hashset_with_ttl = HashSetWithTTL::new(&statsig_runtime, Duration::from_millis(10));

    hashset_with_ttl.add("test_key".to_string()).unwrap();
    assert!(hashset_with_ttl.contains("test_key").unwrap());

    // Wait for the TTL to expire and the set to be reset
    sleep(Duration::from_millis(100)).await;
    assert!(!hashset_with_ttl.contains("test_key").unwrap());
}

#[tokio::test]
async fn test_shutdown_stops_reset() {
    initialize_simple_output_logger(&Some(LogLevel::Debug));

    let statsig_runtime = StatsigRuntime::get_runtime();
    let hashset_with_ttl = HashSetWithTTL::new(&statsig_runtime, Duration::from_millis(10));

    hashset_with_ttl.add("test_key".to_string()).unwrap();
    assert!(hashset_with_ttl.contains("test_key").unwrap());

    hashset_with_ttl.shutdown().await;
    sleep(Duration::from_millis(100)).await;

    assert!(hashset_with_ttl.contains("test_key").unwrap());
}
