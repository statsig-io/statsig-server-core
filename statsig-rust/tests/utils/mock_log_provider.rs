use statsig_rust::output_logger::OutputLogProvider;
use std::sync::Mutex;

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
        self.logs.lock().unwrap().clear();
    }
}

impl OutputLogProvider for MockLogProvider {
    fn initialize(&self) {
        self.logs.lock().unwrap().push(RecordedLog::Init);
    }

    fn debug(&self, tag: &str, msg: String) {
        self.logs
            .lock()
            .unwrap()
            .push(RecordedLog::Debug(tag.to_string(), msg));
    }

    fn info(&self, tag: &str, msg: String) {
        self.logs
            .lock()
            .unwrap()
            .push(RecordedLog::Info(tag.to_string(), msg));
    }

    fn warn(&self, tag: &str, msg: String) {
        self.logs
            .lock()
            .unwrap()
            .push(RecordedLog::Warn(tag.to_string(), msg));
    }

    fn error(&self, tag: &str, msg: String) {
        self.logs
            .lock()
            .unwrap()
            .push(RecordedLog::Error(tag.to_string(), msg));
    }

    fn shutdown(&self) {
        self.logs.lock().unwrap().push(RecordedLog::Shutdown);
    }
}
