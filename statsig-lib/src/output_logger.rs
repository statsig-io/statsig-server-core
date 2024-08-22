use log::{debug, info, warn, error, Level};

const MAX_CHARS: usize = 200;
const TRUNCATED_SUFFIX: &str = "...[TRUNCATED]";

pub fn log_message(level: Level, msg: String) {
    let truncated_msg;
    if msg.chars().count() > MAX_CHARS {
        let visible_chars = MAX_CHARS.saturating_sub(TRUNCATED_SUFFIX.len());
        truncated_msg = format!(
            "{}{}",
            msg.chars().take(visible_chars).collect::<String>(),
            TRUNCATED_SUFFIX
        );
    } else {
        truncated_msg = msg;
    }

    match level {
        Level::Debug => debug!("[Statsig]: {}" ,truncated_msg),
        Level::Info => info!("[Statsig]: {}", truncated_msg),
        Level::Warn => warn!("[Statsig]: {}", truncated_msg),
        Level::Error => error!("[Statsig]: {}", truncated_msg),
        _ => {},
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
