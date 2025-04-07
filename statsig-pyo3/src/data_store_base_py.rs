use async_trait::async_trait;
use pyo3::{pyclass, pymethods, FromPyObject, PyObject, Python};
use pyo3_stub_gen::derive::*;
use statsig_rust::{
    data_store_interface::{DataStoreResponse, DataStoreTrait, RequestPath},
    log_e, StatsigErr,
};

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
        Python::with_gil(|py| {
            if let Some(initialize_fn) = &self.initialize_fn {
                if let Err(e) = initialize_fn.call(py, (), None) {
                    log_e!(TAG, "Failed to call DataStoreBasePy.initialize: {:?}", e);
                    return Err(StatsigErr::DataStoreFailure(
                        "Failed to initialize DataStoreBasePy".to_string(),
                    ));
                }
            }
            Ok(())
        })
    }

    async fn shutdown(&self) -> Result<(), StatsigErr> {
        Python::with_gil(|py| {
            if let Some(shutdown_fn) = &self.shutdown_fn {
                if let Err(e) = shutdown_fn.call(py, (), None) {
                    log_e!(TAG, "Failed to call DataStoreBasePy.shutdown: {:?}", e);
                    return Err(StatsigErr::DataStoreFailure(
                        "Failed to shutdown DataStoreBasePy".to_string(),
                    ));
                }
            }
            Ok(())
        })
    }

    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr> {
        Python::with_gil(|py| {
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
                                match result_attr.extract::<String>(py) {
                                    Ok(s) => Some(s),
                                    Err(_) => None,
                                }
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
        Python::with_gil(|py| {
            if let Some(set_fn) = &self.set_fn {
                let result = set_fn.call(py, (String::from(key), String::from(value), time), None);
                match result {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log_e!(TAG, "Failed to call DataStoreBasePy.set: {:?}", e);
                        Err(StatsigErr::DataStoreFailure(
                            "Failed to set in DataStoreBasePy".to_string(),
                        ))
                    }
                }
            } else {
                Err(StatsigErr::DataStoreFailure(
                    "No 'set' function provided".to_string(),
                ))
            }
        })
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool {
        Python::with_gil(|py| {
            if let Some(support_polling_updates_for_fn) = &self.support_polling_updates_for_fn {
                let result = support_polling_updates_for_fn.call(py, (path.to_string(),), None);
                match result {
                    Ok(value) => value.extract(py).unwrap(),
                    Err(e) => {
                        log_e!(
                            TAG,
                            "Failed to call DataStoreBasePy.support_polling_updates_for: {:?}",
                            e
                        );
                        false
                    }
                }
            } else {
                false
            }
        })
    }
}
