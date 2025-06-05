use tokio::time::{sleep, Duration};

#[tokio::main]
pub async fn main() {
    println!("Legacy Benchmark");
    sleep(Duration::from_secs(1)).await;
}
