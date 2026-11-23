use std::os::raw::c_char;

use statsig_rust::{
    log_d, log_e, output_logger::OutputLogProvider, unwrap_or_noop, InstanceRegistry,
};

use crate::ffi_utils::c_char_to_string;

const TAG: &str = "OutputLoggerProviderC";

// Level values passed to `log_fn`. They match the numbering used by
// `statsig_rust::output_logger::LogLevel` (None = 0 is never sent).
pub const OUTPUT_LOG_LEVEL_ERROR: u32 = 1;
pub const OUTPUT_LOG_LEVEL_WARN: u32 = 2;
pub const OUTPUT_LOG_LEVEL_INFO: u32 = 3;
pub const OUTPUT_LOG_LEVEL_DEBUG: u32 = 4;

/// Bridges the core `OutputLogProvider` trait to C function pointers.
///
/// `tag_ptr` / `msg_ptr` passed to `log_fn` are borrowed UTF-8 buffers that are
/// NOT null-terminated and are only valid for the duration of the call. The
/// callee must copy them and must not free them.
///
/// Note: the core keeps the active provider in process-global state, so the
/// function pointers must stay callable for the lifetime of the process once
/// the provider has been handed to a `Statsig` instance.
pub struct OutputLoggerProviderC {
    pub init_fn: extern "C" fn(),
    pub log_fn: extern "C" fn(
        level: u32,
        tag_ptr: *const c_char,
        tag_length: u64,
        msg_ptr: *const c_char,
        msg_length: u64,
    ),
    pub shutdown_fn: extern "C" fn(),
}

impl OutputLoggerProviderC {
    fn log(&self, level: u32, tag: &str, msg: &str) {
        // Do not call log_* macros from here: the core holds the logger lock
        // while invoking the provider.
        (self.log_fn)(
            level,
            tag.as_ptr() as *const c_char,
            tag.len() as u64,
            msg.as_ptr() as *const c_char,
            msg.len() as u64,
        );
    }
}

// -------------------------------------------------------------------- [ Trait Impl ]

impl OutputLogProvider for OutputLoggerProviderC {
    fn initialize(&self) {
        (self.init_fn)();
    }

    fn debug(&self, tag: &str, msg: String) {
        self.log(OUTPUT_LOG_LEVEL_DEBUG, tag, &msg);
    }

    fn info(&self, tag: &str, msg: String) {
        self.log(OUTPUT_LOG_LEVEL_INFO, tag, &msg);
    }

    fn warn(&self, tag: &str, msg: String) {
        self.log(OUTPUT_LOG_LEVEL_WARN, tag, &msg);
    }

    fn error(&self, tag: &str, msg: String) {
        self.log(OUTPUT_LOG_LEVEL_ERROR, tag, &msg);
    }

    fn shutdown(&self) {
        (self.shutdown_fn)();
    }
}

// -------------------------------------------------------------------- [ C Extern ]

#[no_mangle]
pub extern "C" fn output_logger_provider_create(
    init_fn: extern "C" fn(),
    log_fn: extern "C" fn(
        level: u32,
        tag_ptr: *const c_char,
        tag_length: u64,
        msg_ptr: *const c_char,
        msg_length: u64,
    ),
    shutdown_fn: extern "C" fn(),
) -> u64 {
    InstanceRegistry::register(OutputLoggerProviderC {
        init_fn,
        log_fn,
        shutdown_fn,
    })
    .unwrap_or_else(|| {
        log_e!(TAG, "Failed to create OutputLoggerProviderC");
        0
    })
}

#[no_mangle]
pub extern "C" fn output_logger_provider_release(provider_ref: u64) {
    InstanceRegistry::remove(&provider_ref);
}

#[no_mangle]
pub extern "C" fn __internal__test_output_logger_provider(
    provider_ref: u64,
    action: *const c_char,
    tag: *const c_char,
    message: *const c_char,
) {
    let provider = match InstanceRegistry::get::<OutputLoggerProviderC>(&provider_ref) {
        Some(provider) => provider,
        None => {
            log_d!(TAG, "TEST OutputLoggerProvider reference not found");
            return;
        }
    };

    let action = unwrap_or_noop!(c_char_to_string(action));
    let tag = c_char_to_string(tag).unwrap_or_default();
    let message = c_char_to_string(message).unwrap_or_default();

    match action.as_str() {
        "init" => provider.initialize(),
        "debug" => provider.debug(&tag, message),
        "info" => provider.info(&tag, message),
        "warn" => provider.warn(&tag, message),
        "error" => provider.error(&tag, message),
        "shutdown" => provider.shutdown(),
        _ => panic!("Invalid action: {action}"),
    }
}

