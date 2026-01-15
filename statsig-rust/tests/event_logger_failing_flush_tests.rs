mod utils;

use crate::utils::{
    mock_log_provider::{MockLogProvider, RecordedLog},
    mock_specs_adapter::MockSpecsAdapter,
};
use fancy_regex::Regex;
use parking_lot::Mutex;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::{sync::Arc, time::Duration};

fn get_dropped_count(provider: &Arc<MockLogProvider>) -> i64 {
    let logs = provider.logs.try_lock_for(Duration::from_secs(5)).unwrap();
    let found = logs.iter().rev().filter(|log| {
        if let RecordedLog::Warn(_, msg) = log {
            return msg.contains("Dropped");
        }

        false
    });

    let re = Regex::new(r"Dropped (\d+)\.").unwrap();
    let mut count = 0;
    for log in found {
        if let RecordedLog::Warn(_, msg) = log {
            if let Ok(Some(captures)) = re.captures(msg) {
                if let Some(matched) = captures.get(1) {
                    count += matched.as_str().parse::<i64>().unwrap();
                }
            }
        }
    }

    count
}

#[tokio::test]
async fn test_failing_flush() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/eval_proj_dcs.json"));
    let provider: Arc<MockLogProvider> = Arc::new(MockLogProvider {
        logs: Mutex::new(Vec::new()),
    });

    let options = StatsigOptions {
        log_event_url: Some("http://localhost".to_string()),
        specs_adapter: Some(specs_adapter),
        event_logging_max_queue_size: Some(100),
        event_logging_max_pending_batch_queue_size: Some(2),
        output_logger_provider: Some(provider.clone()),
        ..Default::default()
    };

    let statsig = Arc::new(Statsig::new("secret-key", Some(Arc::new(options))));
    statsig.initialize().await.unwrap();

    // fill the queue
    for i in 0..200 {
        let user = StatsigUser::with_user_id(format!("user-a-{i}"));
        let _ = statsig.check_gate(&user, "a-gate");
    }

    assert_eventually!(|| { get_dropped_count(&provider) == 1 });

    let statsig_clone = statsig.clone();
    std::thread::spawn(move || {
        for i in 0..300_000 {
            let user = StatsigUser::with_user_id(format!("user-b-{i}"));
            let _ = statsig_clone.check_gate(&user, "a-gate");
        }
    })
    .join()
    .unwrap();

    statsig.flush_events().await;

    assert_eventually!(|| {
        get_dropped_count(&provider) == 299901 /* 300K + 1 diagnostics - 200 */
    });
}
