use std::time::Duration;

pub async fn wait_one_ms() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}