// -------------------------------------------------------------------- [ Tests ]

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::sync::Mutex;

    lazy_static::lazy_static! {
        static ref CALLS: Mutex<Vec<(String, u32, String, String)>> = Mutex::new(Vec::new());
    }

    extern "C" fn init_cb() {
        CALLS
            .lock()
            .unwrap()
            .push(("init".into(), 0, String::new(), String::new()));
    }

    extern "C" fn log_cb(
        level: u32,
        tag_ptr: *const c_char,
        tag_length: u64,
        msg_ptr: *const c_char,
        msg_length: u64,
    ) {
        let read = |ptr: *const c_char, len: u64| unsafe {
            let bytes = std::slice::from_raw_parts(ptr as *const u8, len as usize);
            String::from_utf8(bytes.to_vec()).unwrap()
        };
        CALLS.lock().unwrap().push((
            "log".into(),
            level,
            read(tag_ptr, tag_length),
            read(msg_ptr, msg_length),
        ));
    }

    extern "C" fn shutdown_cb() {
        CALLS
            .lock()
            .unwrap()
            .push(("shutdown".into(), 0, String::new(), String::new()));
    }

    fn call(provider_ref: u64, action: &str, tag: &str, msg: &str) {
        let action = CString::new(action).unwrap();
        let tag = CString::new(tag).unwrap();
        let msg = CString::new(msg).unwrap();
        __internal__test_output_logger_provider(
            provider_ref,
            action.as_ptr(),
            tag.as_ptr(),
            msg.as_ptr(),
        );
    }

    #[test]
    fn test_output_logger_provider_bridges_all_calls() {
        let provider_ref = output_logger_provider_create(init_cb, log_cb, shutdown_cb);
        assert_ne!(provider_ref, 0);

        call(provider_ref, "init", "", "");
        call(provider_ref, "debug", "TagD", "debug msg");
        call(provider_ref, "info", "TagI", "info msg");
        call(provider_ref, "warn", "TagW", "warn msg");
        call(provider_ref, "error", "TagE", "erröр ✓ msg");
        call(provider_ref, "shutdown", "", "");

        output_logger_provider_release(provider_ref);
        assert!(InstanceRegistry::get::<OutputLoggerProviderC>(&provider_ref).is_none());

        let calls = CALLS.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec![
                ("init".into(), 0, "".into(), "".into()),
                (
                    "log".into(),
                    OUTPUT_LOG_LEVEL_DEBUG,
                    "TagD".into(),
                    "debug msg".into()
                ),
                (
                    "log".into(),
                    OUTPUT_LOG_LEVEL_INFO,
                    "TagI".into(),
                    "info msg".into()
                ),
                (
                    "log".into(),
                    OUTPUT_LOG_LEVEL_WARN,
                    "TagW".into(),
                    "warn msg".into()
                ),
                (
                    "log".into(),
                    OUTPUT_LOG_LEVEL_ERROR,
                    "TagE".into(),
                    "erröр ✓ msg".into()
                ),
                ("shutdown".into(), 0, "".into(), "".into()),
            ]
        );
    }

    extern "C" fn noop_init() {}
    extern "C" fn noop_log(_: u32, _: *const c_char, _: u64, _: *const c_char, _: u64) {}
    extern "C" fn noop_shutdown() {}

    #[test]
    fn test_options_from_data_resolves_output_logger_provider_and_level() {
        use crate::statsig_options_c::{statsig_options_create_from_data, statsig_options_release};
        use statsig_rust::StatsigOptions;

        let provider_ref = output_logger_provider_create(noop_init, noop_log, noop_shutdown);
        let json = format!(
            r#"{{"output_logger_provider_ref":{provider_ref},"output_log_level":"debug"}}"#
        );
        let json = CString::new(json).unwrap();

        let options_ref = statsig_options_create_from_data(json.as_ptr());
        let options = InstanceRegistry::get::<StatsigOptions>(&options_ref).unwrap();

        assert!(options.output_logger_provider.is_some());
        assert!(matches!(
            options.output_log_level,
            Some(statsig_rust::output_logger::LogLevel::Debug)
        ));

        // Unknown refs are ignored rather than failing options creation.
        let json = CString::new(r#"{"output_logger_provider_ref":123}"#).unwrap();
        let bad_options_ref = statsig_options_create_from_data(json.as_ptr());
        let bad_options = InstanceRegistry::get::<StatsigOptions>(&bad_options_ref).unwrap();
        assert!(bad_options.output_logger_provider.is_none());

        statsig_options_release(options_ref);
        statsig_options_release(bad_options_ref);
        output_logger_provider_release(provider_ref);
    }
}
