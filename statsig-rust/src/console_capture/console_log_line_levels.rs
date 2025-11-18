#[derive(Debug)]
pub enum StatsigLogLineLevel {
    Trace,
    Debug,
    Log,
    Info,
    Warn,
    Error,
}

impl StatsigLogLineLevel {
    pub fn to_status_string(&self) -> String {
        match self {
            StatsigLogLineLevel::Trace => "trace".to_string(),
            StatsigLogLineLevel::Debug => "debug".to_string(),
            StatsigLogLineLevel::Log => "info".to_string(), // info and log map to the same status
            StatsigLogLineLevel::Info => "info".to_string(),
            StatsigLogLineLevel::Warn => "warn".to_string(),
            StatsigLogLineLevel::Error => "error".to_string(),
        }
    }

    pub fn from_string(level: &str) -> Option<StatsigLogLineLevel> {
        match level.to_lowercase().as_str() {
            "trace" => Some(StatsigLogLineLevel::Trace),
            "debug" => Some(StatsigLogLineLevel::Debug),
            "log" => Some(StatsigLogLineLevel::Log),
            "info" => Some(StatsigLogLineLevel::Info),
            "warn" | "warning" => Some(StatsigLogLineLevel::Warn),
            "error" | "err" => Some(StatsigLogLineLevel::Error),
            _ => None,
        }
    }
}
