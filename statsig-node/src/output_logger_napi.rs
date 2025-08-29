use napi_derive::napi;
use statsig_rust::output_logger::OutputLogProvider as OutputLogProviderActual;

use napi::bindgen_prelude::FnArgs;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};

type LogFnArgs = FnArgs<(String, String)>;
type LogFn = ThreadsafeFunction<LogFnArgs, (), LogFnArgs, false>;

#[napi(object, object_to_js = false)]
pub struct OutputLoggerProvider {
    #[napi(js_name = "initialize", ts_type = "() => void")]
    pub initialize_fn: Option<ThreadsafeFunction<()>>,

    #[napi(js_name = "debug", ts_type = "(tag: string, message: string) => void")]
    pub debug_fn: Option<LogFn>,

    #[napi(js_name = "info", ts_type = "(tag: string, message: string) => void")]
    pub info_fn: Option<LogFn>,

    #[napi(js_name = "warn", ts_type = "(tag: string, message: string) => void")]
    pub warn_fn: Option<LogFn>,

    #[napi(js_name = "error", ts_type = "(tag: string, message: string) => void")]
    pub error_fn: Option<LogFn>,

    #[napi(js_name = "shutdown", ts_type = "() => void")]
    pub shutdown_fn: Option<ThreadsafeFunction<()>>,
}

impl OutputLogProviderActual for OutputLoggerProvider {
    fn initialize(&self) {
        let fnc = match &self.initialize_fn {
            Some(f) => f,
            None => {
                eprintln!("[OutputLoggerNapi] No 'initialize' function provided");
                return;
            }
        };

        fnc.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
    }

    fn debug(&self, tag: &str, msg: String) {
        let fnc = match &self.debug_fn {
            Some(f) => f,
            None => return,
        };

        let args = (tag.to_string(), msg).into();
        fnc.call(args, ThreadsafeFunctionCallMode::Blocking);
    }

    fn info(&self, tag: &str, msg: String) {
        let fnc = match &self.info_fn {
            Some(f) => f,
            None => return,
        };

        let args = (tag.to_string(), msg).into();
        fnc.call(args, ThreadsafeFunctionCallMode::Blocking);
    }

    fn warn(&self, tag: &str, msg: String) {
        let fnc = match &self.warn_fn {
            Some(f) => f,
            None => return,
        };

        let args = (tag.to_string(), msg).into();
        fnc.call(args, ThreadsafeFunctionCallMode::Blocking);
    }

    fn error(&self, tag: &str, msg: String) {
        let fnc = match &self.error_fn {
            Some(f) => f,
            None => return,
        };

        let args = (tag.to_string(), msg).into();
        fnc.call(args, ThreadsafeFunctionCallMode::Blocking);
    }

    fn shutdown(&self) {
        let fnc = match &self.shutdown_fn {
            Some(f) => f,
            None => {
                eprintln!("[OutputLoggerNapi] No 'shutdown' function provided");
                return;
            }
        };

        fnc.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
    }
}

#[napi(js_name = "__internal__testOutputLogger")]
pub async fn test_output_logger(
    logger: OutputLoggerProvider,
    action: String,
    tag: Option<String>,
    message: Option<String>,
) {
    match action.as_str() {
        "initialize" => logger.initialize(),
        "debug" => logger.debug(
            tag.as_deref().unwrap_or("test"),
            message.unwrap_or_default(),
        ),
        "info" => logger.info(
            tag.as_deref().unwrap_or("test"),
            message.unwrap_or_default(),
        ),
        "warn" => logger.warn(
            tag.as_deref().unwrap_or("test"),
            message.unwrap_or_default(),
        ),
        "error" => logger.error(
            tag.as_deref().unwrap_or("test"),
            message.unwrap_or_default(),
        ),
        "shutdown" => logger.shutdown(),
        _ => eprintln!("[OutputLoggerNapi] Invalid action: {action}"),
    }
}
