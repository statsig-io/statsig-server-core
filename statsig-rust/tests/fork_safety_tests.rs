mod utils;

use serde_json::Value;
use serial_test::serial;
use statsig_rust::statsig_global::StatsigGlobal;
use statsig_rust::{
    ObservabilityClient, SpecsUpdateListener, Statsig, StatsigOptions, StatsigUser,
};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Barrier, Weak};
use std::thread;
use std::time::{Duration, Instant};
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;
use utils::mock_observability_client::MockObservabilityClient;
use utils::mock_scrapi::{Endpoint, EndpointStub, Method, MockScrapi, StubData};
use utils::mock_specs_adapter::MockSpecsAdapter;

fn setup() -> (Arc<Statsig>, Arc<MockSpecsAdapter>, StatsigUser) {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/demo_proj_dcs.json"));
    setup_with_adapter(specs_adapter)
}

fn setup_with_adapter(
    specs_adapter: Arc<MockSpecsAdapter>,
) -> (Arc<Statsig>, Arc<MockSpecsAdapter>, StatsigUser) {
    let statsig = Arc::new(Statsig::new(
        "secret-fork-safety-test",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter.clone()),
            event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    ));

    statsig
        .statsig_runtime
        .get_handle()
        .expect("runtime must be available")
        .block_on(statsig.initialize())
        .expect("Statsig must initialize");
    assert_eq!(specs_adapter.background_sync_calls(), 1);

    (
        statsig,
        specs_adapter,
        StatsigUser::with_user_id("fork-safety-user"),
    )
}

fn wait_for_sync_calls(adapter: &MockSpecsAdapter, expected: usize) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while adapter.background_sync_calls() < expected && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    adapter.background_sync_calls() >= expected
}

fn wait_for_post_fork_sync_calls(adapter: &MockSpecsAdapter, expected: usize) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while adapter.post_fork_background_sync_calls() < expected && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    adapter.post_fork_background_sync_calls() >= expected
}

fn persistent_runtime_tasks(statsig: &Statsig) -> Vec<(String, String)> {
    statsig
        .statsig_runtime
        .get_running_task_ids()
        .into_iter()
        .filter(|(name, _)| name == "EVT_LOG_BG_LOOP" || name == "opts_stats_listen_for")
        .collect()
}

fn persistent_runtime_task_names(statsig: &Statsig) -> Vec<String> {
    let mut names = persistent_runtime_tasks(statsig)
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn wait_for_new_persistent_runtime_tasks(
    statsig: &Statsig,
    task_ids_before_reset: &HashSet<String>,
    expected_names: &[String],
) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if new_persistent_runtime_task_names(statsig, task_ids_before_reset) == expected_names {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    new_persistent_runtime_task_names(statsig, task_ids_before_reset) == expected_names
}

fn new_persistent_runtime_task_names(
    statsig: &Statsig,
    task_ids_before_reset: &HashSet<String>,
) -> Vec<String> {
    let mut names = persistent_runtime_tasks(statsig)
        .into_iter()
        .filter(|(_, id)| !task_ids_before_reset.contains(id))
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn shutdown(statsig: &Statsig) {
    statsig
        .statsig_runtime
        .get_handle()
        .expect("runtime must be available")
        .block_on(statsig.shutdown())
        .expect("Statsig must shut down");
}

#[test]
#[serial]
fn check_gate_rearms_shared_background_tasks_after_runtime_reset() {
    let (statsig, specs_adapter, user) = setup();

    StatsigGlobal::reset();
    let _ = statsig.check_gate(&user, "test_public");

    assert!(
        wait_for_sync_calls(&specs_adapter, 2),
        "check_gate did not re-arm shared background tasks"
    );
    assert_eq!(specs_adapter.post_fork_background_sync_calls(), 0);
    shutdown(&statsig);
}

#[test]
#[serial]
fn log_event_rearms_shared_background_tasks_after_runtime_reset() {
    let (statsig, specs_adapter, user) = setup();

    StatsigGlobal::reset();
    statsig.log_event(&user, "fork-recovery-event", None, None);

    assert!(
        wait_for_sync_calls(&specs_adapter, 2),
        "log_event did not re-arm shared background tasks"
    );
    shutdown(&statsig);
}

#[test]
#[serial]
fn runtime_reset_during_recovery_does_not_permanently_latch_restart() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_blocked_background_sync(
        "tests/data/demo_proj_dcs.json",
        2,
    ));
    let (statsig, specs_adapter, user) = setup_with_adapter(specs_adapter);

    StatsigGlobal::reset();
    let _ = statsig.check_gate(&user, "test_public");
    assert!(
        wait_for_sync_calls(&specs_adapter, 2),
        "the first recovery did not reach the blocking schedule call"
    );

    // Cancels the in-flight recovery future before it can clear its restart latch.
    StatsigGlobal::reset();
    let _ = statsig.check_gate(&user, "test_public");

    assert!(
        wait_for_sync_calls(&specs_adapter, 3),
        "a cancelled recovery permanently latched background task restart"
    );
    specs_adapter.release_background_sync();
    shutdown(&statsig);
}

