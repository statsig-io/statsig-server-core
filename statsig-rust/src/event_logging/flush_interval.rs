use std::sync::atomic::AtomicU64;

use crate::{event_logging::event_logger_constants::EventLoggerConstants, log_d};
use chrono::Utc;
use std::sync::atomic::Ordering::Relaxed;

const TAG: &str = stringify!(FlushInterval);

#[derive(Default)]
pub struct FlushInterval {
    current_flush_interval_ms: AtomicU64,
    last_scheduled_flush_attempt_time: AtomicU64,
}

impl FlushInterval {
    pub fn new() -> Self {
        Self {
            current_flush_interval_ms: AtomicU64::new(EventLoggerConstants::min_flush_interval_ms()),
            last_scheduled_flush_attempt_time: AtomicU64::new(get_now_timestamp()),
        }
    }

    pub fn get_current_flush_interval_ms(&self) -> u64 {
        self.current_flush_interval_ms.load(Relaxed)
    }

    pub fn mark_scheduled_flush_attempt(&self) {
        let now = get_now_timestamp();
        self.last_scheduled_flush_attempt_time.store(now, Relaxed);
    }

    pub fn adjust_for_success(&self) {
        let current = self.load_current_interval();
        let adjusted = (current / 2).max(EventLoggerConstants::min_flush_interval_ms());
        self.current_flush_interval_ms.store(adjusted, Relaxed);

        log_d!(
            TAG,
            "Flush interval adjusted for success: was {}ms, now {}ms",
            current,
            adjusted
        );
    }

    pub fn adjust_for_failure(&self) {
        let current = self.load_current_interval();
        let adjusted = (current * 2).min(EventLoggerConstants::max_flush_interval_ms());
        self.current_flush_interval_ms.store(adjusted, Relaxed);

        log_d!(
            TAG,
            "Flush interval adjusted for failure: was {}ms, now {}ms",
            current,
            adjusted
        );
    }

    pub fn has_cooled_from_most_recent_failure(&self) -> bool {
        let last_flush_attempt_time = self.load_last_scheduled_flush_attempt_time();
        let flush_interval_ms = self.load_current_interval();
        let next_flush_time = last_flush_attempt_time + flush_interval_ms;
        next_flush_time < get_now_timestamp()
    }

    pub fn has_waited_max_allowed_interval(&self) -> bool {
        let last_flush_attempt_time = self.load_last_scheduled_flush_attempt_time();
        let next_flush_time =
            last_flush_attempt_time + EventLoggerConstants::max_flush_interval_ms();
        next_flush_time < get_now_timestamp()
    }

    pub fn has_completely_recovered_from_backoff(&self) -> bool {
        let current_interval = self.load_current_interval();
        current_interval <= EventLoggerConstants::min_flush_interval_ms()
    }

    fn load_current_interval(&self) -> u64 {
        self.current_flush_interval_ms.load(Relaxed)
    }

    fn load_last_scheduled_flush_attempt_time(&self) -> u64 {
        self.last_scheduled_flush_attempt_time.load(Relaxed)
    }
}

fn get_now_timestamp() -> u64 {
    Utc::now().timestamp_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_doubles_backoff() {
        let flush_interval = FlushInterval::new();

        flush_interval.adjust_for_failure();
        assert_eq!(flush_interval.get_current_flush_interval_ms(), 2000);

        flush_interval.adjust_for_failure();
        assert_eq!(flush_interval.get_current_flush_interval_ms(), 4000);
    }

    #[test]
    fn test_failure_backoff_max() {
        let flush_interval = FlushInterval::new();

        for _ in 0..1000 {
            flush_interval.adjust_for_failure();
        }
        assert_eq!(flush_interval.get_current_flush_interval_ms(), 60000);
    }

    #[test]
    fn test_success_halves_backoff() {
        let flush_interval = FlushInterval::new();

        for _ in 0..1000 {
            flush_interval.adjust_for_failure();
        }

        flush_interval.adjust_for_success();
        assert_eq!(flush_interval.get_current_flush_interval_ms(), 30000);

        flush_interval.adjust_for_success();
        assert_eq!(flush_interval.get_current_flush_interval_ms(), 15000);
    }

    #[test]
    fn test_success_backoff_min() {
        let flush_interval = FlushInterval::new();

        for _ in 0..1000 {
            flush_interval.adjust_for_success();
        }

        assert_eq!(flush_interval.get_current_flush_interval_ms(), 1000);
    }
}
