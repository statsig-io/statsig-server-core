use async_trait::async_trait;
use pyo3::{pyclass, pymethods, FromPyObject, PyObject};
use pyo3_stub_gen::derive::*;
use statsig_rust::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    log_e, StatsigErr,
};

use crate::safe_gil::SafeGil;

const TAG: &str = "DataStoreBasey";

#[gen_stub_pyclass]
#[pyclass(name = "DataStoreBase", subclass)]
#[derive(FromPyObject, Default)]
pub struct DataStoreBasePy {
    initialize_fn: Option<PyObject>,
    shutdown_fn: Option<PyObject>,
    get_fn: Option<PyObject>,
    set_fn: Option<PyObject>,
    support_polling_updates_for_fn: Option<PyObject>,
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

            initialize_fn.call(py, (), None).map_err(|e| {
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

            shutdown_fn.call(py, (), None).map_err(|e| {
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

            let result = get_fn.call(py, (key.to_string(),), None);

            match result {
                Ok(py_obj) => {
                    // Manual extraction of fields from Python object
                    let result: Option<String> = match py_obj.getattr(py, "result") {
                        Ok(result_attr) => {
                            if result_attr.is_none(py) {
                                None
                            } else {
                                result_attr.extract::<String>(py).ok()
                            }
                        }
                        Err(_) => None,
                    };

                    let time: Option<u64> = match py_obj.getattr(py, "time") {
                        Ok(time_attr) => {
                            if time_attr.is_none(py) {
                                None
                            } else {
                                match time_attr.extract::<u64>(py) {
                                    Ok(t) => Some(t),
                                    Err(_) => match time_attr.extract::<i64>(py) {
                                        Ok(t) if t >= 0 => Some(t as u64),
                                        _ => None,
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
                .call(py, (String::from(key), String::from(value), time), None)
                .map_err(|e| {
                    log_e!(TAG, "Failed to call DataStoreBasePy.set: {:?}", e);
                    StatsigErr::DataStoreFailure("Failed to set in DataStoreBasePy".to_string())
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

            let result = support_polling_updates_for_fn.call(py, (path.to_string(),), None);
            match result {
                Ok(value) => value.extract(py).unwrap_or_default(),
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
}
