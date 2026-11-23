use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use chrono::Utc;
use rand::Rng;

use crate::global_configs::{GlobalConfigs, MAX_SAMPLING_RATE};

/// Calls recorded per sampled window. Each call adds a start and an end
/// marker, so a full window fills one diagnostics event (50 markers).
pub const API_CALL_WINDOW_CALLS: u32 = 25;
const API_CALL_WINDOW_MS: u64 = 60_000;
const API_CALL_SAMPLING_KEY: &str = "api_call";

/// Decides which evaluation calls get api_call diagnostics.
///
/// Evaluations are the SDK's hot path, so sampling is done per time window,
/// not per call. Once per window the `api_call` rate is rolled; a sampled
/// window records the next `API_CALL_WINDOW_CALLS` calls and an unsampled one
/// records none. An unrecorded call costs one relaxed atomic load.
pub struct ApiCallDiagnostics {
    remaining_calls: AtomicU32,
    window_started_at_ms: AtomicU64,
    next_marker_id: AtomicU64,
}

impl Default for ApiCallDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiCallDiagnostics {
    pub fn new() -> Self {
        Self {
            remaining_calls: AtomicU32::new(0),
            window_started_at_ms: AtomicU64::new(0),
            next_marker_id: AtomicU64::new(0),
        }
    }

    /// Claims a recording slot for one call. Returns the marker ID that pairs
    /// the call's start and end markers, or None if this call is not recorded.
    pub fn try_record_call(&self) -> Option<String> {
        let mut remaining = self.remaining_calls.load(Ordering::Relaxed);
        loop {
            if remaining == 0 {
                return None;
            }
            match self.remaining_calls.compare_exchange_weak(
                remaining,
                remaining - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => remaining = actual,
            }
        }
        let id = self.next_marker_id.fetch_add(1, Ordering::Relaxed);
        Some(id.to_string())
    }

    /// Starts a new window if the current one has run its course.
    /// Returns true if the window ended, so its markers should be sent.
    pub fn try_rotate_window(&self, global_configs: &GlobalConfigs) -> bool {
        self.try_rotate_window_at(global_configs, Utc::now().timestamp_millis() as u64)
    }

    fn try_rotate_window_at(&self, global_configs: &GlobalConfigs, now_ms: u64) -> bool {
        let started_at = self.window_started_at_ms.load(Ordering::Relaxed);
        if now_ms.saturating_sub(started_at) < window_ms() {
            return false;
        }

        // Until the server has sent an api_call rate (the event logger ticks
        // before initialize finishes), record nothing and keep the window due,
        // so the first window is rolled with the real rate instead of waiting
        // a full window.
        let Some(rate) = global_configs
            .use_diagnostics_sampling_rate(API_CALL_SAMPLING_KEY, |rate| rate.copied())
        else {
            self.remaining_calls.store(0, Ordering::Relaxed);
            return true;
        };

        if self
            .window_started_at_ms
            .compare_exchange(started_at, now_ms, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return false; // another caller rotated this window
        }

        let sampled = rand::thread_rng().gen::<f64>() * MAX_SAMPLING_RATE < rate;
        let calls = if sampled { API_CALL_WINDOW_CALLS } else { 0 };
        self.remaining_calls.store(calls, Ordering::Relaxed);
        true
    }
}

fn window_ms() -> u64 {
    std::env::var("STATSIG_TEST_OVERRIDE_API_CALL_WINDOW_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(API_CALL_WINDOW_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn global_configs(sdk_key: &str, rate: Option<f64>) -> std::sync::Arc<GlobalConfigs> {
        let configs = GlobalConfigs::get_instance(sdk_key);
        if let Some(rate) = rate {
            configs.set_diagnostics_sampling_rates(HashMap::from([(
                API_CALL_SAMPLING_KEY.to_string(),
                rate,
            )]));
        }
        configs
    }

    #[test]
    fn test_records_nothing_before_the_first_window() {
        let api_calls = ApiCallDiagnostics::new();
        assert!(api_calls.try_record_call().is_none());
    }

    #[test]
    fn test_sampled_window_records_a_fixed_number_of_calls() {
        let configs = global_configs("secret-api-call-sampled", Some(MAX_SAMPLING_RATE));
        let api_calls = ApiCallDiagnostics::new();

        assert!(api_calls.try_rotate_window_at(&configs, API_CALL_WINDOW_MS));
        let ids: Vec<String> = (0..API_CALL_WINDOW_CALLS + 5)
            .filter_map(|_| api_calls.try_record_call())
            .collect();

        assert_eq!(ids.len(), API_CALL_WINDOW_CALLS as usize);
        assert_eq!(ids.first().map(String::as_str), Some("0"));
        assert_eq!(ids.last().map(String::as_str), Some("24"));
    }

    #[test]
    fn test_unsampled_and_missing_rates_record_nothing() {
        for (sdk_key, rate) in [
            ("secret-api-call-zero-rate", Some(0.0)),
            ("secret-api-call-no-rate", None),
        ] {
            let configs = global_configs(sdk_key, rate);
            let api_calls = ApiCallDiagnostics::new();

            assert!(api_calls.try_rotate_window_at(&configs, API_CALL_WINDOW_MS));
            assert!(api_calls.try_record_call().is_none(), "{sdk_key}");
        }
    }

    #[test]
    fn test_window_stays_due_until_a_rate_is_known() {
        let configs = global_configs("secret-api-call-late-rate", None);
        let api_calls = ApiCallDiagnostics::new();
        let start = API_CALL_WINDOW_MS;

        // the event logger can tick before initialize has loaded the rates
        assert!(api_calls.try_rotate_window_at(&configs, start));
        assert!(api_calls.try_record_call().is_none());

        // once rates arrive, the very next tick starts a window
        configs.set_diagnostics_sampling_rates(HashMap::from([(
            API_CALL_SAMPLING_KEY.to_string(),
            MAX_SAMPLING_RATE,
        )]));
        assert!(api_calls.try_rotate_window_at(&configs, start + 10));
        assert!(api_calls.try_record_call().is_some());
    }

    #[test]
    fn test_window_rotates_at_most_once_per_interval() {
        let configs = global_configs("secret-api-call-interval", Some(MAX_SAMPLING_RATE));
        let api_calls = ApiCallDiagnostics::new();
        let start = API_CALL_WINDOW_MS;

        assert!(api_calls.try_rotate_window_at(&configs, start));
        assert!(!api_calls.try_rotate_window_at(&configs, start + API_CALL_WINDOW_MS - 1));
        assert!(api_calls.try_rotate_window_at(&configs, start + API_CALL_WINDOW_MS));
    }
}
