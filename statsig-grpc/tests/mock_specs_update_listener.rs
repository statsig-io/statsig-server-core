use std::sync::Mutex;
use statsig::{SpecsInfo, SpecsSource, SpecsUpdate, SpecsUpdateListener};

#[derive(Default)]
pub struct MockListener {
    pub received_update: Mutex<Option<SpecsUpdate>>
}
impl SpecsUpdateListener for MockListener {
    fn did_receive_specs_update(&self, update: SpecsUpdate) {
        let mut lock = self.received_update.lock().unwrap();
        *lock = Some(update);
    }

    fn get_current_specs_info(&self) -> SpecsInfo {
        SpecsInfo { lcut: None, source: SpecsSource::NoValues }
    }
}