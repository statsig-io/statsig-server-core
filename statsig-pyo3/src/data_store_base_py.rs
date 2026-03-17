use async_trait::async_trait;
use pyo3::types::{PyAny, PyAnyMethods, PyBytes, PyModule};
use pyo3::{prelude::Bound, pyclass, pymethods, FromPyObject, Py};
use pyo3_stub_gen::derive::*;
use statsig_rust::{
    data_store_interface::{
        DataStoreBytesResponse, DataStoreResponse, DataStoreTrait, RequestPath,
    },
    log_e, StatsigErr,
};

use crate::safe_gil::SafeGil;

const TAG: &str = "DataStoreBasey";

#[gen_stub_pyclass]
#[pyclass(name = "DataStoreBase", module = "statsig_python_core", subclass)]
#[derive(FromPyObject, Default)]
pub struct DataStoreBasePy {
    initialize_fn: Option<Py<PyAny>>,
    shutdown_fn: Option<Py<PyAny>>,
    get_fn: Option<Py<PyAny>>,
    get_bytes_fn: Option<Py<PyAny>>,
    supports_bytes_fn: Option<Py<PyAny>>,
    set_fn: Option<Py<PyAny>>,
    set_bytes_fn: Option<Py<PyAny>>,
    support_polling_updates_for_fn: Option<Py<PyAny>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl DataStoreBasePy {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DataStoreTrait for DataStoreBasePy {
    async fn initialize(&self) -> Result<(), StatsigErr> {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return Ok(()),
            };

            let initialize_fn = match &self.initialize_fn {
                Some(f) => f,
                None => return Ok(()),
            };

            initialize_fn.as_ref().call0(py).map_err(|e| {
                log_e!(TAG, "Failed to call DataStoreBasePy.initialize: {:?}", e);
                StatsigErr::DataStoreFailure("Failed to initialize DataStoreBasePy".to_string())
            })?;

            Ok(())
        })
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return Ok(()),
            };

            let shutdown_fn = match &self.shutdown_fn {
                Some(f) => f,
                None => return Ok(()),
            };

            shutdown_fn.as_ref().call0(py).map_err(|e| {
                log_e!(TAG, "Failed to call DataStoreBasePy.shutdown: {:?}", e);
                StatsigErr::DataStoreFailure("Failed to shutdown DataStoreBasePy".to_string())
            })?;

            Ok(())
        })
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => {
                    return Err(StatsigErr::DataStoreFailure(
                        "Python interpreter has been shutdown".to_string(),
                    ))
                }
            };

            let get_fn = match &self.get_fn {
                Some(f) => f,
                None => {
                    return Err(StatsigErr::DataStoreFailure(
                        "No 'get' function provided".to_string(),
                    ))
                }
            };

            let result = get_fn.as_ref().call(py, (key.to_string(),), None);

            match result {
                Ok(py_obj) => {
                    let py_obj = py_obj.bind(py);
                    // Manual extraction of fields from Python object
                    let result: Option<String> = match py_obj.getattr("result") {
                        Ok(result_attr) => {
                            if result_attr.is_none() {
                                None
                            } else {
                                extract_to_string(&result_attr)
                            }
                        }
                        Err(_) => None,
                    };

                    let time: Option<u64> = match py_obj.getattr("time") {
                        Ok(time_attr) => {
                            if time_attr.is_none() {
                                None
                            } else {
                                match time_attr.extract::<u64>() {
                                    Ok(t) => Some(t),
                                    Err(_) => match time_attr.extract::<i64>() {
                                        Ok(t) if t >= 0 => Some(t as u64),
                                        Ok(_) => None,
                                        Err(_) => None,
                                    },
                                }
                            }
                        }
                        Err(_) => None,
                    };

                    Ok(DataStoreResponse { result, time })
                }
                Err(e) => Err(StatsigErr::DataStoreFailure(e.to_string())),
            }
        })
    }

    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr> {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => {
                    return Err(StatsigErr::DataStoreFailure(
                        "Python interpreter has been shutdown".to_string(),
                    ))
                }
            };

            let set_fn = match &self.set_fn {
                Some(f) => f,
                None => {
                    return Err(StatsigErr::DataStoreFailure(
                        "No 'set' function provided".to_string(),
                    ))
                }
            };

            set_fn
                .as_ref()
                .call(py, (String::from(key), String::from(value), time), None)
                .map_err(|e| {
                    log_e!(TAG, "Failed to call DataStoreBasePy.set: {:?}", e);
                    StatsigErr::DataStoreFailure("Failed to set in DataStoreBasePy".to_string())
                })?;

            Ok(())
        })
    }

    async fn get_bytes(&self, key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
        if self.get_bytes_fn.is_none() {
            let response = self.get(key).await?;
            return Ok(DataStoreBytesResponse {
                result: response.result.map(|value| value.into_bytes()),
                time: response.time,
            });
        }

        let get_bytes_fn = self.get_bytes_fn.as_ref();
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => {
                    return Err(StatsigErr::DataStoreFailure(
                        "Python interpreter has been shutdown".to_string(),
                    ))
                }
            };

            let get_bytes_fn = match get_bytes_fn {
                Some(f) => f,
                None => {
                    return Err(StatsigErr::DataStoreFailure(
                        "No 'get_bytes' function provided".to_string(),
                    ))
                }
            };

            let result = get_bytes_fn.call(py, (key.to_string(),), None);

            match result {
                Ok(py_obj) => {
                    let py_obj = py_obj.bind(py);
                    let result: Option<Vec<u8>> = match py_obj.getattr("result") {
                        Ok(result_attr) => {
                            if result_attr.is_none() {
                                None
                            } else {
                                result_attr.extract::<Vec<u8>>().ok()
                            }
                        }
                        Err(_) => None,
                    };

                    let time: Option<u64> = match py_obj.getattr("time") {
                        Ok(time_attr) => {
                            if time_attr.is_none() {
                                None
                            } else {
                                match time_attr.extract::<u64>() {
                                    Ok(t) => Some(t),
                                    Err(_) => match time_attr.extract::<i64>() {
                                        Ok(t) if t >= 0 => Some(t as u64),
                                        _ => None,
                                    },
                                }
                            }
                        }
                        Err(_) => None,
                    };

                    Ok(DataStoreBytesResponse { result, time })
                }
                Err(e) => Err(StatsigErr::DataStoreFailure(e.to_string())),
            }
        })
    }

    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        time: Option<u64>,
    ) -> Result<(), StatsigErr> {
        if self.set_bytes_fn.is_none() {
            let value = std::str::from_utf8(value).map_err(|e| {
                StatsigErr::DataStoreFailure(format!("Failed to decode bytes as UTF-8: {e}"))
            })?;
            return self.set(key, value, time).await;
        }

        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => {
                    return Err(StatsigErr::DataStoreFailure(
                        "Python interpreter has been shutdown".to_string(),
                    ))
                }
            };

            let set_bytes_fn = match &self.set_bytes_fn {
                Some(f) => f,
                None => {
                    return Err(StatsigErr::DataStoreFailure(
                        "No 'set_bytes' function provided".to_string(),
                    ))
                }
            };

            let value = PyBytes::new(py, value);
            set_bytes_fn
                .call(py, (String::from(key), value, time), None)
                .map_err(|e| {
                    log_e!(TAG, "Failed to call DataStoreBasePy.set_bytes: {:?}", e);
                    StatsigErr::DataStoreFailure(
                        "Failed to set_bytes in DataStoreBasePy".to_string(),
                    )
                })?;

            Ok(())
        })
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => {
                    return false;
                }
            };

            let support_polling_updates_for_fn = match &self.support_polling_updates_for_fn {
                Some(f) => f,
                None => {
                    return false;
                }
            };

            let result =
                support_polling_updates_for_fn
                    .as_ref()
                    .call(py, (path.to_string(),), None);
            match result {
                Ok(value) => value.extract::<bool>(py).unwrap_or_default(),
                Err(e) => {
                    log_e!(
                        TAG,
                        "Failed to call DataStoreBasePy.support_polling_updates_for: {:?}",
                        e
                    );
                    false
                }
            }
        })
    }

    fn supports_bytes(&self) -> bool {
        let supports_bytes_fn = match &self.supports_bytes_fn {
            Some(f) => f,
            None => return false,
        };

        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => {
                    return false;
                }
            };

            let result = supports_bytes_fn.call(py, (), None);
            match result {
                Ok(value) => value.extract::<bool>(py).unwrap_or_default(),
                Err(_) => false,
            }
        })
    }
}

fn extract_to_string(result_attr: &Bound<'_, PyAny>) -> Option<String> {
    if let Ok(result) = result_attr.extract::<String>() {
        return Some(result);
    }

    let py = result_attr.py();
    let encoded = PyModule::import(py, "json").ok()?;
    let encoded = encoded.call_method1("dumps", (result_attr,)).ok()?;

    if let Ok(result) = encoded.extract::<String>() {
        return Some(result);
    }

    if let Ok(result_str) = result_attr.str() {
        if let Ok(result) = result_str.extract::<String>() {
            return Some(result);
        }
    }

    None
}
