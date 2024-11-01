use std::sync::Arc;
use tokio::time::{sleep, Duration};
use sigstat::{ Statsig, StatsigOptions, StatsigUser};
use std::env;
use sigstat::output_logger::LogLevel;

#[tokio::main]
async fn main() {
    let opts = Arc::new(StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        ..StatsigOptions::new()
    });

    let sdk_key = env::var("test_api_key").expect("test_api_key environment variable not set");

    let statsig = Statsig::new(&sdk_key, Some(opts));
    let _ = statsig.initialize().await;
    let user = StatsigUser::with_user_id("a-user".to_string());
    loop {
        let gate = statsig.check_gate(&user, "test_public");
        println!("Gate {:?}", gate);
        let exp = statsig.get_experiment(&user, "experiment_with_many_params");
        println!("Experiment {:?} {:?}", exp.rule_id, exp.value.get("a_string").unwrap().string_value);
        sleep(Duration::from_secs(10)).await;
    }
}
