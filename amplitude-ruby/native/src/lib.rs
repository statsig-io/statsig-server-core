use amplitude_rust::{Amplitude, BaseEvent, Config, EventOptions, Identify, JsonObject};
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
    configuration: Config,
}

#[derive(Deserialize)]
struct IdentifyRequest {
    operations: Identify,
    options: EventOptions,
    event_properties: Option<JsonObject>,
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
        let amplitude = Amplitude::new(&config.api_key, Some(config.configuration))
            .map_err(|error| error.to_string())?;
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
    unsafe {
        call_with_client(client, |client| {
            let mut event: BaseEvent = read_json(event_json)?;
            event.library.get_or_insert_with(|| {
                concat!("amplitude-ruby-core/", env!("CARGO_PKG_VERSION")).into()
            });
            client
                .runtime
                .block_on(client.amplitude.track(event))
                .map_err(|error| error.to_string())
        })
    }
}

#[no_mangle]
unsafe extern "C" fn amplitude_client_identify(
    client: *const Client,
    request_json: *const c_char,
) -> *mut c_char {
    unsafe {
        call_with_client(client, |client| {
            let request: IdentifyRequest = read_json(request_json)?;
            client
                .runtime
                .block_on(client.amplitude.identify(
                    request.operations,
                    request.options,
                    request.event_properties,
                ))
                .map_err(|error| error.to_string())
        })
    }
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

unsafe fn call_with_client(
    client: *const Client,
    call: impl FnOnce(&Client) -> Result<(), String>,
) -> *mut c_char {
    // The call borrows the client and owns the inputs for the duration of the request.
    ffi_result(AssertUnwindSafe(|| {
        let client = unsafe { client.as_ref() }.ok_or("Amplitude client is missing")?;
        call(client)
    }))
    .err()
    .unwrap_or(ptr::null_mut())
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
