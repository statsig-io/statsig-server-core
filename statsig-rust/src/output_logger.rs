use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use log::{debug, error, info, warn, Level};

const MAX_CHARS: usize = 400;
const TRUNCATED_SUFFIX: &str = "...[TRUNCATED]";

const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::Warn;

lazy_static::lazy_static! {
    static ref LOGGER_STATE: RwLock<LoggerState> = RwLock::new(LoggerState {
        level: DEFAULT_LOG_LEVEL,
        provider: None,
    });
}

struct LoggerState {
    level: LogLevel,
    provider: Option<Arc<dyn OutputLogProvider>>,
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug)]
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

    fn to_number(&self) -> u32 {
        match self {
            LogLevel::Debug => 4,
            LogLevel::Info => 3,
            LogLevel::Warn => 2,
            LogLevel::Error => 1,
            LogLevel::None => 0,
        }
    }
}

pub trait OutputLogProvider: Send + Sync {
    fn initialize(&self);
    fn debug(&self, tag: &str, msg: String);
    fn info(&self, tag: &str, msg: String);
    fn warn(&self, tag: &str, msg: String);
    fn error(&self, tag: &str, msg: String);
    fn shutdown(&self);
}

pub fn initialize_output_logger(
    level: &Option<LogLevel>,
    provider: Option<Arc<dyn OutputLogProvider>>,
) {
    if INITIALIZED.load(Ordering::SeqCst) {
        return;
    }

    let mut state = LOGGER_STATE.write().unwrap();
    let level = level.as_ref().unwrap_or(&DEFAULT_LOG_LEVEL).clone();
    state.level = level.clone();

    if let Some(provider_impl) = provider {
        provider_impl.initialize();
        state.provider = Some(provider_impl);
    } else {
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
    INITIALIZED.store(true, Ordering::SeqCst);
}

pub fn shutdown_output_logger() {
    let mut state = LOGGER_STATE.write().unwrap();
    if let Some(provider) = &mut state.provider {
        provider.shutdown();
    }
    INITIALIZED.store(false, Ordering::SeqCst);
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

    let sanitized_msg = sanitize(&truncated_msg);

    if let Ok(state) = LOGGER_STATE.read() {
        if let Some(provider) = &state.provider {
            match level {
                LogLevel::Debug => provider.debug(tag, sanitized_msg),
                LogLevel::Info => provider.info(tag, sanitized_msg),
                LogLevel::Warn => provider.warn(tag, sanitized_msg),
                LogLevel::Error => provider.error(tag, sanitized_msg),
                _ => {}
            }
            return;
        }
    }

    if let Some(level) = level.to_third_party_level() {
        let mut target = String::from("Statsig::");
        target += tag;

        match level {
            Level::Debug => debug!(target: target.as_str(), "{}", sanitized_msg),
            Level::Info => info!(target: target.as_str(), "{}", sanitized_msg),
            Level::Warn => warn!(target: target.as_str(), "{}", sanitized_msg),
            Level::Error => error!(target: target.as_str(), "{}", sanitized_msg),
            _ => {}
        };
    }
}

fn sanitize(input: &str) -> String {
    input
        .split("secret-")
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.to_string()
            } else {
                let (key, rest) =
                    part.split_at(part.chars().take_while(|c| c.is_alphanumeric()).count());
                let sanitized_key = if key.len() > 5 {
                    format!("{}*****{}", &key[..5], rest)
                } else {
                    format!("{}*****{}", key, rest)
                };
                format!("secret-{}", sanitized_key)
            }
        })
        .collect()
}

pub fn has_valid_log_level(level: &LogLevel) -> bool {
    let state = LOGGER_STATE.read().unwrap();
    let current_level = &state.level;
    level.to_number() <= current_level.to_number()
}

#[macro_export]
macro_rules! log_d {
  ($tag:expr, $($arg:tt)*) => {
        {
            let level = $crate::output_logger::LogLevel::Debug;
            if $crate::output_logger::has_valid_log_level(&level) {
                $crate::output_logger::log_message($tag, level, format!($($arg)*));
            }
        }
    }
}

#[macro_export]
macro_rules! log_i {
  ($tag:expr, $($arg:tt)*) => {
        {
            let level = $crate::output_logger::LogLevel::Info;
            if $crate::output_logger::has_valid_log_level(&level) {
                $crate::output_logger::log_message($tag, level, format!($($arg)*));
            }
        }
    }
}

#[macro_export]
macro_rules! log_w {
  ($tag:expr, $($arg:tt)*) => {
        {
            let level = $crate::output_logger::LogLevel::Warn;
            if $crate::output_logger::has_valid_log_level(&level) {
                $crate::output_logger::log_message($tag, level, format!($($arg)*));
            }
        }
    }
}

#[macro_export]
macro_rules! log_e {
  ($tag:expr, $($arg:tt)*) => {
        {
            let level = $crate::output_logger::LogLevel::Error;
            if $crate::output_logger::has_valid_log_level(&level) {
                $crate::output_logger::log_message($tag, level, format!($($arg)*));
            }
        }
    }
}

#[macro_export]
macro_rules! log_error_to_statsig_and_console {
    ($ops_stats:expr, $tag:expr, $err:expr) => {
        let event = ErrorBoundaryEvent {
            bypass_dedupe: false,
            info: $err.clone(),
            tag: $tag.to_string(),
            extra: None,
            dedupe_key: None,
        };
        $ops_stats.log_error(event);

        $crate::output_logger::log_message(
            &$tag,
            $crate::output_logger::LogLevel::Error,
            $err.to_string(),
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_sanitize_url_for_logging() {
        let test_cases = HashMap::from(
            [
                ("https://api.statsigcdn.com/v2/download_config_specs/secret-jadkfjalkjnsdlvcnjsdfaf.json", "https://api.statsigcdn.com/v2/download_config_specs/secret-jadkf*****.json"),
                ("https://api.statsigcdn.com/v1/log_event/","https://api.statsigcdn.com/v1/log_event/"),
                ("https://api.statsigcdn.com/v2/download_config_specs/secret-jadkfjalkjnsdlvcnjsdfaf.json?sinceTime=1", "https://api.statsigcdn.com/v2/download_config_specs/secret-jadkf*****.json?sinceTime=1"),
            ]
        );
        for (before, expected) in test_cases {
            let sanitized = sanitize(before);
            assert!(sanitized == expected);
        }
    }

    #[test]
    fn test_multiple_secrets() {
        let input = "Multiple secrets: secret-key1 and secret-key2";
        let sanitized = sanitize(input);
        assert_eq!(
            sanitized,
            "Multiple secrets: secret-key1***** and secret-key2*****"
        );
    }

    #[test]
    fn test_short_secret() {
        let input = "Short secret: secret-a";
        let sanitized = sanitize(input);
        assert_eq!(sanitized, "Short secret: secret-a*****");
    }
}
