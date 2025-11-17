use std::collections::HashMap;
use std::sync::Arc;

use crate::console_capture::console_log_line_levels::StatsigLogLineLevel;
use crate::event_logging::event_logger::EventLogger;
use crate::event_logging::event_queue::queued_passthrough::EnqueuePassthroughOp;
use crate::event_logging::statsig_event_internal::StatsigEventInternal;
use crate::observability::console_capture_observer::ConsoleCaptureEvent;
use crate::user::StatsigUserLoggable;

pub struct ConsoleCapture {
    event_logger: Arc<EventLogger>,
}

impl ConsoleCapture {
    pub fn new(event_logger: Arc<EventLogger>) -> Self {
        Self { event_logger }
    }

    pub fn handle_console_capture_event(&self, event: ConsoleCaptureEvent) {
        let log_level = StatsigLogLineLevel::from_string(&event.level);
        let Some(log_level) = log_level else {
            return;
        };

        let metadata = event.stack_trace.map(|stack_trace| {
            let mut map = HashMap::new();
            map.insert("trace".to_string(), stack_trace);
            map
        });

        self.event_logger.enqueue(EnqueuePassthroughOp {
            event: StatsigEventInternal::new_statsig_log_line_event(
                StatsigUserLoggable::null(),
                log_level,
                Some(event.payload.join(" ")),
                metadata,
                Some(event.timestamp),
            ),
        });
    }
}