#[cfg(unix)]
#[test]
#[serial]
fn forked_child_replaces_an_inherited_in_flight_restart_claim() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_blocked_background_sync(
        "tests/data/demo_proj_dcs.json",
        2,
    ));
    let (statsig, specs_adapter, user) = setup_with_adapter(specs_adapter);

    StatsigGlobal::reset();
    let _ = statsig.check_gate(&user, "test_public");
    assert!(
        wait_for_sync_calls(&specs_adapter, 2),
        "the parent recovery did not reach the blocking schedule call"
    );

    let child_pid = unsafe { libc::fork() };
    assert!(child_pid >= 0, "fork failed");

    if child_pid == 0 {
        let _ = statsig.check_gate(&user, "test_public");
        let exit_code = if wait_for_post_fork_sync_calls(&specs_adapter, 1) {
            0
        } else {
            1
        };
        unsafe { libc::_exit(exit_code) };
    }

    let status = wait_for_child(child_pid, Duration::from_secs(5));
    assert!(libc::WIFEXITED(status), "child did not exit normally");
    assert_eq!(
        libc::WEXITSTATUS(status),
        0,
        "child remained latched to the parent's in-flight restart"
    );

    specs_adapter.release_background_sync();
    shutdown(&statsig);
}

#[test]
#[serial]
fn explicit_postfork_restart_schedules_tasks_without_an_evaluation() {
    let (statsig, specs_adapter, _) = setup();
    let persistent_tasks_before_reset = persistent_runtime_task_names(&statsig);
    let task_ids_before_reset = persistent_runtime_tasks(&statsig)
        .into_iter()
        .map(|(_, id)| id)
        .collect::<HashSet<_>>();
    assert_eq!(
        persistent_tasks_before_reset,
        vec![
            "EVT_LOG_BG_LOOP".to_string(),
            "opts_stats_listen_for".to_string(),
            "opts_stats_listen_for".to_string(),
            "opts_stats_listen_for".to_string(),
        ]
    );

    StatsigGlobal::reset();
    assert!(statsig.restart_background_tasks_after_fork());

    assert_eq!(specs_adapter.background_sync_calls(), 1);
    assert_eq!(specs_adapter.post_fork_background_sync_calls(), 1);
    assert!(
        wait_for_new_persistent_runtime_tasks(
            &statsig,
            &task_ids_before_reset,
            &persistent_tasks_before_reset,
        ),
        "post-fork restart did not restore the event logger and OpsStats listeners; observed {:?}",
        new_persistent_runtime_task_names(&statsig, &task_ids_before_reset),
    );
    shutdown(&statsig);
}

