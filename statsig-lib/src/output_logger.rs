#[macro_export]
macro_rules! log_d {
  ($($arg:tt)*) => {
        log::debug!("[Statsig]: {}", format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_i {
  ($($arg:tt)*) => {
        log::info!("[Statsig]: {}", format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_w {
  ($($arg:tt)*) => {
        log::warn!("[Statsig]: {}", format!($($arg)*))
    }
}

#[macro_export]
macro_rules! log_e {
  ($($arg:tt)*) => {
        log::error!("[Statsig]: {}", format!($($arg)*))
    }
}
