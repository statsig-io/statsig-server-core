use async_trait::async_trait;
use napi::bindgen_prelude::{FnArgs, Promise};
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;

use sigstat::StatsigErr::DataStoreFailure;
use sigstat::{
    data_store_interface::{
        DataStoreResponse as DataStoreResponseActual, DataStoreTrait, RequestPath,
    },
    log_e, StatsigErr,
};

const TAG: &str = "DataStoreNapi";

#[napi(object)]
pub struct DataStoreResponse {
    pub result: Option<String>,
    pub time: Option<i64>,
}

#[napi(object, object_to_js = false)]
pub struct DataStore {
    #[napi(js_name = "initialize", ts_type = "() => Promise<void>")]
    pub initialize_fn: Option<ThreadsafeFunction<(), Promise<()>, (), false>>,

    #[napi(js_name = "shutdown", ts_type = "() => Promise<void>")]
    pub shutdown_fn: Option<ThreadsafeFunction<(), Promise<()>, (), false>>,

    #[napi(
        js_name = "get",
        ts_type = "(key: string) => Promise<DataStoreResponse>"
    )]
    pub get_fn: Option<ThreadsafeFunction<String, Promise<DataStoreResponse>, String, false>>,

    #[napi(
        js_name = "set",
        ts_type = "(key: string, value: string, time?: number) => Promise<void>"
    )]
    pub set_fn: Option<
        ThreadsafeFunction<
            FnArgs<(String, String, Option<i64>)>,
            Promise<()>,
            FnArgs<(String, String, Option<i64>)>,
            false,
        >,
    >,

    #[napi(
        js_name = "supportPollingUpdatesFor",
        ts_type = "(key: string) => Promise<boolean>"
    )]
    pub support_polling_updates_for_fn:
        Option<ThreadsafeFunction<String, Promise<bool>, String, false>>,
}

#[async_trait]
impl DataStoreTrait for DataStore {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        let fnc = match &self.initialize_fn {
            Some(f) => f,
            None => {
                return Err(DataStoreFailure(
                    "No 'initialize' function provided".to_string(),
                ))
            }
        };

        let result = fnc.call_async(()).await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(DataStoreFailure(e.to_string())),
        }
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        let fnc = match &self.shutdown_fn {
            Some(f) => f,
            None => {
                return Err(DataStoreFailure(
                    "No 'shutdown' function provided".to_string(),
                ))
            }
        };

        let result = fnc.call_async(()).await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(DataStoreFailure(e.to_string())),
        }
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponseActual, StatsigErr> {
        let fnc = match &self.get_fn {
            Some(f) => f,
            None => return Err(DataStoreFailure("No 'get' function provided".to_string())),
        };

        let result = match fnc.call_async(key.to_string()).await {
            Ok(result) => result.await,
            Err(err) => return Err(DataStoreFailure(err.to_string())),
        };

        match result {
            Ok(response) => Ok(DataStoreResponseActual {
                time: response.time.map(|t| t as u64),
                result: response.result,
            }),
            Err(err) => Err(DataStoreFailure(err.to_string())),
        }
    }

    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
        let fnc = match &self.set_fn {
            Some(f) => f,
            None => return Err(DataStoreFailure("No 'set' function provided".to_string())),
        };

        let args = (key.into(), value.into(), time.map(|t| t as i64));
        let result = match fnc.call_async(args.into()).await {
            Ok(result) => result.await,
            Err(err) => return Err(DataStoreFailure(err.to_string())),
        };

        result.map_err(|e| DataStoreFailure(e.to_string()))
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        let fnc = match &self.support_polling_updates_for_fn {
            Some(f) => f,
            None => return false,
        };

        let result = match fnc.call_async(path.to_string()).await {
            Ok(result) => result.await,
            Err(err) => {
                log_e!(TAG, "supportPollingUpdatesFor error {}", err);
                return false;
            }
        };

        result.unwrap_or_else(|err| {
            log_e!(TAG, "supportPollingUpdatesFor error {}", err);
            false
        })
    }
}

#[napi(js_name = "__internal__testDataStore")]
pub async fn test_data_store(
    store: DataStore,
    path: String,
    value: String,
) -> (Option<DataStoreResponse>, bool) {
    store.initialize().await.unwrap_or_else(|err| {
        log_e!(TAG, "TEST DataStoreFailure {}", err);
    });

    let get_result = match store.get(&path.to_string()).await {
        Ok(response) => Some(DataStoreResponse {
            time: response.time.map(|t| t as i64),
            result: response.result,
        }),
        Err(err) => {
            log_e!(TAG, "TEST DataStoreFailure {}", err);
            None
        }
    };

    store
        .set(&path.to_string(), &value, None)
        .await
        .unwrap_or_else(|err| {
            log_e!(TAG, "TEST DataStoreFailure {}", err);
        });

    let path = match path.as_str() {
        "/v2/download_config_specs" => RequestPath::RulesetsV2,
        "/v1/download_config_specs" => RequestPath::RulesetsV1,
        "/v1/get_id_lists" => RequestPath::IDListsV1,
        "id_list" => RequestPath::IDList,
        _ => panic!("Invalid request path: {}", path),
    };

    let polling_result = store.support_polling_updates_for(path).await;

    store.shutdown().await.unwrap_or_else(|err| {
        log_e!(TAG, "TEST DataStoreFailure {}", err);
    });

    (get_result, polling_result)
}
