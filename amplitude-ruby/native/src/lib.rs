use amplitude_rust::{
    Amplitude, AmplitudeError, Config, Event, EventOptions, Identify, JsonObject,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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

#[derive(Serialize)]
struct Completion {
    event: Event,
    status_code: u16,
    message: Option<String>,
}

// Inputs are caller-owned. Returned client pointers and any non-null error or
// completion strings must be passed to their corresponding free function once.
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
unsafe extern "C" fn amplitude_client_track_with_completion(
    client: *const Client,
    event_json: *const c_char,
    completion_out: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        call_with_client(client, completion_out, |client| {
            let mut event: Event = read_json(event_json)?;
            let original = (!completion_out.is_null()).then(|| event.clone());
            event.library.get_or_insert_with(|| {
                concat!("amplitude-ruby-core/", env!("CARGO_PKG_VERSION")).into()
            });
            let result = client.runtime.block_on(client.amplitude.track(event));
            complete(result, original, completion_out)
        })
    }
}

#[no_mangle]
unsafe extern "C" fn amplitude_client_identify_with_completion(
    client: *const Client,
    request_json: *const c_char,
    completion_out: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        call_with_client(client, completion_out, |client| {
            let request: IdentifyRequest = read_json(request_json)?;
            let original = (!completion_out.is_null()).then(|| {
                Event::from_identify(
                    request.operations.clone(),
                    request.options.clone(),
                    request.event_properties.clone(),
                )
                .unwrap_or_else(|_| Event {
                    options: request.options.clone(),
                    event_properties: request.event_properties.clone(),
                    ..Event::new("$identify")
                })
            });
            let result = client.runtime.block_on(client.amplitude.identify(
                request.operations,
                request.options,
                request.event_properties,
            ));
            complete(result, original, completion_out)
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
unsafe extern "C" fn amplitude_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}

unsafe fn complete(
    result: Result<u16, AmplitudeError>,
    event: Option<Event>,
    completion_out: *mut *mut c_char,
) -> Result<(), String> {
    if let Some(event) = event {
        let completion = Completion {
            event,
            status_code: result
                .as_ref()
                .copied()
                .unwrap_or_else(AmplitudeError::status_code),
            message: result.as_ref().err().map(ToString::to_string),
        };
        let json = serde_json::to_vec(&completion).map_err(|error| error.to_string())?;
        let value = CString::new(json).map_err(|error| error.to_string())?;
        unsafe { *completion_out = value.into_raw() };
    }
    result.map(|_| ()).map_err(|error| error.to_string())
}

unsafe fn call_with_client(
    client: *const Client,
    completion_out: *mut *mut c_char,
    call: impl FnOnce(&Client) -> Result<(), String>,
) -> *mut c_char {
    if !completion_out.is_null() {
        unsafe { *completion_out = ptr::null_mut() };
    }
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
