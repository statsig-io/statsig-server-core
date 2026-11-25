use amplitude_rust::{Amplitude, AmplitudeEvent, AmplitudeOptions};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    ffi::{c_char, CStr, CString},
    panic::{catch_unwind, AssertUnwindSafe, UnwindSafe},
    ptr,
};
use tokio::runtime::{Builder, Runtime};

struct Client {
    amplitude: Amplitude,
    runtime: Runtime,
}

#[derive(Deserialize)]
struct ClientConfig {
    api_key: String,
    server_url: Option<String>,
    timeout_ms: Option<u64>,
}

// All exported functions take caller-owned inputs. Returned client and error
// pointers must be passed to their corresponding free function exactly once.
#[no_mangle]
unsafe extern "C" fn amplitude_client_new(
    config_json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut Client {
    if error_out.is_null() {
        return ptr::null_mut();
    }

    let result = ffi_result(|| {
        let config: ClientConfig = unsafe { read_json(config_json) }?;
        let mut options = AmplitudeOptions::default();
        options.server_url = config.server_url.unwrap_or(options.server_url);
        options.timeout_ms = config.timeout_ms.unwrap_or(options.timeout_ms);
        let amplitude =
            Amplitude::new(&config.api_key, Some(options)).map_err(|error| error.to_string())?;
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Box::new(Client { amplitude, runtime }))
    });

    match result {
        Ok(client) => {
            unsafe { *error_out = ptr::null_mut() };
            Box::into_raw(client)
        }
        Err(error) => {
            unsafe { *error_out = error };
            ptr::null_mut()
        }
    }
}

#[no_mangle]
unsafe extern "C" fn amplitude_client_track(
    client: *const Client,
    event_json: *const c_char,
) -> *mut c_char {
    // The call borrows the client and owns the event for the duration of the request.
    ffi_result(AssertUnwindSafe(|| {
        let client = unsafe { client.as_ref() }.ok_or("Amplitude client is missing")?;
        let event: AmplitudeEvent = unsafe { read_json(event_json) }?;
        client
            .runtime
            .block_on(client.amplitude.track(event))
            .map_err(|error| error.to_string())
    }))
    .err()
    .unwrap_or(ptr::null_mut())
}

#[no_mangle]
unsafe extern "C" fn amplitude_client_free(client: *mut Client) {
    if !client.is_null() {
        drop(unsafe { Box::from_raw(client) });
    }
}

#[no_mangle]
unsafe extern "C" fn amplitude_error_free(error: *mut c_char) {
    if !error.is_null() {
        drop(unsafe { CString::from_raw(error) });
    }
}

unsafe fn read_json<T: DeserializeOwned>(value: *const c_char) -> Result<T, String> {
    if value.is_null() {
        return Err("Amplitude JSON input is missing".into());
    }
    let bytes = unsafe { CStr::from_ptr(value) }.to_bytes();
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

fn ffi_result<T>(call: impl FnOnce() -> Result<T, String> + UnwindSafe) -> Result<T, *mut c_char> {
    catch_unwind(call)
        .unwrap_or_else(|_| Err("Amplitude native call panicked".into()))
        .map_err(|error| {
            CString::new(error)
                .unwrap_or_else(|_| CString::new("Amplitude native call failed").unwrap())
                .into_raw()
        })
}
