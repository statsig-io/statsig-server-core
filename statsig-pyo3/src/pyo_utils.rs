use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::types::{
    PyAnyMethods, PyBool, PyDict, PyDictMethods, PyFloat, PyFloatMethods, PyInt, PyList,
    PyListMethods, PyModule, PyString, PyStringMethods, PyTypeMethods,
};
use pyo3::{Bound, Py, PyAny, PyErr, PyResult, PyTypeCheck, Python};
use serde_json::{json, Number, Value};
use statsig_rust::evaluation::dynamic_string::DynamicString;
use statsig_rust::{log_e, log_w, DynamicValue};
use std::borrow::Cow;
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

pub fn map_to_py_dict(py: Python, map: &HashMap<String, Value>) -> Py<PyAny> {
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

pub fn map_to_py_dict_direct(py: Python, map: &HashMap<String, Value>) -> PyResult<Py<PyDict>> {
    let py_dict = PyDict::new(py);

    for (key, value) in map {
        set_json_value_in_dict(py, &py_dict, key, value)?;
    }

    Ok(py_dict.unbind())
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

pub fn list_of_values_to_py_list(py: Python, list: &Vec<Value>) -> PyResult<Py<PyAny>> {
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

fn json_array_to_py_list(py: Python, values: &Vec<Value>) -> PyResult<Py<PyAny>> {
    let py_list = PyList::empty(py);

    for value in values {
        append_json_value(py, &py_list, value)?;
    }

    Ok(py_list.unbind().into())
}

fn json_object_to_py_dict(py: Python, map: &serde_json::Map<String, Value>) -> PyResult<Py<PyAny>> {
    let py_dict = PyDict::new(py);

    for (key, value) in map {
        set_json_value_in_dict(py, &py_dict, key, value)?;
    }

    Ok(py_dict.unbind().into())
}

fn set_json_value_in_dict(
    py: Python,
    py_dict: &Bound<PyDict>,
    key: &str,
    value: &Value,
) -> PyResult<()> {
    match value {
        Value::Null => py_dict.set_item(key, py.None())?,
        Value::Bool(v) => py_dict.set_item(key, *v)?,
        Value::Number(v) => set_number_in_dict(py_dict, key, v)?,
        Value::String(v) => py_dict.set_item(key, v)?,
        Value::Array(v) => py_dict.set_item(key, json_array_to_py_list(py, v)?)?,
        Value::Object(v) => py_dict.set_item(key, json_object_to_py_dict(py, v)?)?,
    }

    Ok(())
}

fn append_json_value(py: Python, py_list: &Bound<PyList>, value: &Value) -> PyResult<()> {
    match value {
        Value::Null => py_list.append(py.None())?,
        Value::Bool(v) => py_list.append(*v)?,
        Value::Number(v) => append_number(py_list, v)?,
        Value::String(v) => py_list.append(v)?,
        Value::Array(v) => py_list.append(json_array_to_py_list(py, v)?)?,
        Value::Object(v) => py_list.append(json_object_to_py_dict(py, v)?)?,
    }

    Ok(())
}

fn set_number_in_dict(py_dict: &Bound<PyDict>, key: &str, value: &Number) -> PyResult<()> {
    if let Some(v) = value.as_i64() {
        py_dict.set_item(key, v)?;
    } else if let Some(v) = value.as_u64() {
        py_dict.set_item(key, v)?;
    } else if let Some(v) = value.as_f64() {
        py_dict.set_item(key, v)?;
    } else {
        py_dict.set_item(key, value.to_string())?;
    }

    Ok(())
}

fn append_number(py_list: &Bound<PyList>, value: &Number) -> PyResult<()> {
    if let Some(v) = value.as_i64() {
        py_list.append(v)?;
    } else if let Some(v) = value.as_u64() {
        py_list.append(v)?;
    } else if let Some(v) = value.as_f64() {
        py_list.append(v)?;
    } else {
        py_list.append(value.to_string())?;
    }

    Ok(())
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

    if let Ok(dict) = value.cast::<PyDict>() {
        let mut hashmap = HashMap::new();
        for (key, val) in dict.iter() {
            let key_str = key.extract::<String>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err("Dictionary keys must be strings")
            })?;
            hashmap.insert(key_str, py_any_to_value(&val)?);
        }
        return Ok(Value::Object(hashmap.into_iter().collect()));
    }

    if let Ok(list) = value.cast::<PyList>() {
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

    if let Ok(dict) = value.cast::<PyDict>() {
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

    if let Ok(list) = value.cast::<PyList>() {
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

// ------------------------------------------------------------------------------- [ Statsig User Creation ]

pub fn opt_py_dict_ref_to_hashmap(
    data: Option<&Bound<'_, PyDict>>,
) -> Option<HashMap<String, DynamicValue>> {
    let data = data?;
    Some(py_dict_ref_to_hashmap(data))
}

fn py_any_ref_to_dynamic_value(value: &Bound<'_, PyAny>) -> Option<DynamicValue> {
    if let Some(dv) = try_as_cow_str(value) {
        return Some(DynamicValue::from(dv.as_ref()));
    }

    if let Some(bool) = try_as_bool(value) {
        return Some(DynamicValue::from(bool));
    }

    if let Some(float) = try_as_float(value) {
        return Some(DynamicValue::from(float));
    }

    if let Some(int) = try_as_int(value) {
        return Some(DynamicValue::from(int));
    }

    if let Some(list) = try_as_list(value) {
        return Some(DynamicValue::from(list));
    }

    if let Some(dict) = try_as_dict(value) {
        return Some(DynamicValue::from(dict));
    }

    match value.get_type().name() {
        Ok(name) => log_w!(TAG, "Unsupported value type: {} for value: {}", name, value),
        Err(e) => log_w!(TAG, "Unsupported value type for value: {} - {}", value, e),
    }

    None
}

fn py_dict_ref_to_hashmap(dict: &Bound<'_, PyDict>) -> HashMap<String, DynamicValue> {
    let mut values = HashMap::with_capacity(dict.len());

    for (key, value) in dict.iter() {
        let dynamic_val = match py_any_ref_to_dynamic_value(&value) {
            Some(v) => v,
            None => {
                log_w!(
                    TAG,
                    "Skipping entry: Unsupported value type for key '{}'",
                    key
                );
                continue;
            }
        };

        let key_string = match try_as_cow_str(&key) {
            Some(k) => k.to_string(),
            None => {
                log_w!(TAG, "Skipping entry: Non-string key '{}'", key);
                continue;
            }
        };

        values.insert(key_string, dynamic_val);
    }

    values
}

fn try_as_cow_str<'py>(value: &'py Bound<'py, PyAny>) -> Option<Cow<'py, str>> {
    if !PyString::type_check(value) {
        return None;
    }

    // SAFETY: This is what the "safe" version does internally, but its faster because we skip the Error creation
    let py_string = unsafe { value.cast_unchecked::<PyString>() };

    let cow_str = match py_string.to_cow() {
        Ok(cow_str_ref) => cow_str_ref,
        Err(e) => {
            log_w!(TAG, "Failed to convert PyString to Cow<str>: {}", e);
            return None;
        }
    };

    Some(cow_str)
}

fn try_as_bool<'py>(value: &'py Bound<'py, PyAny>) -> Option<bool> {
    if !PyBool::type_check(value) {
        return None;
    }

    // SAFETY: This is what the "safe" version does internally, but its faster because we skip the Error creation
    let pybool = unsafe { value.cast_unchecked::<PyBool>() };

    Some(pybool == true)
}

fn try_as_int<'py>(value: &'py Bound<'py, PyAny>) -> Option<i64> {
    if !PyInt::type_check(value) {
        return None;
    }

    // SAFETY: This is what the "safe" version does internally, but its faster because we skip the Error creation
    let pyint = unsafe { value.cast_unchecked::<PyInt>() };

    let value = unsafe { pyo3::ffi::PyLong_AsLong(pyint.as_ptr()) };
    Some(value)
}

fn try_as_float<'py>(value: &'py Bound<'py, PyAny>) -> Option<f64> {
    if !PyFloat::type_check(value) {
        return None;
    }

    // SAFETY: This is what the "safe" version does internally, but its faster because we skip the Error creation
    let pyfloat = unsafe { value.cast_unchecked::<PyFloat>() };

    let value = pyfloat.value();
    Some(value)
}

fn try_as_list<'py>(value: &'py Bound<'py, PyAny>) -> Option<Vec<DynamicValue>> {
    if !PyList::type_check(value) {
        return None;
    }

    // SAFETY: This is what the "safe" version does internally, but its faster because we skip the Error creation
    let pylist = unsafe { value.cast_unchecked::<PyList>() };
    let mut values = Vec::with_capacity(pylist.len());
    for item in pylist.iter() {
        match py_any_ref_to_dynamic_value(&item) {
            Some(dynamic_val) => values.push(dynamic_val),
            None => log_w!(TAG, "Skipping entry: Unsupported value type for list item"),
        };
    }

    Some(values)
}

fn try_as_dict<'py>(value: &'py Bound<'py, PyAny>) -> Option<HashMap<String, DynamicValue>> {
    if !PyDict::type_check(value) {
        return None;
    }

    // SAFETY: This is what the "safe" version does internally, but its faster because we skip the Error creation
    let pydict = unsafe { value.cast_unchecked::<PyDict>() };
    Some(py_dict_ref_to_hashmap(pydict))
}
