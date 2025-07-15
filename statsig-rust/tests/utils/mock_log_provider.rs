use std::time::Duration;

use parking_lot::Mutex;
use statsig_rust::output_logger::OutputLogProvider;

#[derive(Debug, PartialEq)]
pub enum RecordedLog {
    Debug(String, String),
    Info(String, String),
    Warn(String, String),
    Error(String, String),
    Init,
    Shutdown,
}

pub struct MockLogProvider {
    pub logs: Mutex<Vec<RecordedLog>>,
}

impl MockLogProvider {
    pub fn new() -> Self {
        Self {
            logs: Mutex::new(Vec::new()),
        }
    }

    pub fn clear(&self) {
        self.logs
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .clear();
    }
}

impl OutputLogProvider for MockLogProvider {
    fn initialize(&self) {
        self.logs
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .push(RecordedLog::Init);
    }

    fn debug(&self, tag: &str, msg: String) {
        self.logs
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .push(RecordedLog::Debug(tag.to_string(), msg));
    }

    fn info(&self, tag: &str, msg: String) {
        self.logs
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .push(RecordedLog::Info(tag.to_string(), msg));
    }

    fn warn(&self, tag: &str, msg: String) {
        self.logs
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .push(RecordedLog::Warn(tag.to_string(), msg));
    }

    fn error(&self, tag: &str, msg: String) {
        self.logs
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .push(RecordedLog::Error(tag.to_string(), msg));
    }

    fn shutdown(&self) {
        self.logs
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .push(RecordedLog::Shutdown);
    }
}
