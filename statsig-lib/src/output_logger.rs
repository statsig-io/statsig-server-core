use log::{debug, error, info, warn, Level};

const MAX_CHARS: usize = 400;
const TRUNCATED_SUFFIX: &str = "...[TRUNCATED]";

#[derive(Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn from_int(level: u32) -> Option<LogLevel> {
        let result = match level {
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => return None,
        };

        Some(result)
    }

    fn to_third_party_level(&self) -> Level {
        match self {
            LogLevel::Debug => Level::Debug,
            LogLevel::Info => Level::Info,
            LogLevel::Warn => Level::Warn,
            LogLevel::Error => Level::Error,
        }
    }
}

pub fn initialize_simple_output_logger(level: &Option<LogLevel>) {
    let level = level
        .as_ref()
        .unwrap_or(&LogLevel::Warn)
        .clone()
        .to_third_party_level();

    match simple_logger::init_with_level(level) {
        Ok(_) => {}
        Err(_) => {
            log::set_max_level(level.to_level_filter());
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

    match level.to_third_party_level() {
        Level::Debug => debug!("[Statsig.{}] {}", tag, truncated_msg),
        Level::Info => info!("[Statsig.{}] {}", tag, truncated_msg),
        Level::Warn => warn!("[Statsig.{}] {}", tag, truncated_msg),
        Level::Error => error!("[Statsig.{}] {}", tag, truncated_msg),
        _ => {}
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
    };
    $ops_stats.log_error(event);

    $crate::output_logger::log_message(&$tag, $crate::output_logger::LogLevel::Error, err_message)
  }
}
