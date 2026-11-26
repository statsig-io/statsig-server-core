use crate::{AmplitudeError, Event};
use std::{
    future::Future,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::{
    sync::{Notify, OnceCell},
    time::{sleep_until, Instant},
};

pub(crate) struct Queue {
    interval: Duration,
    capacity: usize,
    current: Mutex<Weak<Batch>>,
}

impl Queue {
    pub(crate) fn new(interval: Duration, capacity: usize) -> Self {
        Self {
            interval,
            capacity,
            current: Mutex::default(),
        }
    }

    pub(crate) fn push(&self, event: Event) -> Arc<Batch> {
        let mut current = self.current.lock().unwrap();
        if let Some(batch) = current.upgrade() {
            let mut events = batch.events.lock().unwrap();
            if events.len() < self.capacity && Instant::now() < batch.deadline {
                events.push(event);
                if events.len() == self.capacity {
                    batch.full.notify_one();
                }
                drop(events);
                return batch;
            }
        }

        let batch = Arc::new(Batch {
            deadline: Instant::now() + self.interval,
            capacity: self.capacity,
            events: Mutex::new(vec![event]),
            full: Notify::new(),
            result: OnceCell::new(),
        });
        *current = Arc::downgrade(&batch);
        batch
    }
}

pub(crate) struct Batch {
    deadline: Instant,
    capacity: usize,
    events: Mutex<Vec<Event>>,
    full: Notify,
    result: OnceCell<Result<u16, AmplitudeError>>,
}

impl Batch {
    pub(crate) async fn deliver<F>(
        &self,
        send: impl FnOnce(Vec<Event>) -> F,
    ) -> Result<u16, AmplitudeError>
    where
        F: Future<Output = Result<u16, AmplitudeError>>,
    {
        // A waiting call drives the upload. If it is canceled, another waiter can
        // resume delivery with the same events and insert IDs.
        self.result
            .get_or_init(|| async {
                if self.events.lock().unwrap().len() < self.capacity {
                    tokio::select! {
                        _ = self.full.notified() => {}
                        _ = sleep_until(self.deadline) => {}
                    }
                }
                let events = self.events.lock().unwrap().clone();
                send(events).await
            })
            .await
            .clone()
    }
}
