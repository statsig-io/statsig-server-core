use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyList, PyListMethods, PyModule};
use pyo3::{Bound, PyAny, PyErr, PyObject, PyResult, Python};
use serde_json::{json, Number, Value};
use statsig_rust::evaluation::dynamic_string::DynamicString;
use statsig_rust::{log_e, DynamicValue};
use std::collections::HashMap;

const TAG: &str = "PyoUtils";

pub fn py_dict_to_json_value_map(dict: &Bound<PyDict>) -> HashMap<String, Value> {
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
            Ok(v) => v.json_value,
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

pub fn py_list_to_list(py_list: &Bound<PyList>) -> PyResult<Vec<String>> {
    let mut converted_list = Vec::new();
    for value in py_list {
        match value.extract::<String>() {
            Ok(s) => converted_list.push(s),
            Err(e) => return Err(e),
        }
    }
    Ok(converted_list)
}

pub fn py_list_to_list_of_values(py_list: &Bound<PyList>) -> PyResult<Vec<Value>> {
    let mut converted_list = Vec::new();
    for value in py_list {
        let v = py_any_to_value(&value)?;
        converted_list.push(v);
    }
    Ok(converted_list)
}

pub fn list_of_values_to_py_list(py: Python, list: &Vec<Value>) -> PyResult<PyObject> {
    let py_list = PyList::empty(py);
    for value in list {
        match value {
            Value::String(s) => py_list.append(s)?,
            Value::Number(n) => py_list.append(n.as_i64().unwrap_or(0))?,
            Value::Bool(b) => py_list.append(b)?,
            Value::Object(o) => {
                let map: HashMap<String, Value> =
                    o.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                py_list.append(map_to_py_dict(py, &map))?
            }
            Value::Array(a) => py_list.append(list_of_values_to_py_list(py, a)?)?,
            Value::Null => py_list.append(py.None())?,
        }
    }
    Ok(py_list.into())
}

pub fn get_string_from_py_dict_throw_on_none(
    py_dict: &Bound<PyDict>,
    key: &str,
) -> PyResult<String> {
    match py_dict.get_item(key)? {
        Some(v) => Ok(v.extract::<String>()?),
        None => Err(PyErr::new::<PyTypeError, _>("Value in dict is null")),
    }
}

pub fn py_any_to_value(value: &Bound<PyAny>) -> PyResult<Value> {
    if let Ok(val) = value.extract::<String>() {
        return Ok(Value::String(val));
    }

    if let Ok(val) = value.extract::<bool>() {
        return Ok(Value::Bool(val));
    }

    if let Ok(val) = value.extract::<i64>() {
        return Ok(Value::Number(Number::from(val)));
    }

    if let Ok(val) = value.extract::<f64>() {
        return Ok(Value::Number(Number::from(val as i64)));
    }

    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut hashmap = HashMap::new();
        for (key, val) in dict.iter() {
            let key_str = key.extract::<String>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err("Dictionary keys must be strings")
            })?;
            hashmap.insert(key_str, py_any_to_value(&val)?);
        }
        return Ok(Value::Object(hashmap.into_iter().collect()));
    }

    if let Ok(list) = value.downcast::<PyList>() {
        let mut vec = Vec::new();
        let mut str_vec = Vec::new();

        if let Ok(iter) = list.try_iter() {
            for value in iter {
                let value = value?;
                let dyn_value = py_any_to_dynamic_value(&value)?;

                str_vec.push(dyn_value.string_value.clone());
                vec.push(py_any_to_value(&value)?);
            }
        }

        return Ok(Value::Array(vec));
    }

    Err(PyValueError::new_err("Unsupported value type"))
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
        let dyn_str = DynamicString::from(json_string);

        return Ok(DynamicValue {
            array_value: Some(vec.clone()),
            string_value: Some(dyn_str),
            json_value: json!(vec),
            ..DynamicValue::default()
        });
    }

    Err(PyValueError::new_err("Unsupported value type"))
}
