use pyo3::exceptions::PyValueError;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyList};
use pyo3::{Bound, PyAny, PyObject, PyResult, Python};
use serde_json::json;
use sigstat::DynamicValue;
use std::collections::HashMap;

/// Converts a `PyObject` into a `HashMap<String, DynamicValue>`, if it's a `PyDict`
pub fn py_object_to_map(
    py: Python,
    obj: Option<&PyObject>,
) -> PyResult<Option<HashMap<String, DynamicValue>>> {
    match obj {
        Some(py_obj) => {
            if let Ok(dict) = py_obj.downcast_bound::<PyDict>(py) {
                Ok(Some(py_dict_to_map(dict)?))
            } else {
                Err(pyo3::exceptions::PyTypeError::new_err(
                    "Expected a dictionary",
                ))
            }
        }
        None => Ok(None),
    }
}

pub fn py_dict_to_map(dict: &Bound<PyDict>) -> PyResult<HashMap<String, DynamicValue>> {
    let mut hashmap = HashMap::new();
    for (key, value) in dict.iter() {
        let key_str = key
            .extract::<String>()
            .map_err(|_| PyValueError::new_err("Keys must be strings"))?;

        let value_json = py_any_to_dynamic_value(&value)
            .map_err(|_| PyValueError::new_err(format!("Invalid value for key '{}'", key_str)))?;
        hashmap.insert(key_str, value_json);
    }
    Ok(hashmap)
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
