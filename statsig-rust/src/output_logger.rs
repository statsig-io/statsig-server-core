use log::{debug, error, info, warn, Level};

const MAX_CHARS: usize = 400;
const TRUNCATED_SUFFIX: &str = "...[TRUNCATED]";

const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::Warn;

#[derive(Clone)]
pub enum LogLevel {
    None,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<&str> for LogLevel {
    fn from(level: &str) -> Self {
        match level.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            "none" => LogLevel::None,
            _ => DEFAULT_LOG_LEVEL,
        }
    }
}

impl From<u32> for LogLevel {
    fn from(level: u32) -> Self {
        match level {
            0 => LogLevel::None,
            1 => LogLevel::Error,
            2 => LogLevel::Warn,
            3 => LogLevel::Info,
            4 => LogLevel::Debug,
            _ => DEFAULT_LOG_LEVEL,
        }
    }
}

impl LogLevel {
    fn to_third_party_level(&self) -> Option<Level> {
        match self {
            LogLevel::Debug => Some(Level::Debug),
            LogLevel::Info => Some(Level::Info),
            LogLevel::Warn => Some(Level::Warn),
            LogLevel::Error => Some(Level::Error),
            LogLevel::None => None,
        }
    }
}

pub fn initialize_simple_output_logger(level: &Option<LogLevel>) {
    let level = level.as_ref().unwrap_or(&DEFAULT_LOG_LEVEL).clone();

    let final_level = match level {
        LogLevel::None => {
            return;
        }
        _ => match level.to_third_party_level() {
            Some(level) => level,
            None => return,
        },
    };

    match simple_logger::init_with_level(final_level) {
        Ok(()) => {}
        Err(_) => {
            log::set_max_level(final_level.to_level_filter());
        }
    }
}

pub fn log_message(tag: &str, level: LogLevel, msg: String) {
    let truncated_msg = if msg.chars().count() > MAX_CHARS {
        let visible_chars = MAX_CHARS.saturating_sub(TRUNCATED_SUFFIX.len());
        format!(
            "{}{}",
            msg.chars().take(visible_chars).collect::<String>(),
            TRUNCATED_SUFFIX
        )
    } else {
        msg
    };

    if let Some(level) = level.to_third_party_level() {
        match level {
            Level::Debug => debug!("[Statsig.{}] {}", tag, truncated_msg),
            Level::Info => info!("[Statsig.{}] {}", tag, truncated_msg),
            Level::Warn => warn!("[Statsig.{}] {}", tag, truncated_msg),
            Level::Error => error!("[Statsig.{}] {}", tag, truncated_msg),
            _ => {}
        };
    }
}

#[macro_export]
macro_rules! log_d {
  ($tag:expr, $($arg:tt)*) => {
        $crate::output_logger::log_message($tag, $crate::output_logger::LogLevel::Debug, format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_i {
  ($tag:expr, $($arg:tt)*) => {
        $crate::output_logger::log_message($tag, $crate::output_logger::LogLevel::Info, format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_w {
  ($tag:expr, $($arg:tt)*) => {
        $crate::output_logger::log_message($tag, $crate::output_logger::LogLevel::Warn, format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_e {
  ($tag:expr, $($arg:tt)*) => {
        $crate::output_logger::log_message($tag, $crate::output_logger::LogLevel::Error, format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_error_to_statsig_and_console {
  ($ops_stats:expr, $tag:expr, $($arg:tt)*) => {
    let err_message = format!($($arg)*);
    let event = ErrorBoundaryEvent {
        exception: err_message.clone(),
        tag: $tag.to_string(),
        extra: None,
    };
    $ops_stats.log_error(event);

    $crate::output_logger::log_message(&$tag, $crate::output_logger::LogLevel::Error, err_message)
  }
}
