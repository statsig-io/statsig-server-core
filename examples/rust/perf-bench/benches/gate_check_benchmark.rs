use criterion::{criterion_group, criterion_main, Criterion};
use perf_bench::noop_event_logging_adapter::NoopEventLoggingAdapter;
use perf_bench::static_specs_adapter::StaticSpecsAdapter;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::collections::HashMap;
use std::sync::Arc;

async fn setup() -> (StatsigUser, Statsig) {
    let custom_ids: HashMap<String, String> =
        HashMap::from([("companyID".into(), "an_employee".into())]);

    let user = StatsigUser {
        user_id: Some("user-d".into()),
        country: Some("GB".into()),
        ..StatsigUser::with_custom_ids(custom_ids)
    };

    let mut options = StatsigOptions::new();
    options.enable_country_lookup = Some(true);
    options.enable_user_agent_parsing = Some(true);
    options.environment = Some("development".to_string());

    options.specs_adapter = Some(Arc::new(StaticSpecsAdapter::with_data("benches/data.json")));

    options.event_logging_adapter = Some(Arc::new(NoopEventLoggingAdapter::default()));

    let statsig = Statsig::new("secret-key", Some(Arc::new(options)));
    statsig.initialize().await.unwrap();

    (user, statsig)
}

fn test_all_gate_checks() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let (user, statsig) = setup().await;

        let values = statsig.get_current_values().unwrap().values.clone();

        for gate_name in values.feature_gates.keys() {
            for _ in 0..1000 {
                let _ = statsig.check_gate(&user, gate_name);
            }
        }
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("all gates", |b| b.iter(|| test_all_gate_checks()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
