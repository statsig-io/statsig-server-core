use log::{debug, error, info, warn, Level};

const MAX_CHARS: usize = 300;
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

pub fn log_message(level: Level, msg: String) {
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

    match level {
        Level::Debug => debug!("[Statsig] {}", truncated_msg),
        Level::Info => info!("[Statsig] {}", truncated_msg),
        Level::Warn => warn!("[Statsig] {}", truncated_msg),
        Level::Error => error!("[Statsig] {}", truncated_msg),
        _ => {}
    }
}

#[macro_export]
macro_rules! log_d {
  ($($arg:tt)*) => {
        $crate::output_logger::log_message(log::Level::Debug, format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_i {
  ($($arg:tt)*) => {
        $crate::output_logger::log_message(log::Level::Info, format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_w {
  ($($arg:tt)*) => {
        $crate::output_logger::log_message(log::Level::Warn, format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_e {
  ($($arg:tt)*) => {
        $crate::output_logger::log_message(log::Level::Error, format!($($arg)*))
    }
}
