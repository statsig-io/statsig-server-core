use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

use napi::bindgen_prelude::{FnArgs, JsValuesTupleIntoVec};
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::Status;
use napi_derive::napi;
use serde_json::Value;
use statsig_rust::{log_e, log_w, PersistentStorage, StickyValues};

const TAG: &str = "PersistentStorageNapi";
const MAX_RECV_TIMEOUT: Duration = Duration::from_secs(5);

type SaveFnArgs = FnArgs<(String, String, Value)>;
type DeleteFnArgs = FnArgs<(String, String)>;
type TypedJsFunction<Arg, ReturnVal> = Option<ThreadsafeFunction<Arg, ReturnVal, Arg, false>>;

#[napi(object, object_to_js = false, js_name = "PersistentStorage")]
pub struct PersistentStorageNapi {
    #[napi(
        js_name = "load",
        ts_type = "(key: string) => UserPersistedValues | null"
    )]
    pub load_fn: TypedJsFunction<String, Value>,

    #[napi(
        js_name = "save",
        ts_type = "(key: string, config_name: string, data: StickyValues) => void"
    )]
    pub save_fn: TypedJsFunction<SaveFnArgs, ()>,

    #[napi(
        js_name = "delete",
        ts_type = "(key: string, config_name: string) => void"
    )]
    pub delete_fn: TypedJsFunction<DeleteFnArgs, ()>,
}

// ------------------------------------------------------------------------------- [PersistentStorage Trait]

impl PersistentStorage for PersistentStorageNapi {
    fn load(&self, key: String) -> Option<statsig_rust::UserPersistedValues> {
        let fnc = match &self.load_fn {
            Some(f) => f,
            None => {
                log_w!(TAG, "No 'load' function provided");
                return None;
            }
        };

        let (tx, rx) = mpsc::channel::<Value>();

        let status = fnc.call_with_return_value(
            key,
            ThreadsafeFunctionCallMode::Blocking,
            move |result, _| {
                let value = match result {
                    Ok(value) => value,
                    Err(e) => {
                        log_e!(
                            TAG,
                            "Failed to get result from 'load' function {}",
                            e.to_string()
                        );
                        return Err(e);
                    }
                };

                if let Err(e) = tx.send(value) {
                    log_e!(TAG, "Failed to send result to mpsc {}", e.to_string());
                }

                Ok(())
            },
        );

        if status != Status::Ok {
            log_e!(TAG, "Failed to call 'load' function");
            return None;
        }

        let value = match rx.recv_timeout(MAX_RECV_TIMEOUT) {
            Ok(value) => value,
            Err(e) => {
                log_e!(TAG, "Failed to receive result from mpsc {}", e.to_string());
                return None;
            }
        };

        match serde_json::from_value::<HashMap<String, StickyValues>>(value) {
            Ok(sticky_values) => Some(sticky_values),
            Err(e) => {
                log_e!(TAG, "Failed to parse sticky values {}", e.to_string());
                None
            }
        }
    }

    fn save(&self, key: &str, config_name: &str, data: StickyValues) {
        let value = match serde_json::to_value(data) {
            Ok(value) => value,
            Err(e) => {
                log_e!(TAG, "Failed to serialize sticky values {}", e.to_string());
                return;
            }
        };

        call_fn_without_return_value(
            "save",
            &self.save_fn,
            (key.to_string(), config_name.to_string(), value).into(),
        );
    }

    fn delete(&self, key: &str, config_name: &str) {
        call_fn_without_return_value(
            "delete",
            &self.delete_fn,
            (key.to_string(), config_name.to_string()).into(),
        );
    }
}

fn call_fn_without_return_value<Args>(
    fn_name: &str,
    opt_fnc: &TypedJsFunction<Args, ()>,
    args: Args,
) where
    Args: JsValuesTupleIntoVec,
{
    let fnc = match &opt_fnc {
        Some(f) => f,
        None => {
            log_w!(TAG, "No '{}' function provided", fn_name);
            return;
        }
    };

    let status = fnc.call(args, ThreadsafeFunctionCallMode::Blocking);

    if status != Status::Ok {
        log_e!(TAG, "Failed to call '{}' function", fn_name);
    }
}

// ------------------------------------------------------------------------------- [Test Helper]

#[napi(js_name = "__internal__testPersistentStorage")]
pub async fn test_persistent_storage(
    store: PersistentStorageNapi,
    action: String,
    key: Option<String>,
    config_name: Option<String>,
    data: Option<Value>,
) -> Option<HashMap<String, String>> {
    match action.as_str() {
        "load" => store.load("test".to_string()).map(|values| {
            values
                .into_iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        serde_json::to_string(&value).unwrap_or_else(|e| {
                            panic!("Failed to serialize sticky values for key: {} {}", key, e);
                        }),
                    )
                })
                .collect()
        }),
        "save" => {
            let sticky =
                serde_json::from_value::<StickyValues>(data.unwrap()).unwrap_or_else(|e| {
                    panic!(
                        "Failed to deserialize sticky values for key: {} {}",
                        key.clone().unwrap_or_default(),
                        e
                    );
                });
            store.save(key.unwrap().as_str(), config_name.unwrap().as_str(), sticky);
            None
        }
        "delete" => {
            store.delete(key.unwrap().as_str(), config_name.unwrap().as_str());
            None
        }
        _ => None,
    }
}
