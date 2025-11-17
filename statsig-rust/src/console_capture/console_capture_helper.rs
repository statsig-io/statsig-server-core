use std::sync::Arc;

use crate::event_logging::event_logger::EventLogger;
use crate::observability::console_capture_observer::ConsoleCaptureEvent;

pub struct ConsoleCapture {}

impl ConsoleCapture {
    pub fn new(_: Arc<EventLogger>) -> Self {
        Self {}
    }

    pub fn handle_console_capture_event(&self, _: ConsoleCaptureEvent) {
        // self.event_logger.enqueue(EnqueuePassthroughOp {
        //     event: StatsigEventInternal::new_statsig_log_line_event(event),
        // });
    }
}
