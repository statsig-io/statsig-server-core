use pyo3::exceptions::PyValueError;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyList, PyModule};
use pyo3::{Bound, PyAny, PyObject, PyResult, Python};
use serde_json::{json, Value};
use statsig_rust::{log_e, DynamicValue};
use std::collections::HashMap;

const TAG: &str = "PyoUtils";

pub fn py_dict_to_map(dict: &Bound<PyDict>) -> HashMap<String, DynamicValue> {
    let mut hashmap = HashMap::new();
    for (key, value) in dict.iter() {
        let key_str = match key.extract::<String>() {
            Ok(k) => k,
            Err(_) => {
                log_e!(TAG, "Skipping entry: Key must be a string");
                continue;
            }
        };

        let value_json = match py_any_to_dynamic_value(&value) {
            Ok(v) => v,
            Err(_) => {
                log_e!(TAG, "Skipping entry: Invalid value for key '{}'", key_str);
                continue;
            }
        };

        hashmap.insert(key_str, value_json);
    }
    hashmap
}

pub fn map_to_py_dict(py: Python, map: &HashMap<String, Value>) -> PyObject {
    let value = match serde_json::to_string(&map) {
        Ok(v) => v,
        Err(e) => {
            log_e!(TAG, "Failed to serialize map to JSON: {}", e);
            return PyDict::new(py).unbind().into();
        }
    };

    let json = match PyModule::import(py, "json") {
        Ok(j) => j,
        Err(e) => {
            log_e!(TAG, "Failed to import json module: {}", e);
            return PyDict::new(py).unbind().into();
        }
    };

    return match json.call_method1("loads", (value.clone(),)) {
        Ok(d) => d.unbind(),
        Err(e) => {
            log_e!(TAG, "Failed to call json.loads: {}", e);
            return PyDict::new(py).unbind().into();
        }
    };
}

/// order matters in this function, please don't change
pub fn py_any_to_dynamic_value(value: &Bound<PyAny>) -> PyResult<DynamicValue> {
    if let Ok(val) = value.extract::<String>() {
        return Ok(DynamicValue::from(val));
    }

    if let Ok(val) = value.extract::<bool>() {
        return Ok(DynamicValue::from(val));
    }

    if let Ok(val) = value.extract::<i64>() {
        return Ok(DynamicValue::from(val));
    }

    if let Ok(val) = value.extract::<f64>() {
        return Ok(DynamicValue::from(val));
    }

    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut hashmap = HashMap::new();
        for (key, val) in dict.iter() {
            let key_str = key.extract::<String>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err("Dictionary keys must be strings")
            })?;
            hashmap.insert(key_str, py_any_to_dynamic_value(&val)?);
        }
        let json_value = json!(hashmap
            .iter()
            .map(|(k, v)| (k, &v.json_value))
            .collect::<HashMap<_, _>>());
        return Ok(DynamicValue {
            object_value: Some(hashmap.clone()),
            json_value,
            ..DynamicValue::default()
        });
    }

    if let Ok(list) = value.downcast::<PyList>() {
        let mut vec = Vec::new();
        let mut str_vec = Vec::new();

        if let Ok(iter) = list.try_iter() {
            for value in iter {
                let value = value?;
                let dyn_value = py_any_to_dynamic_value(&value)?;

                str_vec.push(dyn_value.string_value.clone());
                vec.push(py_any_to_dynamic_value(&value)?);
            }
        }

        let json_string = serde_json::to_string(&str_vec).unwrap_or_else(|_| "[]".to_string());

        return Ok(DynamicValue {
            array_value: Some(vec.clone()),
            string_value: Some(json_string.clone()),
            json_value: json!(vec),
            lowercase_string_value: Some(json_string.to_lowercase()),
            ..DynamicValue::default()
        });
    }

    Err(PyValueError::new_err("Unsupported value type"))
}
