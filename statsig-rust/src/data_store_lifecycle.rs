use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use parking_lot::Mutex as RegistryMutex;
use tokio::sync::{watch, Mutex};

use crate::data_store_interface::{
    DataStoreBytesResponse, DataStoreFeature, DataStoreResponse, DataStoreTrait, RequestPath,
};
use crate::statsig_global::{current_pid, current_runtime_epoch};
use crate::StatsigErr;

enum Lifecycle {
    Uninitialized,
    Initialized,
    ShuttingDown(watch::Receiver<Option<Result<(), StatsigErr>>>),
}

/// Tracks every DataStore poll until its future has been dropped, including
/// when the runtime aborts a task during a timed-out SDK shutdown.
pub(crate) struct DataStorePollingTask {
    process: ArcSwap<PollingProcess>,
}

struct PollingProcess {
    pid: u32,
    state: RegistryMutex<PollingState>,
    refresh: Arc<Mutex<()>>,
}

struct PollingState {
    epoch: u64,
    closed: bool,
    shutdowns_in_progress: usize,
    generations: Vec<Arc<PollingGeneration>>,
}

struct PollingGeneration {
    stop: watch::Sender<bool>,
    active: watch::Sender<usize>,
    scheduling: Mutex<()>,
    loop_running: AtomicBool,
}

pub(crate) struct PollCompletion(Arc<PollingGeneration>, bool);

pub(crate) struct PollingShutdownGuard(Arc<PollingProcess>);

impl Drop for PollingShutdownGuard {
    fn drop(&mut self) {
        self.0.state.lock().shutdowns_in_progress -= 1;
    }
}

impl Drop for PollCompletion {
    fn drop(&mut self) {
        if self.1 {
            self.0.loop_running.store(false, Ordering::SeqCst);
        }
        self.0.active.send_modify(|active| *active -= 1);
    }
}

impl PollCompletion {
    pub(crate) async fn lock_scheduling(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.0.scheduling.lock().await
    }

    pub(crate) fn is_stopped(&self) -> bool {
        *self.0.stop.borrow()
    }
}

impl PollingGeneration {
    fn new() -> Self {
        let (stop, _) = watch::channel(false);
        let (active, _) = watch::channel(0);
        Self {
            stop,
            active,
            scheduling: Mutex::new(()),
            loop_running: AtomicBool::new(false),
        }
    }
}

impl PollingProcess {
    fn new() -> Self {
        Self {
            pid: current_pid(),
            refresh: Arc::new(Mutex::new(())),
            state: RegistryMutex::new(PollingState {
                epoch: current_runtime_epoch(),
                closed: false,
                shutdowns_in_progress: 0,
                generations: vec![Arc::new(PollingGeneration::new())],
            }),
        }
    }
}

impl DataStorePollingTask {
    pub(crate) fn new() -> Self {
        Self {
            process: ArcSwap::from_pointee(PollingProcess::new()),
        }
    }

    // Shared across runtime generations, but never touches a parent's inherited
    // lock in a forked child. Waiters remain inside their registered work future.
    pub(crate) async fn lock_refresh(&self) -> tokio::sync::OwnedMutexGuard<()> {
        self.current_process().refresh.clone().lock_owned().await
    }

    pub(crate) fn is_stopped_in_epoch(&self, epoch: u64) -> bool {
        let process = self.current_process();
        let state = process.state.lock();
        state.closed || state.epoch != epoch || current_runtime_epoch() != epoch
    }

    fn current_process(&self) -> Arc<PollingProcess> {
        loop {
            let current = self.process.load_full();
            if current.pid == current_pid() {
                return current;
            }
            let previous = self
                .process
                .compare_and_swap(&current, Arc::new(PollingProcess::new()));
            if Arc::ptr_eq(&previous, &current) {
                // The child cannot drain tasks or drop locks owned by vanished threads.
                std::mem::forget(current);
            }
        }
    }

    pub(crate) fn begin(&self) -> Result<(watch::Receiver<bool>, PollCompletion), StatsigErr> {
        self.begin_in_epoch(current_runtime_epoch())
    }

    pub(crate) fn begin_loop(
        &self,
    ) -> Result<Option<(watch::Receiver<bool>, PollCompletion)>, StatsigErr> {
        self.begin_loop_in_epoch(current_runtime_epoch())
    }

