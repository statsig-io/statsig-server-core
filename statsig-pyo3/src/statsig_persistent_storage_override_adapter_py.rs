use pyo3::{
    prelude::*,
    types::{PyDict, PyList},
};
use pyo3_stub_gen::derive::*;
use statsig_rust::{
    interned_string::InternedString, log_d, log_e, log_w,
    specs_response::explicit_params::ExplicitParameters, PersistentStorage, SecondaryExposure,
    StickyValues as StickyValuesActual, UserPersistedValues as UserPersistedValuesActual,
};
use std::collections::HashMap;

use crate::{
    pyo_utils::{
        get_string_from_py_dict_throw_on_none, map_to_py_dict, py_dict_to_json_value_map,
        py_list_to_list,
    },
    safe_gil::SafeGil,
};
const TAG: &str = stringify!(PersistentStorageOverrideAdapterPY);

macro_rules! extract_or_default {
    ($dict:expr, $key:expr, $default:expr) => {{
        match $dict.get_item($key)? {
            Some(v) => v.extract()?,
            None => $default,
        }
    }};
}

pub fn convert_dict_to_user_persisted_values(
    py: Python,
    py_dict: Py<PyDict>,
    config_name: &str,
) -> PyResult<UserPersistedValuesActual> {
    let py_dict_ref = py_dict.bind(py);
    let sticky_value_py = py_dict_ref.get_item(config_name)?;
    let mut user_persisted_value = HashMap::new();
    match sticky_value_py {
        Some(value_py_unwrapped) => {
            let py_dict = value_py_unwrapped.extract::<Bound<PyDict>>()?;
            let value = extract_or_default!(py_dict, "value", false);
            let json_value: Option<Bound<PyDict>> =
                extract_or_default!(py_dict, "json_value", None);
            let rule_id: Option<String> = extract_or_default!(py_dict, "rule_id", None);
            let group_name: Option<String> = extract_or_default!(py_dict, "group_name", None);
            let secondary_exposures: Bound<'_, PyList> =
                extract_or_default!(py_dict, "secondary_exposures", PyList::empty(py));
            let undelegated_secondary_exposures: Bound<'_, PyList> = extract_or_default!(
                py_dict,
                "undelegated_secondary_exposures",
                PyList::empty(py)
            );
            let config_delegate: Option<String> =
                extract_or_default!(py_dict, "config_delegate", None);
            let explicit_parameters: Option<Bound<'_, PyList>> =
                extract_or_default!(py_dict, "explicit_parameters", None);
            let time: Option<u64> = extract_or_default!(py_dict, "time", None);
            let config_version: Option<u32> = extract_or_default!(py_dict, "config_version", None);
            let converted_explicit_parameters = match explicit_parameters {
                Some(s) => Some(py_list_to_list(&s)?),
                None => None,
            };

            let sticky_value = StickyValuesActual {
                value,
                rule_id: rule_id.map(InternedString::from_string),
                group_name,
                config_delegate: config_delegate.map(InternedString::from_string),
                time,
                json_value: json_value.map(|v| py_dict_to_json_value_map(&v)),
                secondary_exposures: convert_py_lists_to_secondary_exposures(&secondary_exposures)?,
                undelegated_secondary_exposures: Some(convert_py_lists_to_secondary_exposures(
                    &undelegated_secondary_exposures,
                )?),
                explicit_parameters: converted_explicit_parameters
                    .map(ExplicitParameters::from_vec),
                config_version,
            };
            user_persisted_value.insert(config_name.to_string(), sticky_value);
            Ok(user_persisted_value)
        }
        None => Ok(HashMap::new()),
    }
}

fn convert_stick_value_to_py_obj(
    py: Python,
    sticky_values: StickyValuesActual,
) -> PyResult<Bound<PyAny>> {
    let module = py.import("statsig_python_core")?;
    let sticky_value_class = module.getattr("StickyValues")?;
    let py_dict = PyDict::new(py);
    py_dict.set_item("value", sticky_values.value)?;
    let json_value = sticky_values
        .json_value
        .as_ref()
        .map(|v| map_to_py_dict(py, v));
    py_dict.set_item("json_value", json_value)?;
    py_dict.set_item(
        "rule_id",
        sticky_values
            .rule_id
            .map(|r| r.unperformant_to_string())
            .unwrap_or_default(),
    )?;
    py_dict.set_item("group_name", sticky_values.group_name.clone())?;
    let secondary_exposures: Vec<Bound<PyDict>> =
        convert_secondary_exposures_to_py_dict(py, sticky_values.secondary_exposures)?;
    py_dict.set_item("secondary_exposures", secondary_exposures)?;
    py_dict.set_item(
        "explicit_parameters",
        sticky_values
            .explicit_parameters
            .as_ref()
            .map(|p| p.unperformant_to_vec()),
    )?;
    py_dict.set_item(
        "config_delegate",
        sticky_values
            .config_delegate
            .as_ref()
            .map(|d| d.unperformant_to_string()),
    )?;
    let undelegated_secondary_exposures = match sticky_values.undelegated_secondary_exposures {
        Some(exp) => {
            let dict = convert_secondary_exposures_to_py_dict(py, exp)?;
            Some(dict)
        }
        None => None,
    };
    py_dict.set_item(
        "undelegated_secondary_exposures",
        undelegated_secondary_exposures,
    )?;
    py_dict.set_item("time", sticky_values.time.unwrap_or(0))?;
    py_dict.set_item("config_version", sticky_values.config_version.unwrap_or(0))?;
    let sticky_value_instance = sticky_value_class.call((), Some(&py_dict))?;
    Ok(sticky_value_instance)
}