#[test]
#[serial]
fn failed_explicit_postfork_restart_is_retried_lazily() {
    let (statsig, specs_adapter, user) = setup();
    let expected_persistent_tasks = persistent_runtime_task_names(&statsig);
    let task_ids_before_reset = persistent_runtime_tasks(&statsig)
        .into_iter()
        .map(|(_, id)| id)
        .collect::<HashSet<_>>();
    specs_adapter.fail_next_post_fork_background_sync();

    StatsigGlobal::reset();
    assert!(!statsig.restart_background_tasks_after_fork());
    assert_eq!(specs_adapter.post_fork_background_sync_calls(), 1);
    assert!(
        new_persistent_runtime_task_names(&statsig, &task_ids_before_reset).is_empty(),
        "failed adapter recovery must not leave partial event or observability loops"
    );

    let _ = statsig.check_gate(&user, "test_public");
    assert!(
        wait_for_new_persistent_runtime_tasks(
            &statsig,
            &task_ids_before_reset,
            &expected_persistent_tasks,
        ),
        "a lazy operation must retry a failed explicit recovery"
    );
    assert_eq!(specs_adapter.background_sync_calls(), 2);
    assert_eq!(specs_adapter.post_fork_background_sync_calls(), 1);
    shutdown(&statsig);
}

#[test]
#[serial]
fn postfork_restart_restores_external_observability_without_reinitializing_it() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/demo_proj_dcs.json"));
    let observability_client = Arc::new(MockObservabilityClient::new());
    let weak_observability_client =
        Arc::downgrade(&observability_client) as Weak<dyn ObservabilityClient>;
    let statsig = Statsig::new(
        "secret-fork-observability-test",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter),
            event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
            observability_client: Some(weak_observability_client),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );

    statsig
        .statsig_runtime
        .get_handle()
        .expect("runtime must be available")
        .block_on(statsig.initialize())
        .expect("Statsig must initialize");
    assert_eq!(
        observability_client
            .init_called_times
            .load(Ordering::SeqCst),
        1
    );
    let task_ids_before_reset = persistent_runtime_tasks(&statsig)
        .into_iter()
        .map(|(_, id)| id)
        .collect::<HashSet<_>>();

    StatsigGlobal::reset();
    assert!(statsig.restart_background_tasks_after_fork());

    let expected_tasks = vec![
        "EVT_LOG_BG_LOOP".to_string(),
        "opts_stats_listen_for".to_string(),
        "opts_stats_listen_for".to_string(),
        "opts_stats_listen_for".to_string(),
        "opts_stats_listen_for".to_string(),
    ];
    assert!(
        wait_for_new_persistent_runtime_tasks(&statsig, &task_ids_before_reset, &expected_tasks),
        "post-fork restart did not restore the external observability listener; observed {:?}",
        new_persistent_runtime_task_names(&statsig, &task_ids_before_reset),
    );
    assert_eq!(
        observability_client
            .init_called_times
            .load(Ordering::SeqCst),
        1,
        "fork recovery must resubscribe, not reinitialize, the observability client"
    );
    shutdown(&statsig);
}

#[test]
#[serial]
fn repeated_explicit_and_lazy_recovery_schedule_one_postfork_sync() {
    let (statsig, specs_adapter, user) = setup();

    StatsigGlobal::reset();
    assert!(statsig.restart_background_tasks_after_fork());

    let barrier = Arc::new(Barrier::new(9));
    thread::scope(|scope| {
        for _ in 0..8 {
            let statsig = statsig.clone();
            let user = user.clone();
            let barrier = barrier.clone();
            scope.spawn(move || {
                barrier.wait();
                assert!(statsig.restart_background_tasks_after_fork());
                let _ = statsig.check_gate(&user, "test_public");
            });
        }
        barrier.wait();
    });

    assert_eq!(specs_adapter.background_sync_calls(), 1);
    assert_eq!(specs_adapter.post_fork_background_sync_calls(), 1);
    shutdown(&statsig);
}