    fn begin_loop_in_epoch(
        &self,
        epoch: u64,
    ) -> Result<Option<(watch::Receiver<bool>, PollCompletion)>, StatsigErr> {
        let (stop, mut completion) = self.begin_in_epoch(epoch)?;
        if completion.0.loop_running.swap(true, Ordering::SeqCst) {
            return Ok(None);
        }
        completion.1 = true;
        Ok(Some((stop, completion)))
    }

    fn begin_in_epoch(
        &self,
        epoch: u64,
    ) -> Result<(watch::Receiver<bool>, PollCompletion), StatsigErr> {
        let process = self.current_process();
        let mut state = process.state.lock();
        if state.closed {
            return Err(StatsigErr::ShutdownFailure(
                "DataStore polling was shut down".to_string(),
            ));
        }
        if state.epoch < epoch {
            for generation in &state.generations {
                generation.stop.send_replace(true);
            }
            state
                .generations
                .retain(|generation| *generation.active.borrow() != 0);
            state.generations.push(Arc::new(PollingGeneration::new()));
            state.epoch = epoch;
        }
        let generation = state.generations.last().unwrap().clone();
        generation.active.send_modify(|active| *active += 1);
        Ok((
            generation.stop.subscribe(),
            PollCompletion(generation, false),
        ))
    }

    pub(crate) fn stop(&self) {
        let process = self.current_process();
        let mut state = process.state.lock();
        state.closed = true;
        for generation in &state.generations {
            generation.stop.send_replace(true);
        }
    }

    pub(crate) fn close_for_shutdown(&self, on_closed: impl FnOnce()) -> PollingShutdownGuard {
        let process = self.current_process();
        let mut state = process.state.lock();
        state.shutdowns_in_progress += 1;
        state.closed = true;
        for generation in &state.generations {
            generation.stop.send_replace(true);
        }
        on_closed();
        drop(state);
        PollingShutdownGuard(process)
    }

    pub(crate) fn reopen(&self, on_admitted: impl FnOnce()) -> Result<(), StatsigErr> {
        let process = self.current_process();
        let mut state = process.state.lock();
        if state.shutdowns_in_progress != 0
            || (state.closed && state.generations.iter().any(|g| *g.active.borrow() != 0))
        {
            return Err(StatsigErr::ShutdownFailure(
                "DataStore scheduling is still draining".to_string(),
            ));
        }
        if state.closed {
            state.closed = false;
            state.epoch = current_runtime_epoch();
            state.generations = vec![Arc::new(PollingGeneration::new())];
        }
        on_admitted();
        Ok(())
    }

    pub(crate) async fn stop_and_wait(&self) {
        let generations = {
            let process = self.current_process();
            let mut state = process.state.lock();
            state.closed = true;
            for generation in &state.generations {
                generation.stop.send_replace(true);
            }
            state.generations.clone()
        };
        for generation in generations {
            let mut active = generation.active.subscribe();
            loop {
                if *active.borrow_and_update() == 0 || active.changed().await.is_err() {
                    break;
                }
            }
        }
    }
}

/// Shares one DataStore lifecycle between config specs and ID lists. Neither
/// adapter owns the underlying store once both can consume it.
pub(crate) struct SharedDataStore {
    inner: Arc<dyn DataStoreTrait>,
    lifecycle: Mutex<Lifecycle>,
}

impl SharedDataStore {
    pub(crate) fn new(inner: Arc<dyn DataStoreTrait>) -> Self {
        Self {
            inner,
            lifecycle: Mutex::new(Lifecycle::Uninitialized),
        }
    }
}

#[async_trait]
impl DataStoreTrait for SharedDataStore {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        let mut lifecycle = self.lifecycle.lock().await;
        match &*lifecycle {
            Lifecycle::Initialized => Ok(()),
            Lifecycle::ShuttingDown(_) => Err(StatsigErr::InvalidOperation(
                "DataStore was already shut down".to_string(),
            )),
            Lifecycle::Uninitialized => {
                self.inner.initialize().await?;
                *lifecycle = Lifecycle::Initialized;
                Ok(())
            }
        }
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        let mut completion = {
            let mut lifecycle = self.lifecycle.lock().await;
            match &*lifecycle {
                Lifecycle::Uninitialized => return Ok(()),
                Lifecycle::ShuttingDown(completion) => completion.clone(),
                Lifecycle::Initialized => {
                    let (sender, receiver) = watch::channel(None);
                    *lifecycle = Lifecycle::ShuttingDown(receiver.clone());
                    let inner = self.inner.clone();
                    tokio::spawn(async move {
                        sender.send_replace(Some(inner.shutdown().await));
                    });
                    receiver
                }
            }
        };

