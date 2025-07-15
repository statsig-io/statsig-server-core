use mem_bench::{
    noop_event_logging_adapter::NoopEventLoggingAdapter, static_specs_adapter::StaticSpecsAdapter,
};
use statsig_rust::{
    dyn_value, user::user_data::UserData, Statsig, StatsigOptions, StatsigRuntime, StatsigUser,
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    let statsig_rt = StatsigRuntime::get_runtime();
    statsig_rt.get_handle().block_on(async {
        let options = Arc::new(StatsigOptions {
            wait_for_country_lookup_init: Some(true),
            wait_for_user_agent_init: Some(true),
            specs_adapter: Some(Arc::new(StaticSpecsAdapter::with_data("dcs_data.json"))),
            event_logging_adapter: Some(Arc::new(NoopEventLoggingAdapter::default())),
            ..StatsigOptions::new()
        });

        let statsig = Statsig::new_shared("secret-key", Some(options))
            .expect("could not create perf statsig");

        statsig
            .initialize()
            .await
            .expect("could not initialize statsig");

        let gate_names = statsig
            .get_context()
            .spec_store
            .data
            .read()
            .expect("Failed to lock spec store data")
            .values
            .feature_gates
            .keys()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        for _ in 0..3 {
            profile_gate_checks(&gate_names).await;
            println!("--- Checked All Gates ---");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        statsig
            .shutdown()
            .await
            .expect("could not shutdown statsig");
    });
}

async fn profile_gate_checks(gate_names: &[String]) {
    let overall_start = Instant::now();

    let statsig = Statsig::shared();

    let user = create_user();

    let mut gate_results = Vec::new();

    for gate_name in gate_names {
        let start = Instant::now();
        let mut gate_value = false;
        for _ in 0..400 {
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

fn create_user() -> StatsigUser {
    StatsigUser {
        data: Arc::new(UserData {
            user_id: Some(dyn_value!("a_user")),
            email: Some(dyn_value!("daniel@statsig.com")),
            ip: Some(dyn_value!("127.0.0.1")),
            user_agent: Some("Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1".into()),
            country: Some(dyn_value!("US")),
            locale: Some(dyn_value!("en-US")),
            app_version: Some(dyn_value!("1.0.0")),
            custom_ids: Some(HashMap::from([
                ("companyID".into(), dyn_value!("statsig")),
                ("groupID".to_string(), dyn_value!("sdk_team"),
            )])),
            custom: Some(HashMap::from([(
                "test_custom_field".to_string(),
                dyn_value!("test_custom_field_value"),
            )])),
            private_attributes: Some(HashMap::from([(
                "test_private_attribute".to_string(),
                dyn_value!("test_private_attribute_value"),
            )])),
        }),
    }
}
