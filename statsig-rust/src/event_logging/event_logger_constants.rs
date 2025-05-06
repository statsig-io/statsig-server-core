const TICK_INTERVAL_MS: u64 = 1000;
const MIN_FLUSH_INTERVAL_MS: u64 = 1000;
const MAX_LOG_EVENT_RETRIES: u8 = 5;

pub struct EventLoggerConstants;

impl EventLoggerConstants {
    pub fn tick_interval_ms() -> u64 {
        let env_var = std::env::var("STATSIG_TEST_OVERRIDE_TICK_INTERVAL_MS")
            .ok()
            .unwrap_or_default();

        if env_var.is_empty() {
            return TICK_INTERVAL_MS;
        }

        env_var.parse::<u64>().unwrap_or(TICK_INTERVAL_MS)
    }

    pub fn min_flush_interval_ms() -> u64 {
        let env_var = std::env::var("STATSIG_TEST_OVERRIDE_MIN_FLUSH_INTERVAL_MS")
            .ok()
            .unwrap_or_default();

        if env_var.is_empty() {
            return MIN_FLUSH_INTERVAL_MS;
        }

        env_var.parse::<u64>().unwrap_or(MIN_FLUSH_INTERVAL_MS)
    }

    pub fn max_log_event_retries() -> u8 {
        let env_var = std::env::var("STATSIG_TEST_OVERRIDE_MAX_LOG_EVENT_RETRIES")
            .ok()
            .unwrap_or_default();

        if env_var.is_empty() {
            return MAX_LOG_EVENT_RETRIES;
        }

        env_var.parse::<u8>().unwrap_or(MAX_LOG_EVENT_RETRIES)
    }
}
