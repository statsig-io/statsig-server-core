pub struct EventLoggerDynamicDefaults {
    pub flush_interval_ms: u64,
    pub batching_interval: u64,
    pub min_flush_interval_ms: u64,
}

impl EventLoggerDynamicDefaults {
    #[cfg(not(feature = "testing"))]
    pub fn new() -> Self {
        Self {
            flush_interval_ms: 1_000,
            batching_interval: 60_000,
            min_flush_interval_ms: 1_000,
        }
    }

    #[cfg(feature = "testing")]
    pub fn new() -> Self {
        Self {
            flush_interval_ms: Self::get_env_or_default("FLUSH_INTERVAL_MS", 1),
            batching_interval: Self::get_env_or_default("BATCHING_INTERVAL", 5),
            min_flush_interval_ms: Self::get_env_or_default("MIN_FLUSH_INTERVAL_MS", 1),
        }
    }

    #[cfg(feature = "testing")]
    fn get_env_or_default(env_var: &str, default: u64) -> u64 {
        std::env::var(env_var)
            .ok()
            .and_then(|val| val.parse().ok())
            .unwrap_or(default)
    }
}
