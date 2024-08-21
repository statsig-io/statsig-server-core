use std::sync::Arc;
use tokio::time::{sleep, Duration};
use statsig::{LogLevel, Statsig, StatsigOptions, StatsigUser};

#[tokio::main]
async fn main() {
    let opts = Arc::new(StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        ..StatsigOptions::new()
    });

    let statsig = Statsig::new("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW", Some(opts));
    let _ = statsig.initialize().await;
    let user = StatsigUser::with_user_id("a-user".to_string());
    loop {
        let exp = statsig.get_experiment(&user, "another_experiment");
        println!("Experiment {:?}", exp.group_name);
        sleep(Duration::from_secs(10)).await;
    }
}
