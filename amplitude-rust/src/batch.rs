use crate::{AmplitudeError, Event, FlushResults};
use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    runtime::Handle,
    sync::{watch, Notify},
    time::{sleep_until, Instant},
};

type Outcome = Result<u16, AmplitudeError>;

pub(crate) struct Queue {
    interval: Duration,
    capacity: usize,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    pending: Option<Arc<Batch>>,
    active: Vec<Arc<Batch>>,
    closed: bool,
}

impl Queue {
    pub(crate) fn new(interval: Duration, capacity: usize) -> Self {
        Self {
            interval,
            capacity,
            state: Mutex::default(),
        }
    }

    pub(crate) fn enqueue<F>(
        self: &Arc<Self>,
        event: Event,
        send: impl FnOnce(Vec<Event>) -> F + Send + 'static,
    ) -> Result<Arc<Batch>, AmplitudeError>
    where
        F: Future<Output = Outcome> + Send + 'static,
    {
        let runtime = Handle::try_current()
            .map_err(|_| AmplitudeError::InvalidInput("Event delivery requires a Tokio runtime"))?;
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return Err(AmplitudeError::Closed);
        }
        let now = Instant::now();
        if let Some(batch) = state.pending.take() {
            if now < batch.deadline {
                let mut events = batch.events.lock().unwrap();
                events.push(event);
                if events.len() == self.capacity {
                    batch.sealed.notify_one();
                } else {
                    state.pending = Some(Arc::clone(&batch));
                }
                drop(events);
                return Ok(batch);
            }
            batch.sealed.notify_one();
        }

        let (outcome, receiver) = watch::channel(None);
        let batch = Arc::new(Batch {
            deadline: now + self.interval,
            events: Mutex::new(vec![event]),
            sealed: Notify::new(),
            outcome: receiver,
        });
        if self.capacity > 1 {
            state.pending = Some(Arc::clone(&batch));
        } else {
            batch.sealed.notify_one();
        }
        state.active.push(Arc::clone(&batch));
        let upload = ActiveBatch {
            queue: Arc::clone(self),
            batch: Arc::clone(&batch),
        };
        drop(state);
        runtime.spawn(async move {
            let events = upload.take_events().await;
            outcome.send_replace(Some(send(events).await));
        });
        Ok(batch)
    }

    pub(crate) async fn drain(&self, close: bool) -> FlushResults {
        let active = {
            let mut state = self.state.lock().unwrap();
            state.closed |= close;
            if let Some(batch) = state.pending.take() {
                batch.sealed.notify_one();
            }
            state.active.clone()
        };
        let mut results = Vec::with_capacity(active.len());
        for batch in active {
            results.push(batch.result().await);
        }
        results
    }
}

pub(crate) struct Batch {
    deadline: Instant,
    events: Mutex<Vec<Event>>,
    sealed: Notify,
    outcome: watch::Receiver<Option<Outcome>>,
}

impl Batch {
    pub(crate) async fn result(&self) -> Outcome {
        let mut receiver = self.outcome.clone();
        let outcome = receiver
            .wait_for(Option::is_some)
            .await
            .map_err(|_| AmplitudeError::WorkerStopped)?;
        outcome
            .as_ref()
            .cloned()
            .ok_or(AmplitudeError::WorkerStopped)?
    }
}

struct ActiveBatch {
    queue: Arc<Queue>,
    batch: Arc<Batch>,
}

impl ActiveBatch {
    async fn take_events(&self) -> Vec<Event> {
        tokio::select! {
            _ = self.batch.sealed.notified() => {}
            _ = sleep_until(self.batch.deadline) => {}
        }
        let mut state = self.queue.state.lock().unwrap();
        self.clear_pending(&mut state);
        std::mem::take(&mut *self.batch.events.lock().unwrap())
    }

    fn clear_pending(&self, state: &mut State) {
        if state
            .pending
            .as_ref()
            .is_some_and(|pending| Arc::ptr_eq(pending, &self.batch))
        {
            state.pending = None;
        }
    }
}

impl Drop for ActiveBatch {
    fn drop(&mut self) {
        let mut state = self.queue.state.lock().unwrap();
        self.clear_pending(&mut state);
        state
            .active
            .retain(|batch| !Arc::ptr_eq(batch, &self.batch));
    }
}
