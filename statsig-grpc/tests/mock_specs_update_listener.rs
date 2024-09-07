use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use sigstat::{SpecsInfo, SpecsSource, SpecsUpdate, SpecsUpdateListener};

#[derive(Default)]
pub struct MockListener {
    pub received_update: Mutex<Option<SpecsUpdate>>,
    next_update_notify: Mutex<Option<Arc<Notify>>>,
}

impl MockListener {
    pub async fn wait_for_next_update(&self) -> Result<(), Elapsed> {
        let notify = Arc::new(Notify::new());
        {
            *self.next_update_notify.lock().unwrap() = Some(notify.clone());
        }

        timeout(Duration::from_secs(10), notify.notified()).await
    }

    pub fn force_get_most_recent_update(&self) -> SpecsUpdate {
        self.nullable_get_most_recent_update()
            .unwrap()
    }

    pub fn nullable_get_most_recent_update(&self) -> Option<SpecsUpdate> {
        self
            .received_update
            .lock()
            .unwrap()
            .take()
    }
}
impl SpecsUpdateListener for MockListener {
    fn did_receive_specs_update(&self, update: SpecsUpdate) {
        *self.received_update.lock().unwrap() = Some(update);

        let notify = self.next_update_notify.lock().unwrap().take();
        if let Some(notify) = notify {
            notify.notify_one();
        }
    }

    fn get_current_specs_info(&self) -> SpecsInfo {
        SpecsInfo { lcut: None, source: SpecsSource::NoValues }
    }
}