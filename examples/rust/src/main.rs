use std::sync::Arc;
use tokio::time::{sleep, Duration};
use sigstat::{LogLevel, Statsig, StatsigOptions, StatsigUser};
use std::env;

#[tokio::main]
async fn main() {
    let opts = Arc::new(StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        ..StatsigOptions::new()
    });

    let sdk_key = env::var("test_api_key").expect("test_api_key environment variable not set");

    let statsig = Statsig::new(sdk_key, Some(opts));
    let _ = statsig.initialize().await;
    let user = StatsigUser::with_user_id("a-user".to_string());
    loop {
        let exp = statsig.get_experiment(&user, "another_experiment");
        println!("Experiment {:?}", exp.group_name);
        sleep(Duration::from_secs(10)).await;
    }
}
