use pyo3::{
    types::{PyAnyMethods, PyDict, PyList, PyListMethods},
    Bound, Py, PyAny, PyResult, Python,
};
use serde_json::{Map as SerdeMap, Number as SerdeNumber, Value as SerdeValue};
use statsig_rust::{
    interned_string::InternedString,
    statsig_types_raw::{DynamicConfigRaw, ExperimentRaw, FeatureGateRaw, SuffixedRuleId},
    DynamicReturnable, EvaluationDetails, SecondaryExposure,
};

pub(crate) fn raw_gate_to_py_dict(py: Python, raw: &FeatureGateRaw) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);

    dict.set_item("name", raw.name)?;
    dict.set_item("value", raw.value)?;

    py_dict_insert_rule_id(&dict, &raw.rule_id)?;

    dict.set_item("idType", opt_interned_str(&raw.id_type))?;
    dict.set_item("details", evaluation_details_to_py_dict(py, raw.details)?)?;

    Ok(dict.unbind())
}

pub(crate) fn raw_dynamic_config_to_py_dict(
    py: Python,
    raw: &DynamicConfigRaw,
) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);

    dict.set_item("name", raw.name)?;

    if let Some(value) = raw.value {
        py_dict_insert_json_value(py, &dict, value)?;
    }

    py_dict_insert_rule_id(&dict, &raw.rule_id)?;

    dict.set_item("idType", opt_interned_str(&raw.id_type))?;
    dict.set_item("details", evaluation_details_to_py_dict(py, raw.details)?)?;

    Ok(dict.unbind())
}

pub(crate) fn raw_experiment_to_py_dict(py: Python, raw: &ExperimentRaw) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);

    dict.set_item("name", raw.name)?;

    if let Some(value) = raw.value {
        py_dict_insert_json_value(py, &dict, value)?;
    }

    py_dict_insert_rule_id(&dict, &raw.rule_id)?;

    dict.set_item("idType", opt_interned_str(&raw.id_type))?;
    dict.set_item("details", evaluation_details_to_py_dict(py, raw.details)?)?;
    dict.set_item("groupName", opt_interned_str(&raw.group_name))?;
    dict.set_item("isExperimentActive", raw.is_experiment_active)?;

    if let Some(secondary_exposures) = raw.secondary_exposures {
        dict.set_item(
            "secondaryExposures",
            secondary_exposures_to_py_list(py, secondary_exposures)?,
        )?;
    }

    Ok(dict.unbind())
}

fn evaluation_details_to_py_dict(py: Python, details: &EvaluationDetails) -> PyResult<Py<PyDict>> {
    let raw = PyDict::new(py);
    raw.set_item("reason", &details.reason)?;
    raw.set_item("lcut", details.lcut)?;
    raw.set_item("received_at", details.received_at)?;
    raw.set_item("version", details.version)?;

    Ok(raw.unbind())
}

fn secondary_exposures_to_py_list(
    py: Python,
    secondary_exposures: &Vec<SecondaryExposure>,
) -> PyResult<Py<PyList>> {
    let py_list = PyList::empty(py);
    for secondary_exposure in secondary_exposures {
        let dict = PyDict::new(py);
        dict.set_item("gate", secondary_exposure.gate.as_str())?;
        dict.set_item("gateValue", secondary_exposure.gate_value.as_str())?;
        dict.set_item("ruleID", secondary_exposure.rule_id.as_str())?;
        py_list.append(dict.unbind())?;
    }
    Ok(py_list.unbind())
}

fn opt_interned_str<'a>(value: &'a Option<&'a InternedString>) -> Option<&'a str> {
    value.as_ref().map(|value| value.as_str())
}

fn py_dict_insert_rule_id(py_dict: &Bound<PyDict>, rule_id: &SuffixedRuleId) -> PyResult<()> {
    if let Some(rule_id) = rule_id.try_as_unprefixed_str() {
        py_dict.set_item("ruleID", rule_id)?;
    } else {
        py_dict.set_item("ruleID", rule_id.unperformant_to_string())?;
    }
    Ok(())
}

fn py_dict_insert_json_value(
    py: Python,
    py_dict: &Bound<PyDict>,
    returnable: &DynamicReturnable,
) -> PyResult<()> {
    if let Some(value) = returnable.get_serde_map() {
        py_dict.set_item("value", json_object_to_py_dict(py, value)?)?;
    }

    Ok(())
}

// ------------------------------------------------------------------------------- [ Serde to Pyo3 ]

fn json_array_to_py_list(py: Python, values: Vec<SerdeValue>) -> PyResult<Py<PyAny>> {
    let py_list = PyList::empty(py);

    for value in values {
        py_list_insert_value(py, &py_list, value)?;
    }

    Ok(py_list.unbind().into())
}

fn json_object_to_py_dict(py: Python, map: SerdeMap<String, SerdeValue>) -> PyResult<Py<PyDict>> {
    let py_dict = PyDict::new(py);

    for (key, value) in map {
        py_dict_insert_value(py, &py_dict, key, value)?;
    }

    Ok(py_dict.unbind())
}

fn py_dict_insert_value(
    py: Python,
    py_dict: &Bound<PyDict>,
    key: String,
    value: SerdeValue,
) -> PyResult<()> {
    match value {
        SerdeValue::Null => py_dict.set_item(key, py.None())?,
        SerdeValue::Bool(v) => py_dict.set_item(key, v)?,
        SerdeValue::Number(v) => py_dict_insert_number(py_dict, key, v)?,
        SerdeValue::String(v) => py_dict.set_item(key, v)?,
        SerdeValue::Array(v) => py_dict.set_item(key, json_array_to_py_list(py, v)?)?,
        SerdeValue::Object(v) => py_dict.set_item(key, json_object_to_py_dict(py, v)?)?,
    }

    Ok(())
}

fn py_dict_insert_number(py_dict: &Bound<PyDict>, key: String, value: SerdeNumber) -> PyResult<()> {
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

fn py_list_insert_value(py: Python, py_list: &Bound<PyList>, value: SerdeValue) -> PyResult<()> {
    match value {
        SerdeValue::Null => py_list.append(py.None())?,
        SerdeValue::Bool(v) => py_list.append(v)?,
        SerdeValue::Number(v) => py_list_insert_number(py_list, v)?,
        SerdeValue::String(v) => py_list.append(v)?,
        SerdeValue::Array(v) => py_list.append(json_array_to_py_list(py, v)?)?,
        SerdeValue::Object(v) => py_list.append(json_object_to_py_dict(py, v)?)?,
    }

    Ok(())
}

fn py_list_insert_number(py_list: &Bound<PyList>, value: SerdeNumber) -> PyResult<()> {
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
