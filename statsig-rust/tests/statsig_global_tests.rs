use serial_test::serial;
use statsig_rust::{statsig_global::StatsigGlobal, StatsigRuntime};
use std::time::Duration;

fn is_runtime_none() -> bool {
    let global = StatsigGlobal::get();
    let rt = &global.tokio_runtime;
    let lock_result = rt.try_lock().expect("Failed to lock tokio runtime");
    lock_result.is_none()
}

fn is_runtime_some() -> bool {
    let global = StatsigGlobal::get();
    let rt = &global.tokio_runtime;
    let lock_result = rt.try_lock().expect("Failed to lock tokio runtime");
    lock_result.is_some()
}

#[test]
#[serial]
fn test_get_runtime() {
    let global = StatsigGlobal::get();
    let global_again = StatsigGlobal::get();

    assert_eq!(global.pid, global_again.pid);
}

#[test]
#[serial]
fn test_fork_resetting_runtime() {
    let global = StatsigGlobal::get();
    let original_pid = global.pid;

    assert!(is_runtime_none());

    let original_rt = StatsigRuntime::get_runtime();
    assert_eq!(original_pid, std::process::id());

    original_rt
        .spawn("test", |_| async {
            tokio::time::sleep(Duration::from_secs(5)).await;
        })
        .unwrap();

    assert!(is_runtime_some());

    let pid = unsafe { libc::fork() };
    if pid == 0 {
        let child_global = StatsigGlobal::get();
        assert_ne!(child_global.pid, original_pid);

        assert!(is_runtime_none());

        let child_rt = StatsigRuntime::get_runtime();
        child_rt
            .spawn("test", |_| async {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            })
            .unwrap();

        assert!(is_runtime_some());

        std::process::exit(0);
    }

    let global = StatsigGlobal::get();
    assert_eq!(global.pid, original_pid);

    unsafe {
        let mut status: i32 = 0;
        libc::waitpid(pid, &mut status, 0);
        assert_eq!(libc::WEXITSTATUS(status), 0);
    };

    let another_rt = StatsigRuntime::get_runtime();
    another_rt.shutdown();
    drop(another_rt);

    assert!(is_runtime_some(), "The runtime should not be dropped");
}