#[test]
#[serial]
fn concurrent_lazy_recovery_schedules_one_background_sync() {
    let (statsig, specs_adapter, user) = setup();

    StatsigGlobal::reset();
    let barrier = Arc::new(Barrier::new(17));
    thread::scope(|scope| {
        for _ in 0..16 {
            let statsig = statsig.clone();
            let user = user.clone();
            let barrier = barrier.clone();
            scope.spawn(move || {
                barrier.wait();
                let _ = statsig.check_gate(&user, "test_public");
            });
        }
        barrier.wait();
    });

    assert!(wait_for_sync_calls(&specs_adapter, 2));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(specs_adapter.background_sync_calls(), 2);
    assert_eq!(specs_adapter.post_fork_background_sync_calls(), 0);
    shutdown(&statsig);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn explicit_postfork_restart_refreshes_http_specs_immediately() {
    // Ensure reset advances the runtime epoch even though this test supplies
    // its own Tokio runtime.
    drop(StatsigGlobal::get());

    let mock_scrapi = MockScrapi::new().await;
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data/demo_proj_dcs.json");
    let dcs = fs::read_to_string(path).expect("Unable to read DCS fixture");
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(dcs.clone()),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    let statsig = Statsig::new(
        "secret-fork-safety-http-test",
        Some(Arc::new(StatsigOptions {
            specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            specs_sync_interval_ms: Some(60_000),
            event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );
    statsig.initialize().await.expect("Statsig must initialize");
    let user = StatsigUser::with_user_id("postfork-http-user");
    assert!(statsig.check_gate(&user, "test_public"));

    let initial_lcut = statsig
        .get_context()
        .spec_store
        .get_current_specs_info()
        .lcut
        .expect("initial DCS must provide an LCUT");
    let updated_lcut = initial_lcut + 1;
    let mut updated_dcs: Value = serde_json::from_str(&dcs).expect("DCS fixture must be JSON");
    updated_dcs["time"] = Value::from(updated_lcut);
    updated_dcs["checksum"] = Value::from("postfork-refresh-checksum");
    updated_dcs["feature_gates"]["test_public"]["enabled"] = Value::from(false);
    updated_dcs["feature_gates"]["test_public"]["rules"] = Value::Array(Vec::new());

    mock_scrapi.clear_stubs().await;
    mock_scrapi.clear_requests();
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(
                serde_json::to_string(&updated_dcs).expect("updated DCS must serialize"),
            ),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;

    tokio::task::spawn_blocking(StatsigGlobal::reset)
        .await
        .expect("runtime reset must complete");
    assert!(statsig.restart_background_tasks_after_fork());

    tokio::time::timeout(Duration::from_secs(1), async {
        while mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs) == 0
            || statsig
                .get_context()
                .spec_store
                .get_current_specs_info()
                .lcut
                != Some(updated_lcut)
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("post-fork refresh did not request and apply the new DCS before the polling interval");
    assert!(!statsig.check_gate(&user, "test_public"));

    statsig.shutdown().await.expect("Statsig must shut down");
}

#[test]
#[serial]
fn every_evaluation_family_rearms_background_tasks() {
    let (statsig, specs_adapter, user) = setup();
    let mut expected_sync_calls = 2;

    StatsigGlobal::reset();
    let _ = statsig.get_dynamic_config(&user, "missing-dynamic-config");
    assert!(wait_for_sync_calls(&specs_adapter, expected_sync_calls));

    expected_sync_calls += 1;
    StatsigGlobal::reset();
    let _ = statsig.get_experiment(&user, "missing-experiment");
    assert!(wait_for_sync_calls(&specs_adapter, expected_sync_calls));

    expected_sync_calls += 1;
    StatsigGlobal::reset();
    let _ = statsig.get_layer(&user, "missing-layer");
    assert!(wait_for_sync_calls(&specs_adapter, expected_sync_calls));

    expected_sync_calls += 1;
    StatsigGlobal::reset();
    let _ = statsig.get_parameter_store("missing-parameter-store");
    assert!(wait_for_sync_calls(&specs_adapter, expected_sync_calls));

    expected_sync_calls += 1;
    StatsigGlobal::reset();
    let _ = statsig.get_cmab_ranked_groups(&user, "missing-cmab");
    assert!(wait_for_sync_calls(&specs_adapter, expected_sync_calls));

    expected_sync_calls += 1;
    StatsigGlobal::reset();
    let _ = statsig.get_experiment_groups("missing-experiment-groups");
    assert!(wait_for_sync_calls(&specs_adapter, expected_sync_calls));
    shutdown(&statsig);
}

#[test]
#[serial]
fn explicit_shutdown_prevents_runtime_reset_from_rearming_tasks() {
    let (statsig, specs_adapter, user) = setup();

    shutdown(&statsig);
    StatsigGlobal::reset();
    let _ = statsig.check_gate(&user, "test_public");
    std::thread::sleep(Duration::from_millis(200));

    assert_eq!(specs_adapter.background_sync_calls(), 1);
}

#[test]
#[serial]
fn initialize_after_shutdown_restarts_shared_background_tasks() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/demo_proj_dcs.json"));
    let event_adapter = Arc::new(MockEventLoggingAdapter::new());
    let statsig = Statsig::new(
        "secret-reinitialize-background-tasks-test",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter.clone()),
            event_logging_adapter: Some(event_adapter.clone()),
            event_logging_max_queue_size: Some(10),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );
    statsig
        .statsig_runtime
        .get_handle()
        .expect("runtime must be available")
        .block_on(statsig.initialize())
        .expect("Statsig must initialize");
    let expected_persistent_tasks = persistent_runtime_task_names(&statsig);

    shutdown(&statsig);
    statsig
        .statsig_runtime
        .get_handle()
        .expect("runtime must be available")
        .block_on(statsig.initialize())
        .expect("Statsig must reinitialize");

    assert_eq!(specs_adapter.background_sync_calls(), 2);
    assert!(
        wait_for_new_persistent_runtime_tasks(
            &statsig,
            &HashSet::new(),
            &expected_persistent_tasks
        ),
        "reinitialize did not restore exactly one copy of each persistent task"
    );

    let logged_before = event_adapter
        .no_diagnostics_logged_event_count
        .load(Ordering::SeqCst);
    let user = StatsigUser::with_user_id("reinitialize-background-tasks-user");
    for index in 0..10 {
        statsig.log_event(
            &user,
            &format!("reinitialize-background-task-event-{index}"),
            None,
            None,
        );
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    while event_adapter
        .no_diagnostics_logged_event_count
        .load(Ordering::SeqCst)
        == logged_before
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        event_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst)
            > logged_before,
        "reinitialized event loop did not flush a full batch"
    );
    shutdown(&statsig);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn reinitialize_restores_default_http_background_loops() {
    drop(StatsigGlobal::get());

    let mock_scrapi = MockScrapi::new().await;
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data/demo_proj_dcs.json");
    let dcs = fs::read_to_string(path).expect("Unable to read DCS fixture");
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(dcs.clone()),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String("{}".to_string()),
            ..EndpointStub::with_endpoint(Endpoint::GetIdLists)
        })
        .await;

    let statsig = Statsig::new(
        "secret-reinitialize-http-background-test",
        Some(Arc::new(StatsigOptions {
            specs_url: Some(mock_scrapi.url_for_endpoint(Endpoint::DownloadConfigSpecs)),
            id_lists_url: Some(mock_scrapi.url_for_endpoint(Endpoint::GetIdLists)),
            enable_id_lists: Some(true),
            specs_sync_interval_ms: Some(1_000),
            id_lists_sync_interval_ms: Some(1_000),
            event_logging_adapter: Some(Arc::new(MockEventLoggingAdapter::new())),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );
    statsig.initialize().await.expect("Statsig must initialize");
    statsig.shutdown().await.expect("Statsig must shut down");
    statsig
        .initialize()
        .await
        .expect("Statsig must reinitialize");

    let initial_lcut = statsig
        .get_context()
        .spec_store
        .get_current_specs_info()
        .lcut
        .expect("initial DCS must provide an LCUT");
    let updated_lcut = initial_lcut + 1;
    let mut updated_dcs: Value = serde_json::from_str(&dcs).expect("DCS fixture must be JSON");
    updated_dcs["time"] = Value::from(updated_lcut);
    updated_dcs["checksum"] = Value::from("reinitialize-http-background-checksum");
    updated_dcs["feature_gates"]["test_public"]["enabled"] = Value::from(false);
    updated_dcs["feature_gates"]["test_public"]["rules"] = Value::Array(Vec::new());

    mock_scrapi.clear_stubs().await;
    mock_scrapi.clear_requests();
    mock_scrapi
        .stub(EndpointStub {
            method: Method::GET,
            response: StubData::String(
                serde_json::to_string(&updated_dcs).expect("updated DCS must serialize"),
            ),
            ..EndpointStub::with_endpoint(Endpoint::DownloadConfigSpecs)
        })
        .await;
    mock_scrapi
        .stub(EndpointStub {
            method: Method::POST,
            response: StubData::String("{}".to_string()),
            ..EndpointStub::with_endpoint(Endpoint::GetIdLists)
        })
        .await;

    let recovery = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let task_names = statsig
                .statsig_runtime
                .get_running_task_ids()
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>();
            let adapters_are_running = task_names
                .iter()
                .filter(|name| name.as_str() == "http_specs_bg_sync")
                .count()
                == 1
                && task_names
                    .iter()
                    .filter(|name| name.as_str() == "http_id_list_bg_sync")
                    .count()
                    == 1;
            let updated_specs_installed = statsig
                .get_context()
                .spec_store
                .get_current_specs_info()
                .lcut
                == Some(updated_lcut);
            if adapters_are_running && updated_specs_installed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    assert!(
        recovery.is_ok(),
        "reinitialized default HTTP adapters did not remain alive and install new DCS; tasks={:?}, requests={}, lcut={:?}",
        statsig.statsig_runtime.get_running_task_ids(),
        mock_scrapi.times_called_for_endpoint(Endpoint::DownloadConfigSpecs),
        statsig.get_context().spec_store.get_current_specs_info().lcut,
    );
    statsig.shutdown().await.expect("Statsig must shut down");
}

#[cfg(unix)]
#[test]
#[serial]
fn forked_child_rebuilds_runtime_for_an_inherited_instance() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/demo_proj_dcs.json"));
    let event_adapter = Arc::new(MockEventLoggingAdapter::new());
    let statsig = Statsig::new(
        "secret-fork-lazy-recovery-test",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter.clone()),
            event_logging_adapter: Some(event_adapter.clone()),
            event_logging_max_queue_size: Some(10),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );
    statsig
        .statsig_runtime
        .get_handle()
        .expect("runtime must be available")
        .block_on(statsig.initialize())
        .expect("Statsig must initialize");
    let expected_persistent_tasks = persistent_runtime_task_names(&statsig);
    let user = StatsigUser::with_user_id("fork-lazy-recovery-user");

    let child_pid = unsafe { libc::fork() };
    assert!(child_pid >= 0, "fork failed");

    if child_pid == 0 {
        let _ = statsig.check_gate(&user, "test_public");
        for index in 0..9 {
            statsig.log_event(
                &user,
                &format!("fork-lazy-recovery-event-{index}"),
                None,
                None,
            );
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        let exit_code = loop {
            let specs_restarted = specs_adapter.post_fork_background_sync_calls() >= 1;
            let persistent_tasks_restarted =
                persistent_runtime_task_names(&statsig) == expected_persistent_tasks;
            let event_flushed = event_adapter
                .no_diagnostics_logged_event_count
                .load(Ordering::SeqCst)
                > 0;
            if specs_restarted && persistent_tasks_restarted && event_flushed {
                break 0;
            }
            if Instant::now() >= deadline {
                break (!specs_restarted as i32)
                    | ((!persistent_tasks_restarted as i32) << 1)
                    | ((!event_flushed as i32) << 2);
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        unsafe { libc::_exit(exit_code) };
    }

    let status = wait_for_child(child_pid, Duration::from_secs(5));
    assert!(libc::WIFEXITED(status), "child did not exit normally");
    assert_eq!(
        libc::WEXITSTATUS(status),
        0,
        "lazy child recovery did not restore specs, persistent tasks, and automatic event flush"
    );
    shutdown(&statsig);
}

#[cfg(unix)]
#[test]
#[serial]
fn child_lifecycle_paths_do_not_flush_inherited_parent_events() {
    let specs_adapter = Arc::new(MockSpecsAdapter::with_data("tests/data/demo_proj_dcs.json"));
    let event_adapter = Arc::new(MockEventLoggingAdapter::new());
    let statsig = Statsig::new(
        "secret-fork-queue-ownership-test",
        Some(Arc::new(StatsigOptions {
            specs_adapter: Some(specs_adapter),
            event_logging_adapter: Some(event_adapter.clone()),
            disable_country_lookup: Some(true),
            ..StatsigOptions::new()
        })),
    );
    statsig
        .statsig_runtime
        .get_handle()
        .expect("runtime must be available")
        .block_on(statsig.initialize())
        .expect("Statsig must initialize");
    let user = StatsigUser::with_user_id("fork-queue-ownership-user");
    statsig.log_event(&user, "parent-event", None, None);
    let _ = statsig.get_parameter_store("missing-parent-store");

    for initialize_first in [true, false] {
        let child_pid = unsafe { libc::fork() };
        assert!(child_pid >= 0, "fork failed");
        if child_pid == 0 {
            if initialize_first {
                let _ = statsig
                    .statsig_runtime
                    .get_handle()
                    .unwrap()
                    .block_on(statsig.initialize());
                statsig.log_event(&user, "child-event", None, None);
                let _ = statsig.get_parameter_store("missing-child-store");
            }
            let _ = statsig
                .statsig_runtime
                .get_handle()
                .unwrap()
                .block_on(statsig.shutdown());
            let child_events = event_adapter
                .logged_payloads
                .lock()
                .unwrap()
                .iter()
                .flat_map(|payload| payload.events.as_array().into_iter().flatten())
                .cloned()
                .collect::<Vec<_>>();
            let event_names = child_events
                .iter()
                .filter_map(|event| event.get("eventName").and_then(|name| name.as_str()))
                .collect::<Vec<_>>();
            let child_checks = child_events
                .iter()
                .filter(|event| {
                    event.get("eventName").and_then(|name| name.as_str())
                        == Some("statsig::non_exposed_checks")
                })
                .filter_map(|event| {
                    event
                        .pointer("/metadata/checks")
                        .and_then(|checks| checks.as_str())
                })
                .collect::<Vec<_>>();
            let child_flushed_inherited_state = event_names.contains(&"parent-event")
                || event_names
                    .iter()
                    .filter(|name| **name == "statsig::non_exposed_checks")
                    .count()
                    != usize::from(initialize_first)
                || event_names.contains(&"child-event") != initialize_first
                || child_checks
                    .iter()
                    .any(|checks| checks.contains("missing-parent-store"))
                || child_checks
                    .iter()
                    .any(|checks| checks.contains("missing-child-store"))
                    != initialize_first;
            unsafe { libc::_exit(child_flushed_inherited_state as i32) };
        }
        let status = wait_for_child(child_pid, Duration::from_secs(5));
        assert!(libc::WIFEXITED(status), "child did not exit normally");
        assert_eq!(
            libc::WEXITSTATUS(status),
            0,
            "child lifecycle flushed copied parent events"
        );
    }

    shutdown(&statsig);
    assert_eq!(
        event_adapter
            .no_diagnostics_logged_event_count
            .load(Ordering::SeqCst),
        2,
        "only the parent may deliver event and non-exposure state queued before fork"
    );
}

#[cfg(unix)]
fn wait_for_child(child_pid: libc::pid_t, timeout: Duration) -> libc::c_int {
    let deadline = Instant::now() + timeout;
    loop {
        let mut status = 0;
        let result = unsafe { libc::waitpid(child_pid, &mut status, libc::WNOHANG) };
        assert!(result >= 0, "waitpid failed");
        if result == child_pid {
            return status;
        }
        if Instant::now() >= deadline {
            unsafe {
                libc::kill(child_pid, libc::SIGKILL);
                libc::waitpid(child_pid, &mut status, 0);
            }
            panic!("forked child did not recover within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
