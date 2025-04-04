use statsig_rust::{Statsig, StatsigOptions, StatsigRuntime, StatsigUser};
use std::{
    env,
    sync::Arc,
    time::{Duration, Instant},
};

fn main() {
    let statsig_rt = StatsigRuntime::get_runtime();
    statsig_rt.runtime_handle.block_on(async {
        let sdk_key = env::var("test_api_key").expect("test_api_key environment variable not set");

        let options = Arc::new(StatsigOptions {
            enable_country_lookup: Some(true),
            enable_user_agent_parsing: Some(true),
            specs_sync_interval_ms: Some(1),
            disable_all_logging: Some(true),
            ..StatsigOptions::new()
        });

        let statsig =
            Statsig::new_shared(&sdk_key, Some(options)).expect("could not create perf statsig");

        statsig
            .initialize()
            .await
            .expect("could not initialize statsig");

        let spec_store_data = statsig
            .get_context()
            .spec_store_data
            .expect("spec_store_data is not set");

        let gates = spec_store_data.values.feature_gates;
        let gate_names = gates.keys().collect::<Vec<&String>>();

        for _ in 0..3 {
            profile_gate_checks(&gate_names).await;
            println!("--- Waiting ---");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        statsig
            .shutdown()
            .await
            .expect("could not shutdown statsig");
    });
}

async fn profile_gate_checks(gate_names: &[&String]) {
    let overall_start = Instant::now();

    let statsig = Statsig::shared();

    let user = StatsigUser {
        email: None,
        // user_agent: Some(dyn_value!("test_user_agent")),
        // ip: Some(dyn_value!("127.0.0.1")),
        country: None,
        locale: None,
        ..StatsigUser::with_user_id("test_user_id".to_string())
    };

    let mut gate_results = Vec::new();

    for gate_name in gate_names {
        let start = Instant::now();
        let mut gate_value = false;
        for _ in 0..1000 {
            gate_value = statsig.check_gate(&user, gate_name);
        }

        let duration = start.elapsed();
        gate_results.push((gate_name, gate_value, duration.as_micros()));
    }

    let overall_duration = overall_start.elapsed();
    gate_results.sort_by_key(|(_, _, duration)| std::cmp::Reverse(*duration));

    println!("--- Gate Results ({}ms) ---", overall_duration.as_millis());
    for (gate_name, gate_value, duration) in gate_results {
        println!(
            "{:40} {:10} {} ms",
            gate_name,
            gate_value,
            duration as f64 / 1000.0
        );
    }
}