        loop {
            if let Some(result) = completion.borrow_and_update().clone() {
                return result;
            }
            completion.changed().await.map_err(|_| {
                StatsigErr::ShutdownFailure("DataStore shutdown task ended early".to_string())
            })?;
        }
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        self.inner.get(key).await
    }

    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
        self.inner.set(key, value, time).await
    }

    async fn get_bytes(&self, key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
        self.inner.get_bytes(key).await
    }

    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        time: Option<u64>,
    ) -> Result<(), StatsigErr> {
        self.inner.set_bytes(key, value, time).await
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        self.inner.support_polling_updates_for(path).await
    }

    async fn supports_feature(&self, feature: DataStoreFeature) -> bool {
        self.inner.supports_feature(feature).await
    }

    async fn support_polling_updates_for_feature(&self, feature: DataStoreFeature) -> Option<bool> {
        self.inner
            .support_polling_updates_for_feature(feature)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::sync::Notify;

    #[tokio::test]
    async fn old_completion_cannot_corrupt_replacement_poller() {
        let task = DataStorePollingTask::new();
        let epoch = current_runtime_epoch();
        let (old_stop, old) = task.begin_loop_in_epoch(epoch).unwrap().unwrap();
        let (new_stop, new) = task.begin_loop_in_epoch(epoch + 1).unwrap().unwrap();
        assert!(*old_stop.borrow());
        assert!(!*new_stop.borrow());
        assert!(task.begin_loop_in_epoch(epoch + 1).unwrap().is_none());
        drop(old);
        assert_eq!(*new.0.active.borrow(), 1);
        assert!(new.0.loop_running.load(Ordering::SeqCst));
        assert!(!*new_stop.borrow());

        let mut drain = Box::pin(task.stop_and_wait());
        assert!(tokio::time::timeout(Duration::from_millis(10), &mut drain)
            .await
            .is_err());
        assert!(task.begin_in_epoch(epoch + 2).is_err());
        assert!(task.reopen(|| {}).is_err());
        drop(new);
        tokio::time::timeout(Duration::from_secs(1), drain)
            .await
            .unwrap();
        task.reopen(|| {}).unwrap();
        assert!(task.begin_loop().unwrap().is_some());
    }

    #[tokio::test]
    async fn initialization_cannot_reopen_a_drained_gate_until_all_shutdowns_finish() {
        let task = DataStorePollingTask::new();
        let stopped = AtomicBool::new(false);
        let first = task.close_for_shutdown(|| stopped.store(true, Ordering::SeqCst));
        let second = task.close_for_shutdown(|| stopped.store(true, Ordering::SeqCst));
        task.stop_and_wait().await;
        let initialize = || task.reopen(|| stopped.store(false, Ordering::SeqCst));
        assert!(initialize().is_err());
        drop(first);
        assert!(initialize().is_err());
        assert!(stopped.load(Ordering::SeqCst));
        assert!(task.begin().is_err());
        drop(second);
        initialize().unwrap();
        assert!(!stopped.load(Ordering::SeqCst));
        assert!(task.begin().is_ok());
    }

    #[tokio::test]
    async fn shutdown_drains_superseded_generation_and_refuses_late_registration() {
        let task = DataStorePollingTask::new();
        let epoch = current_runtime_epoch();
        let (_, old) = task.begin_in_epoch(epoch).unwrap();
        let (_, new) = task.begin_in_epoch(epoch + 1).unwrap();
        drop(new);
        let mut drain = Box::pin(task.stop_and_wait());
        assert!(tokio::time::timeout(Duration::from_millis(10), &mut drain)
            .await
            .is_err());
        assert!(task.begin().is_err());
        drop(old);
        tokio::time::timeout(Duration::from_secs(1), drain)
            .await
            .unwrap();
    }

    #[test]
    fn new_process_does_not_enter_inherited_registry_lock() {
        let mut inherited = PollingProcess::new();
        inherited.pid = 0;
        let inherited = Arc::new(inherited);
        let task = Arc::new(DataStorePollingTask {
            process: ArcSwap::from(inherited.clone()),
        });
        let locked = inherited.state.lock();
        let (sent, received) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            sent.send(task.begin().is_ok()).unwrap();
        });
        let result = received.recv_timeout(Duration::from_secs(1));
        drop(locked);
        worker.join().unwrap();
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn refresh_gate_is_shared_across_runtime_generations() {
        let task = DataStorePollingTask::new();
        let epoch = current_runtime_epoch();
        let (old_stop, old) = task.begin_in_epoch(epoch).unwrap();
        let guard = task.lock_refresh().await;
        let (_new_stop, new) = task.begin_in_epoch(epoch + 1).unwrap();
        assert!(*old_stop.borrow());
        let mut queued = Box::pin(task.lock_refresh());
        assert!(futures::poll!(&mut queued).is_pending());
        drop(old);
        assert!(futures::poll!(&mut queued).is_pending());
        drop(guard);
        let _next_guard = tokio::time::timeout(Duration::from_secs(1), queued)
            .await
            .unwrap();
        drop(new);
    }

    #[cfg(unix)]
    #[test]
    fn forked_child_uses_fresh_refresh_gate_while_parent_holds_it() {
        let task = DataStorePollingTask::new();
        let guard = futures::executor::block_on(task.lock_refresh());
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0);
        if pid == 0 {
            let _child_guard = futures::executor::block_on(task.lock_refresh());
            unsafe { libc::_exit(0) };
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut status = 0;
        loop {
            let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
            if result == pid {
                break;
            }
            if std::time::Instant::now() >= deadline {
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                    libc::waitpid(pid, &mut status, 0);
                }
                panic!("child waited on the inherited refresh gate");
            }
            assert_eq!(result, 0);
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(guard);
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), 0);
    }

    struct CountingStore {
        initializes: AtomicUsize,
        shutdowns: AtomicUsize,
    }

    #[async_trait]
    impl DataStoreTrait for CountingStore {
        async fn initialize(&self) -> Result<(), StatsigErr> {
            self.initializes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), StatsigErr> {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
            Ok(DataStoreResponse {
                result: None,
                time: None,
            })
        }

        async fn set(
            &self,
            _key: &str,
            _value: &str,
            _time: Option<u64>,
        ) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn shared_lifecycle_initializes_and_shuts_down_once() {
        let inner = Arc::new(CountingStore {
            initializes: AtomicUsize::new(0),
            shutdowns: AtomicUsize::new(0),
        });
        let shared = Arc::new(SharedDataStore::new(inner.clone()));

        let (first, second) = tokio::join!(shared.initialize(), shared.initialize());
        first.unwrap();
        second.unwrap();
        let (first, second) = tokio::join!(shared.shutdown(), shared.shutdown());
        first.unwrap();
        second.unwrap();

        assert_eq!(inner.initializes.load(Ordering::SeqCst), 1);
        assert_eq!(inner.shutdowns.load(Ordering::SeqCst), 1);
        assert!(shared.initialize().await.is_err());
    }

    struct BlockingShutdownStore {
        started: Notify,
        release: Notify,
        shutdowns: AtomicUsize,
    }

    #[async_trait]
    impl DataStoreTrait for BlockingShutdownStore {
        async fn initialize(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), StatsigErr> {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            self.started.notify_one();
            self.release.notified().await;
            Ok(())
        }

        async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
            unreachable!()
        }

        async fn set(
            &self,
            _key: &str,
            _value: &str,
            _time: Option<u64>,
        ) -> Result<(), StatsigErr> {
            unreachable!()
        }

        async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn cancelled_shutdown_is_supervised_and_repeated_shutdown_waits_once() {
        let inner = Arc::new(BlockingShutdownStore {
            started: Notify::new(),
            release: Notify::new(),
            shutdowns: AtomicUsize::new(0),
        });
        let shared = SharedDataStore::new(inner.clone());
        shared.initialize().await.unwrap();

        {
            let first = shared.shutdown();
            tokio::pin!(first);
            tokio::time::timeout(Duration::from_secs(1), async {
                tokio::select! {
                    _ = &mut first => panic!("shutdown completed before the store was released"),
                    _ = inner.started.notified() => {},
                }
            })
            .await
            .unwrap();
        }
        assert_eq!(inner.shutdowns.load(Ordering::SeqCst), 1);
        assert!(shared.initialize().await.is_err());

        inner.release.notify_one();
        shared.shutdown().await.unwrap();
        shared.shutdown().await.unwrap();
        assert_eq!(inner.shutdowns.load(Ordering::SeqCst), 1);
    }
}