fn convert_secondary_exposures_to_py_dict(
    py: Python,
    exposures: Vec<SecondaryExposure>,
) -> PyResult<Vec<Bound<PyDict>>> {
    exposures
        .iter()
        .map(|exposure: &SecondaryExposure| {
            let secondary_exposure_dict = PyDict::new(py);
            secondary_exposure_dict.set_item("gate", exposure.gate.unperformant_to_string())?;
            secondary_exposure_dict
                .set_item("gateValue", exposure.gate_value.clone().to_string())?;
            secondary_exposure_dict
                .set_item("ruleID", exposure.rule_id.unperformant_to_string())?;
            Ok(secondary_exposure_dict)
        })
        .collect()
}

fn convert_py_lists_to_secondary_exposures(
    py_list: &Bound<PyList>,
) -> PyResult<Vec<SecondaryExposure>> {
    let mut exposures = Vec::new();
    for item in py_list {
        match item.extract::<Bound<PyDict>>() {
            Ok(e) => {
                exposures.push(convert_py_dict_to_secondary_exposure(&e)?);
            }
            Err(e) => return Err(e),
        }
    }

    Ok(exposures)
}

fn convert_py_dict_to_secondary_exposure(py_dict: &Bound<PyDict>) -> PyResult<SecondaryExposure> {
    let gate = get_string_from_py_dict_throw_on_none(py_dict, "gate")?;
    let gate_value = get_string_from_py_dict_throw_on_none(py_dict, "gateValue")?;
    let rule_id = get_string_from_py_dict_throw_on_none(py_dict, "ruleID")?;
    Ok(SecondaryExposure {
        gate: InternedString::from_string(gate),
        gate_value: InternedString::from_string(gate_value),
        rule_id: InternedString::from_string(rule_id),
    })
}

#[gen_stub_pyclass]
#[pyclass(
    name = "PersistentStorageBaseClass",
    module = "statsig_python_core",
    subclass
)]
#[derive(FromPyObject, Default)]
pub struct PersistentStorageBasePy {
    pub load_fn: Option<PyObject>,
    pub save_fn: Option<PyObject>,
    pub delete_fn: Option<PyObject>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PersistentStorageBasePy {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct StatsigPersistentStorageOverrideAdapter {
    inner: PersistentStorageBasePy,
}

impl StatsigPersistentStorageOverrideAdapter {
    pub fn new(inner: PersistentStorageBasePy) -> Self {
        Self { inner }
    }
}

impl PersistentStorage for StatsigPersistentStorageOverrideAdapter {
    // Load function will be called in python side for now
    fn load(&self, key: String) -> Option<UserPersistedValuesActual> {
        match self.inner.load_fn.as_ref() {
            Some(func) => {
                SafeGil::run(|py| {
                    let py = match py {
                        Some(py) => py,
                        None => return,
                    };

                    match func.call(py, (key,), None) {
                        Ok(_) => {
                            // No-op
                            log_w!(TAG, "Calling persistent storage load in rust side. No-op")
                        }
                        Err(e) => log_e!(TAG, "Failed to load from persistent storage: {:?}", e),
                    }
                });
            }
            None => {
                log_e!(
                    TAG,
                    "Failed to load from persistent storage: load function is not defined",
                )
            }
        }
        None
    }

    fn save(&self, key: &str, config_name: &str, data: StickyValuesActual) {
        let save_func = match self.inner.save_fn.as_ref() {
            Some(func) => func,
            None => {
                log_e!(
                    TAG,
                    "Failed to save from persistent storage: save function is not defined",
                );
                return;
            }
        };

        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return,
            };

            match convert_stick_value_to_py_obj(py, data) {
                Ok(sticky_value_py) => {
                    match save_func.call(py, (key, config_name, sticky_value_py.as_any()), None) {
                        Ok(_) => log_d!(TAG, "Save persistent storage"),
                        Err(e) => log_e!(TAG, "Failed to save from persistent storage: {:?}", e),
                    }
                }
                Err(e) => log_e!(TAG, "Failed to save from persistent storage: {:?}", e),
            }
        });
    }

    fn delete(&self, key: &str, config_name: &str) {
        let delete_func = match self.inner.delete_fn.as_ref() {
            Some(func) => func,
            None => {
                log_e!(
                    TAG,
                    "Failed to delete from persistent storage: delete function is not defined",
                );
                return;
            }
        };

        SafeGil::run(|py| {
            let py = match py {
                Some(py) => py,
                None => return,
            };

            match delete_func.call(py, (key, config_name), None) {
                Ok(_) => log_d!(TAG, "Delete persistent storage"),
                Err(e) => log_e!(TAG, "Failed to delete from persistent storage: {:?}", e),
            }
        });
    }
}
