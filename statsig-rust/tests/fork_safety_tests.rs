mod utils;

use serial_test::serial;
use statsig_rust::statsig_global::StatsigGlobal;
use statsig_rust::{Statsig, StatsigOptions, StatsigUser};
use std::sync::Arc;
use std::time::{Duration, Instant};
use utils::mock_event_logging_adapter::MockEventLoggingAdapter;
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
        let exit_code = if wait_for_sync_calls(&specs_adapter, 3) {
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
    let (statsig, specs_adapter, _) = setup();

    shutdown(&statsig);
    statsig
        .statsig_runtime
        .get_handle()
        .expect("runtime must be available")
        .block_on(statsig.initialize())
        .expect("Statsig must reinitialize");

    assert_eq!(specs_adapter.background_sync_calls(), 2);
    shutdown(&statsig);
}

#[cfg(unix)]
#[test]
#[serial]
fn forked_child_rebuilds_runtime_for_an_inherited_instance() {
    let (statsig, specs_adapter, user) = setup();
    let child_pid = unsafe { libc::fork() };
    assert!(child_pid >= 0, "fork failed");

    if child_pid == 0 {
        let _ = statsig.check_gate(&user, "test_public");
        let exit_code = if wait_for_sync_calls(&specs_adapter, 2) {
            0
        } else {
            1
        };
        unsafe { libc::_exit(exit_code) };
    }

    let status = wait_for_child(child_pid, Duration::from_secs(5));
    assert!(libc::WIFEXITED(status), "child did not exit normally");
    assert_eq!(libc::WEXITSTATUS(status), 0);
    shutdown(&statsig);
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
