use serial_test::serial;
use statsig_rust::statsig_global::StatsigGlobal;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

#[cfg(unix)]
#[test]
#[serial]
fn forked_child_replaces_an_inherited_runtime_without_dropping_it() {
    {
        let global = StatsigGlobal::get();
        *global.tokio_runtime.lock() = Some(Arc::new(Runtime::new().unwrap()));
    }

    let child_pid = unsafe { libc::fork() };
    assert!(child_pid >= 0, "fork failed");

    if child_pid == 0 {
        let child_global = StatsigGlobal::get();
        let runtime = Arc::new(Runtime::new().unwrap());
        runtime.block_on(async {});
        *child_global.tokio_runtime.lock() = Some(runtime);

        let exit_code = if child_global.pid == std::process::id() {
            0
        } else {
            1
        };
        unsafe { libc::_exit(exit_code) };
    }

    let status = wait_for_child(child_pid, Duration::from_secs(5));
    assert!(libc::WIFEXITED(status), "child did not exit normally");
    assert_eq!(libc::WEXITSTATUS(status), 0);
    StatsigGlobal::reset();
}

#[cfg(unix)]
#[test]
#[serial]
fn forked_child_can_reset_before_get_without_dropping_inherited_runtime() {
    {
        let global = StatsigGlobal::get();
        *global.tokio_runtime.lock() = Some(Arc::new(Runtime::new().unwrap()));
    }

    let child_pid = unsafe { libc::fork() };
    assert!(child_pid >= 0, "fork failed");

    if child_pid == 0 {
        StatsigGlobal::reset();
        let child_global = StatsigGlobal::get();
        let exit_code = if child_global.pid == std::process::id() {
            0
        } else {
            1
        };
        unsafe { libc::_exit(exit_code) };
    }

    let status = wait_for_child(child_pid, Duration::from_secs(5));
    assert!(libc::WIFEXITED(status), "child did not exit normally");
    assert_eq!(libc::WEXITSTATUS(status), 0);
    StatsigGlobal::reset();
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
            panic!("forked child did not replace the inherited runtime within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
