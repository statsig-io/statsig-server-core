use criterion::{criterion_group, criterion_main, Criterion};
use mem_bench::noop_event_logging_adapter::NoopEventLoggingAdapter;
use mem_bench::static_specs_adapter::StaticSpecsAdapter;
use statsig_rust::user::user_data::UserData;
use statsig_rust::{dyn_value, Statsig, StatsigOptions, StatsigUser};
use std::collections::HashMap;
use std::sync::Arc;

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
      })
    }
}

async fn setup() -> (StatsigUser, Statsig) {
    let user = create_user();

    let mut options = StatsigOptions::new();
    options.wait_for_country_lookup_init = Some(true);
    options.wait_for_user_agent_init = Some(true);
    options.environment = Some("development".to_string());

    options.specs_adapter = Some(Arc::new(StaticSpecsAdapter::with_data("dcs_data.json")));
    options.event_logging_adapter = Some(Arc::new(NoopEventLoggingAdapter::default()));

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (user, statsig)
}

fn all_gate_checks() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let (user, statsig) = setup().await;

        let gate_names = statsig
            .get_context()
            .spec_store
            .data
            .try_read()
            .expect("Failed to lock spec store data")
            .values
            .feature_gates
            .keys()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        for gate_name in gate_names {
            for _ in 0..1000 {
                let _ = statsig.check_gate(&user, &gate_name);
            }
        }
    });
}

fn get_client_init_response() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let (user, statsig) = setup().await;

        for _ in 0..10 {
            let _ = statsig.get_client_init_response(&user);
        }

        let mut user_2 = user.clone();
        user_2.set_country("NZ");
        user_2.set_user_id("b_user");

        for _ in 0..10 {
            let _ = statsig.get_client_init_response(&user_2);
        }
    });
}

fn initialization() {
    for _ in 0..10 {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let _ = setup().await;
        });
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("all gates", |b| b.iter(|| all_gate_checks()));
    c.bench_function("get client init response", |b| {
        b.iter(|| get_client_init_response())
    });
    c.bench_function("statsig init", |b| b.iter(|| initialization()));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = criterion_benchmark
}

// criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
