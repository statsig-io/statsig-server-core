use mem_bench::{
    noop_event_logging_adapter::NoopEventLoggingAdapter, static_specs_adapter::StaticSpecsAdapter,
};
use statsig_rust::{output_logger::LogLevel, Statsig, StatsigOptions, StatsigRuntime};
use std::{sync::Arc, time::Duration};

use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    let statsig_rt = StatsigRuntime::get_runtime();
    statsig_rt.runtime_handle.block_on(async {
        let specs_adapter = Arc::new(StaticSpecsAdapter::with_data("dcs_data.json"));
        let options = Arc::new(StatsigOptions {
            disable_user_agent_parsing: Some(true),
            disable_country_lookup: Some(true),
            specs_adapter: Some(specs_adapter.clone()),
            event_logging_adapter: Some(Arc::new(NoopEventLoggingAdapter::default())),
            output_log_level: Some(LogLevel::Debug),
            ..StatsigOptions::new()
        });

        let statsig = Statsig::new_shared("secret-key", Some(options))
            .expect("could not create perf statsig");

        statsig
            .initialize()
            .await
            .expect("could not initialize statsig");

        for _ in 0..10 {
            specs_adapter
                .manually_sync_specs(None)
                .await
                .expect("failed to sync specs");

            println!("--- Synced specs ---");

            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        statsig
            .shutdown()
            .await
            .expect("could not shutdown statsig");
    });
}
