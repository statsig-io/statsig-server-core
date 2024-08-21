use log::{Level, LevelFilter, Log, Metadata, Record};
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::JsFunction;
use napi_derive::napi;

#[napi]
pub enum LogLevel {
  None = 0,
  Error,
  Warn,
  Info,
  Debug,
}

impl LogLevel {
  fn as_filter(&self) -> LevelFilter {
    match self {
      LogLevel::None => LevelFilter::Off,
      LogLevel::Error => LevelFilter::Error,
      LogLevel::Warn => LevelFilter::Warn,
      LogLevel::Info => LevelFilter::Info,
      LogLevel::Debug => LevelFilter::Debug,
    }
  }
}

#[napi]
pub fn console_logger_init(
  log_level: LogLevel,
  #[napi(ts_arg_type = "(message?: any, ...optionalParams: any[]) => void")] debug_log: JsFunction,
  #[napi(ts_arg_type = "(message?: any, ...optionalParams: any[]) => void")] info_log: JsFunction,
  #[napi(ts_arg_type = "(message?: any, ...optionalParams: any[]) => void")] warn_log: JsFunction,
  #[napi(ts_arg_type = "(message?: any, ...optionalParams: any[]) => void")] error_log: JsFunction,
) -> Option<String> {
  match NodeConsoleLogger::boot(
    log_level.as_filter(),
    debug_log,
    info_log,
    warn_log,
    error_log,
  ) {
    Ok(_) => None,
    Err(e) => Some(format!("Failed to init logger - {}", e)),
  }
}

pub(crate) struct NodeConsoleLogger {
  debug_log: ThreadsafeFunction<String>,
  info_log: ThreadsafeFunction<String>,
  warn_log: ThreadsafeFunction<String>,
  error_log: ThreadsafeFunction<String>,
}

impl NodeConsoleLogger {
  pub fn boot(
    log_level: LevelFilter,
    debug_log: JsFunction,
    info_log: JsFunction,
    warn_log: JsFunction,
    error_log: JsFunction,
  ) -> Result<(), String> {
    let debug_tsfn = create_thread_safe_log_func(debug_log)?;
    let info_tsfn = create_thread_safe_log_func(info_log)?;
    let warn_tsfn = create_thread_safe_log_func(warn_log)?;
    let error_tsfn = create_thread_safe_log_func(error_log)?;

    let inst = NodeConsoleLogger {
      debug_log: debug_tsfn,
      info_log: info_tsfn,
      warn_log: warn_tsfn,
      error_log: error_tsfn,
    };

    match log::set_boxed_logger(Box::new(inst)).map(|()| log::set_max_level(log_level)) {
      Ok(_) => Ok(()),
      Err(e) => Err(format!("{}", e)),
    }
  }
}

impl Log for NodeConsoleLogger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    metadata.level() <= Level::Debug
  }

  fn log(&self, record: &Record) {
    if self.enabled(record.metadata()) {
      let msg = format!("{}", record.args());

      let _ = match record.level() {
        Level::Error => self
          .error_log
          .call(Ok(msg), ThreadsafeFunctionCallMode::NonBlocking),
        Level::Warn => self
          .warn_log
          .call(Ok(msg), ThreadsafeFunctionCallMode::NonBlocking),
        Level::Info => self
          .info_log
          .call(Ok(msg), ThreadsafeFunctionCallMode::NonBlocking),
        Level::Debug => self
          .debug_log
          .call(Ok(msg), ThreadsafeFunctionCallMode::NonBlocking),
        _ => return,
      };
    }
  }

  fn flush(&self) {}
}

fn create_thread_safe_log_func(js_log: JsFunction) -> Result<ThreadsafeFunction<String>, String> {
  js_log
    .create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))
    .map_err(|e| format!("Error creating ThreadsafeFunction<String> {}", e))
}
