use statsig::{Statsig, StatsigUser};
use tokio;
use std::time::Instant;


#[tokio::main]
async fn main() {
    Statsig::initialize("secret-9IWfdzNwExEYHEW4YfOQcFZ4xreZyFkbOXHaNbPsMwW").await;

    let start = Instant::now();
    let mut init_res = String::new();
    for _ in 0..1000 {
      let user = StatsigUser::with_user_id("Dan".to_string());
        init_res = match Statsig::get_client_initialize_response(&user) {
            Ok(s) => s.to_string(),
            _ => "{}".to_string()
        }
    }
    println!("{}", init_res);

    let duration = start.elapsed();
    println!("Time taken: {:.2} ms", duration.as_secs_f64() * 1000.0);
}
