use crate::statsig_global::{current_pid, StatsigGlobal};
use crate::StatsigErr;
use crate::{log_d, log_e};
use arc_swap::{ArcSwap, Guard};
use futures::future::join_all;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::{Builder, Handle, Runtime};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

const TAG: &str = stringify!(StatsigRuntime);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TaskId {
    tag: String,
    tokio_id: tokio::task::Id,
}

pub struct StatsigRuntime {
    spawned_tasks: Arc<ArcSwap<ProcessLocalTasks>>,
    shutdown_notify: Arc<Notify>,
    is_shutdown: Arc<AtomicBool>,
}

struct ProcessLocalTasks {
    pid: u32,
    tasks: Arc<Mutex<HashMap<TaskId, JoinHandle<()>>>>,
}

impl ProcessLocalTasks {
    fn new() -> Self {
        Self {
            pid: current_pid(),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl StatsigRuntime {
    #[must_use]
    pub fn get_runtime() -> Arc<StatsigRuntime> {
        create_runtime_if_required();

        Arc::new(StatsigRuntime {
            spawned_tasks: Arc::new(ArcSwap::from_pointee(ProcessLocalTasks::new())),
            shutdown_notify: Arc::new(Notify::new()),
            is_shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn get_handle(&self) -> Result<Handle, StatsigErr> {
        if let Ok(handle) = Handle::try_current() {
            return Ok(handle);
        }

        let global = StatsigGlobal::get();
        let mut rt = global
            .tokio_runtime
            .try_lock_for(Duration::from_secs(5))
            .ok_or_else(|| StatsigErr::LockFailure("Failed to lock tokio runtime".to_string()))?;
        if rt.is_none() {
            *rt = Some(Arc::new(create_new_runtime()));
        }
        if let Some(rt) = rt.as_ref() {
            return Ok(rt.handle().clone());
        }

        Err(StatsigErr::ThreadFailure(
            "No tokio runtime found".to_string(),
        ))
    }

    pub fn get_num_active_tasks(&self) -> usize {
        match self
            .current_spawned_tasks()
            .try_lock_for(Duration::from_secs(5))
        {
            Some(lock) => lock.len(),
            None => {
                log_e!(TAG, "Failed to lock spawned tasks for get_num_active_tasks");
                0
            }
        }
    }

    pub fn shutdown(&self) {
        self.shutdown_notify.notify_waiters();

        match self
            .current_spawned_tasks()
            .try_lock_for(Duration::from_secs(5))
        {
            Some(mut lock) => {
                for (_, task) in lock.drain() {
                    task.abort();
                }
            }
            None => {
                log_e!(TAG, "Failed to lock spawned tasks for shutdown");
            }
        }
    }

    pub fn spawn<F, Fut>(&self, tag: &str, task: F) -> Result<tokio::task::Id, StatsigErr>
    where
        F: FnOnce(Arc<Notify>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let tag_string = tag.to_string();
        let shutdown_notify = self.shutdown_notify.clone();
        let spawned_tasks = self.current_spawned_tasks();
        let is_shutdown = self.is_shutdown.clone();

        log_d!(TAG, "Spawning task {}", tag);

        let handle = self.get_handle()?.spawn(async move {
            if is_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            let task_id = tokio::task::id();
            log_d!(TAG, "Executing task {}.{}", tag_string, task_id);
            task(shutdown_notify).await;
            remove_join_handle_with_id(spawned_tasks, tag_string, &task_id);
        });

        Ok(self.insert_join_handle(tag, handle))
    }

    pub async fn await_tasks_with_tag(&self, tag: &str) {
        let mut handles = Vec::new();

        match self
            .current_spawned_tasks()
            .try_lock_for(Duration::from_secs(5))
        {
            Some(mut lock) => {
                let keys: Vec<TaskId> = lock.keys().cloned().collect();
                for key in &keys {
                    if key.tag == tag {
                        let removed = if let Some(handle) = lock.remove(key) {
                            handle
                        } else {
                            log_e!(TAG, "No running task found for tag {}", tag);
                            continue;
                        };

                        handles.push(removed);
                    }
                }
            }
            None => {
                log_e!(TAG, "Failed to lock spawned tasks for await_tasks_with_tag");
                return;
            }
        };

        join_all(handles).await;
    }

    pub async fn await_join_handle(
        &self,
        tag: &str,
        handle_id: &tokio::task::Id,
    ) -> Result<(), StatsigErr> {
        let task_id = TaskId {
            tag: tag.to_string(),
            tokio_id: *handle_id,
        };

        let handle = match self
            .current_spawned_tasks()
            .try_lock_for(Duration::from_secs(5))
        {
            Some(mut lock) => match lock.remove(&task_id) {
                Some(handle) => handle,
                None => {
                    return Err(StatsigErr::ThreadFailure(
                        "No running task found".to_string(),
                    ));
                }
            },
            None => {
                log_e!(TAG, "Failed to lock spawned tasks for await_join_handle");
                return Err(StatsigErr::ThreadFailure(
                    "Failed to lock spawned tasks".to_string(),
                ));
            }
        };

        handle
            .await
            .map_err(|e| StatsigErr::ThreadFailure(e.to_string()))?;

        Ok(())
    }

    pub fn get_running_task_ids(&self) -> Vec<(String, String)> {
        let spawned_tasks = self.current_spawned_tasks();
        let tasks = match spawned_tasks.try_lock_for(Duration::from_secs(5)) {
            Some(lock) => lock,
            None => {
                log_e!(TAG, "Failed to lock spawned tasks for get_running_task_ids");
                return Vec::new();
            }
        };

        tasks
            .keys()
            .map(|key| (key.tag.clone(), key.tokio_id.to_string()))
            .collect()
    }

    #[cfg(feature = "with_grpc")]
    pub(crate) fn has_running_task(&self, tag: &str, handle_id: &tokio::task::Id) -> bool {
        let spawned_tasks = self.current_spawned_tasks();
        let tasks = match spawned_tasks.try_lock_for(Duration::from_secs(5)) {
            Some(lock) => lock,
            None => {
                log_e!(TAG, "Failed to lock spawned tasks for has_running_task");
                return false;
            }
        };

        tasks
            .get(&TaskId {
                tag: tag.to_string(),
                tokio_id: *handle_id,
            })
            .is_some_and(|handle| !handle.is_finished())
    }

    fn insert_join_handle(&self, tag: &str, handle: JoinHandle<()>) -> tokio::task::Id {
        let handle_id = handle.id();
        let task_id = TaskId {
            tag: tag.to_string(),
            tokio_id: handle_id,
        };

        match self
            .current_spawned_tasks()
            .try_lock_for(Duration::from_secs(5))
        {
            Some(mut lock) => {
                lock.insert(task_id, handle);
            }
            None => {
                log_e!(TAG, "Failed to lock spawned tasks for insert_join_handle");
            }
        }

        handle_id
    }

    fn current_spawned_tasks(&self) -> Arc<Mutex<HashMap<TaskId, JoinHandle<()>>>> {
        let pid = current_pid();
        loop {
            let current = self.spawned_tasks.load_full();
            if current.pid == pid {
                return current.tasks.clone();
            }

            let replacement = Arc::new(ProcessLocalTasks::new());
            let previous = Guard::into_inner(
                self.spawned_tasks
                    .compare_and_swap(&current, replacement.clone()),
            );
            if Arc::ptr_eq(&previous, &current) {
                // The inherited task registry may be locked by a parent thread
                // that does not exist in this child. Never lock or drop it.
                std::mem::forget(previous);
                return replacement.tasks.clone();
            }
        }
    }
}

pub fn create_new_runtime() -> Runtime {
    #[cfg(not(target_family = "wasm"))]
    return Builder::new_multi_thread()
        .worker_threads(5)
        .thread_name("statsig")
        .enable_all()
        .build()
        .expect("Failed to create a tokio Runtime");

    #[cfg(target_family = "wasm")]
    return Builder::new_current_thread()
        .thread_name("statsig")
        .enable_all()
        .build()
        .expect("Failed to create a tokio Runtime (single-threaded for wasm");
}

fn remove_join_handle_with_id(
    spawned_tasks: Arc<Mutex<HashMap<TaskId, JoinHandle<()>>>>,
    tag: String,
    handle_id: &tokio::task::Id,
) {
    let task_id = TaskId {
        tag,
        tokio_id: *handle_id,
    };

    match spawned_tasks.try_lock_for(Duration::from_secs(5)) {
        Some(mut lock) => {
            lock.remove(&task_id);
        }
        None => {
            log_e!(
                TAG,
                "Failed to lock spawned tasks for remove_join_handle_with_id"
            );
        }
    }
}

fn create_runtime_if_required() {
    if Handle::try_current().is_ok() {
        log_d!(TAG, "External tokio runtime found");
        return;
    }

    let global = StatsigGlobal::get();
    let mut lock = global
        .tokio_runtime
        .try_lock_for(Duration::from_secs(5))
        .expect("Failed to lock owned tokio runtime");

    match lock.as_ref() {
        Some(_) => {
            log_d!(TAG, "Existing StatsigGlobal tokio runtime found");
        }
        None => {
            log_d!(TAG, "Creating new tokio runtime for StatsigGlobal");
            let rt = Arc::new(create_new_runtime());

            lock.replace(rt);
        }
    };
}

impl Drop for StatsigRuntime {
    fn drop(&mut self) {
        self.shutdown();

        // let opt_inner = match self.inner_runtime.lock() {
        //     Ok(mut inner_runtime) => inner_runtime.take(),
        //     Err(e) => {
        //         log_e!(TAG, "Failed to lock inner runtime {}", e);
        //         None
        //     }
        // };

        // let inner = match opt_inner {
        //     Some(inner) => inner,
        //     None => {
        //         log_d!(TAG, "Runtime owned by tokio");
        //         return;
        //     }
        // };

        // if Arc::strong_count(&inner) > 1 {
        //     // Another instance is still using the Runtime, so we can't drop it
        //     return;
        // }

        // if tokio::runtime::Handle::try_current().is_err() {
        //     println!("Not inside the Tokio runtime. Will automatically drop(inner).");
        //     // Not inside the Tokio runtime. Will automatically drop(inner).
        //     return;
        // }

        // log_w!(TAG, "Attempt to shutdown runtime from inside runtime");
        // std::thread::spawn(move || {
        //     println!("Dropping inner runtime from outside the Tokio runtime");
        //     // We should not drop from inside the runtime, but in the odd case we do,
        //     // moving inner to a new thread will prevent a panic
        //     drop(inner);
        // });
    }
}

#[cfg(all(test, unix))]
mod fork_tests {
    use super::*;
    use serial_test::serial;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    #[serial]
    fn forked_child_does_not_wait_on_inherited_spawned_tasks_lock() {
        let runtime = StatsigRuntime::get_runtime();
        let tasks = runtime.current_spawned_tasks();
        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        let holder = thread::spawn(move || {
            let _guard = tasks.lock();
            locked_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        locked_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        let child_pid = unsafe { libc::fork() };
        assert!(child_pid >= 0, "fork failed");

        if child_pid == 0 {
            let count = runtime.get_num_active_tasks();
            let exit_code = if count == 0 { 0 } else { 1 };
            unsafe { libc::_exit(exit_code) };
        }

        let status = wait_for_child(child_pid, Duration::from_secs(1));
        release_tx.send(()).unwrap();
        holder.join().unwrap();

        assert!(libc::WIFEXITED(status), "child did not exit normally");
        assert_eq!(libc::WEXITSTATUS(status), 0);
        StatsigGlobal::reset();
    }

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
                return status;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
