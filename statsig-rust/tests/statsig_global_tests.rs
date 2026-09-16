use serial_test::serial;
use statsig_rust::{statsig_global::StatsigGlobal, StatsigRuntime};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread::ThreadId;
use std::time::{Duration, Instant};

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

static WORKERS_EXITED: AtomicUsize = AtomicUsize::new(0);

struct ExitRecorder;

impl Drop for ExitRecorder {
    fn drop(&mut self) {
        WORKERS_EXITED.fetch_add(1, Ordering::SeqCst);
    }
}

thread_local! {
    /// Initialized on every worker thread that runs a task below, so each one
    /// reports itself when it exits. Portable stand-in for counting OS threads.
    static EXIT_RECORDER: ExitRecorder = const { ExitRecorder };
}

/// Runs work on the shared runtime until every worker thread has been used,
/// and returns the set of threads that ran it. Each of those threads now has
/// an `ExitRecorder` that fires when the thread terminates.
fn occupy_worker_threads(runtime: &StatsigRuntime) -> HashSet<ThreadId> {
    let handle = runtime.get_handle().expect("Failed to get runtime handle");
    let (tx, rx) = mpsc::channel::<ThreadId>();

    for _ in 0..40 {
        let tx = tx.clone();
        handle.spawn(async move {
            EXIT_RECORDER.with(|_| {});
            let _ = tx.send(std::thread::current().id());
            // Hold the worker so the pool spreads the remaining tasks around.
            std::thread::sleep(Duration::from_millis(20));
        });
    }
    drop(tx);

    let mut threads = HashSet::new();
    while let Ok(id) = rx.recv_timeout(Duration::from_secs(10)) {
        threads.insert(id);
    }

    threads
}

fn wait_for(timeout: Duration, condition: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    condition()
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

    assert!(
        is_runtime_some(),
        "Per-instance shutdown must leave the shared runtime for the next instance. \
         StatsigGlobal::shutdown_shared_runtime is what stops it."
    );

    StatsigGlobal::shutdown_shared_runtime();
}

#[test]
#[serial]
fn test_shutdown_shared_runtime_stops_worker_threads() {
    StatsigGlobal::shutdown_shared_runtime();
    WORKERS_EXITED.store(0, Ordering::SeqCst);

    let rt = StatsigRuntime::get_runtime();
    let workers = occupy_worker_threads(&rt);

    assert!(
        !workers.is_empty(),
        "No worker threads ran, so nothing was observed"
    );
    assert_eq!(
        WORKERS_EXITED.load(Ordering::SeqCst),
        0,
        "Worker threads exited while the runtime was still in use"
    );

    // The most complete teardown a binding can do per instance.
    rt.shutdown();
    drop(rt);

    assert!(
        !wait_for(Duration::from_millis(500), || WORKERS_EXITED
            .load(Ordering::SeqCst)
            > 0),
        "Per-instance shutdown stopped shared worker threads. \
         Bindings rely on the shared runtime surviving it."
    );
    assert!(is_runtime_some());

    // This is what a binding must call before the library is unloaded. PHP
    // unloads it at the end of every request, and on Windows that unmaps the
    // module while these threads are still running inside it.
    assert!(StatsigGlobal::shutdown_shared_runtime());

    assert!(
        wait_for(Duration::from_secs(5), || WORKERS_EXITED
            .load(Ordering::SeqCst)
            >= workers.len()),
        "Only {} of {} worker threads exited. Threads that outlive the library \
         fault when the module is unmapped.",
        WORKERS_EXITED.load(Ordering::SeqCst),
        workers.len()
    );
    assert!(is_runtime_none());
}

#[test]
#[serial]
fn test_shutdown_shared_runtime_is_idempotent_and_safe_when_unused() {
    StatsigGlobal::shutdown_shared_runtime();

    // Never initialized, and repeated calls.
    assert!(StatsigGlobal::shutdown_shared_runtime());
    assert!(StatsigGlobal::shutdown_shared_runtime());
    assert!(is_runtime_none());

    let rt = StatsigRuntime::get_runtime();
    assert!(is_runtime_some());
    drop(rt);

    assert!(StatsigGlobal::shutdown_shared_runtime());
    assert!(StatsigGlobal::shutdown_shared_runtime());
    assert!(is_runtime_none());
}

#[test]
#[serial]
fn test_shutdown_shared_runtime_refuses_from_inside_a_runtime_thread() {
    StatsigGlobal::shutdown_shared_runtime();

    let rt = StatsigRuntime::get_runtime();
    let handle = rt.get_handle().expect("Failed to get runtime handle");

    // Dropping a runtime from inside itself panics. PHP's teardown made this
    // reachable, so the guard has to hold rather than relying on callers.
    let reset_from_worker = handle.block_on(async { StatsigGlobal::shutdown_shared_runtime() });

    assert!(
        !reset_from_worker,
        "Shared runtime was reset from inside a runtime thread"
    );
    assert!(
        is_runtime_some(),
        "The runtime was dropped from inside itself"
    );

    drop(rt);
    StatsigGlobal::shutdown_shared_runtime();
}
