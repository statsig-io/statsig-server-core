use std::env;
use std::sync::Arc;
use std::time::Instant;

use tokio::time::{sleep, Duration};

use statsig_rust::output_logger::LogLevel;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};

#[tokio::main]
async fn main() {
    let opts = Arc::new(StatsigOptions {
        output_log_level: Some(LogLevel::Debug),
        ..StatsigOptions::new()
    });

    let sdk_key = env::var("test_api_key").expect("test_api_key environment variable not set");

    let statsig = Statsig::new(&sdk_key, Some(opts));
    let _ = statsig.initialize().await;

    let start = Instant::now();
    for i in 0..5000 {
        let user = StatsigUser::with_user_id(format!("test_user_{}", i));
        let gate = statsig.check_gate(&user, "test_public");
        assert!(gate);
    }

    let duration = start.elapsed();
    assert!(duration.as_millis() < 1000);
    println!("check_gate: {:.2} ms", duration.as_millis());

    // let _ = statsig
    //     .shutdown_with_timeout(Duration::from_millis(100))
    //     .await;
    //
    // // Print active threads to help debug what's keeping the process alive
    // let thread_count = std::thread::available_parallelism().unwrap().get();
    // println!("\nActive threads: {}", thread_count);
    //
    // let active_threads = std::thread::scope(|_| {
    //     std::thread::current()
    //         .name()
    //         .unwrap_or("unnamed")
    //         .to_string()
    // });
    // println!("Current thread: {}", active_threads);

    let user = StatsigUser::with_user_id("a-user".to_string());
    loop {
        let gate = statsig.check_gate(&user, "test_public");
        println!("Gate {:?}", gate);

        let exp = statsig.get_experiment(&user, "experiment_with_many_params");
        println!(
            "Experiment {:?} {:?}",
            exp.rule_id,
            exp.value.get("a_string").unwrap().as_str()
        );

        sleep(Duration::from_secs(10)).await;
    }
}
