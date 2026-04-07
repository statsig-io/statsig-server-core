use std::collections::HashMap;

use pyo3::{
    pyclass,
    types::{PyAnyMethods, PyDict, PyList, PyListMethods},
    Bound, Py, PyAny, PyResult, Python,
};
use pyo3_stub_gen::derive::gen_stub_pyclass;
use rkyv::{collections::swiss_table::ArchivedHashMap, string::ArchivedString, vec::ArchivedVec};
use statsig_rust::{
    evaluation::rkyv_value::{ArchivedRkyvNumber, ArchivedRkyvValue, RkyvNumber, RkyvValue},
    interned_string::InternedString,
    statsig_types_raw::{
        DynamicConfigRaw, ExperimentRaw, FeatureGateRaw, LayerRaw, PartialLayerRaw, SuffixedRuleId,
    },
    EvaluationDetails, SecondaryExposure,
};

#[gen_stub_pyclass]
#[pyclass(name = "LayerParamExposureData", module = "statsig_python_core")]
pub struct LayerParamExposureDataPy {
    pub(crate) inner: PartialLayerRaw,
}

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
        if let Some(value) = value.get_json_archived_ref() {
            dict.set_item("value", rkvy_archived_object_to_py_dict(py, value)?)?;
        } else if let Some(value) = value.get_json_pointer_ref() {
            dict.set_item("value", rkyv_object_to_py_dict(py, value)?)?;
        }
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
        if let Some(value) = value.get_json_archived_ref() {
            dict.set_item("value", rkvy_archived_object_to_py_dict(py, value)?)?;
        } else if let Some(value) = value.get_json_pointer_ref() {
            dict.set_item("value", rkyv_object_to_py_dict(py, value)?)?;
        }
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

pub(crate) fn raw_layer_to_py_dict(py: Python, raw: &LayerRaw) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);

    dict.set_item("name", raw.name)?;

    if let Some(value) = raw.value {
        if let Some(value) = value.get_json_archived_ref() {
            dict.set_item("value", rkvy_archived_object_to_py_dict(py, value)?)?;
        } else if let Some(value) = value.get_json_pointer_ref() {
            dict.set_item("value", rkyv_object_to_py_dict(py, value)?)?;
        }
    }

    py_dict_insert_rule_id(&dict, &raw.rule_id)?;

    dict.set_item("idType", opt_interned_str(&raw.id_type))?;
    dict.set_item("details", evaluation_details_to_py_dict(py, raw.details)?)?;
    dict.set_item("groupName", opt_interned_str(&raw.group_name))?;
    dict.set_item("isExperimentActive", raw.is_experiment_active)?;
    dict.set_item(
        "allocatedExperimentName",
        opt_interned_str(&raw.allocated_experiment_name),
    )?;

    if let Some(secondary_exposures) = raw.secondary_exposures {
        dict.set_item(
            "secondaryExposures",
            secondary_exposures_to_py_list(py, secondary_exposures)?,
        )?;
    }

    dict.set_item(
        "__exposure",
        Py::new(
            py,
            LayerParamExposureDataPy {
                inner: PartialLayerRaw::from(raw),
            },
        )?,
    )?;

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

// ------------------------------------------------------------------------------- [ Rkyv(Archived) to Pyo3 ]

fn rkvy_archived_object_to_py_dict(
    py: Python,
    map: &ArchivedHashMap<ArchivedString, ArchivedRkyvValue>,
) -> PyResult<Py<PyDict>> {
    let py_dict = PyDict::new(py);
    for (key, value) in map.iter() {
        py_dict_insert_rkvy_archived_value(py, &py_dict, key.as_str(), value)?;
    }
    Ok(py_dict.unbind())
}

fn py_dict_insert_rkvy_archived_value(
    py: Python,
    py_dict: &Bound<PyDict>,
    key: &str,
    value: &ArchivedRkyvValue,
) -> PyResult<()> {
    match value {
        ArchivedRkyvValue::Null => py_dict.set_item(key, py.None())?,
        ArchivedRkyvValue::Bool(v) => py_dict.set_item(key, v)?,
        ArchivedRkyvValue::Number(v) => py_dict_insert_rkyv_archived_num(py_dict, key, v)?,
        ArchivedRkyvValue::String(v) => py_dict.set_item(key, v.as_str())?,
        ArchivedRkyvValue::Array(v) => {
            py_dict.set_item(key, rkvy_archived_array_to_py_list(py, v)?)?
        }
        ArchivedRkyvValue::Object(v) => {
            py_dict.set_item(key, rkvy_archived_object_to_py_dict(py, v)?)?
        }
    }

    Ok(())
}

fn py_dict_insert_rkyv_archived_num(
    py_dict: &Bound<PyDict>,
    key: &str,
    value: &ArchivedRkyvNumber,
) -> PyResult<()> {
    match value {
        ArchivedRkyvNumber::PosInt(v) => py_dict.set_item(key, v.to_native())?,
        ArchivedRkyvNumber::NegInt(v) => py_dict.set_item(key, v.to_native())?,
        ArchivedRkyvNumber::Float(v) => py_dict.set_item(key, v.to_native())?,
    }
    Ok(())
}

fn rkvy_archived_array_to_py_list(
    py: Python,
    values: &ArchivedVec<ArchivedRkyvValue>,
) -> PyResult<Py<PyAny>> {
    let py_list = PyList::empty(py);
    for value in values.iter() {
        py_list_insert_rkvy_archived_value(py, &py_list, value)?;
    }

    Ok(py_list.unbind().into())
}

fn py_list_insert_rkvy_archived_value(
    py: Python,
    py_list: &Bound<PyList>,
    value: &ArchivedRkyvValue,
) -> PyResult<()> {
    match value {
        ArchivedRkyvValue::Null => py_list.append(py.None())?,
        ArchivedRkyvValue::Bool(v) => py_list.append(v)?,
        ArchivedRkyvValue::Number(v) => py_list_insert_rkyv_archived_num(py_list, v)?,
        ArchivedRkyvValue::String(v) => py_list.append(v.as_str())?,
        ArchivedRkyvValue::Array(v) => py_list.append(rkvy_archived_array_to_py_list(py, v)?)?,
        ArchivedRkyvValue::Object(v) => py_list.append(rkvy_archived_object_to_py_dict(py, v)?)?,
    }
    Ok(())
}

fn py_list_insert_rkyv_archived_num(
    py_list: &Bound<PyList>,
    value: &ArchivedRkyvNumber,
) -> PyResult<()> {
    match value {
        ArchivedRkyvNumber::PosInt(v) => py_list.append(v.to_native())?,
        ArchivedRkyvNumber::NegInt(v) => py_list.append(v.to_native())?,
        ArchivedRkyvNumber::Float(v) => py_list.append(v.to_native())?,
    }
    Ok(())
}

// ------------------------------------------------------------------------------- [ Rkyv to Pyo3 ]

fn rkyv_object_to_py_dict(py: Python, map: &HashMap<String, RkyvValue>) -> PyResult<Py<PyDict>> {
    let py_dict = PyDict::new(py);
    for (key, value) in map {
        py_dict_insert_rkvy_value(py, &py_dict, key, value)?;
    }
    Ok(py_dict.unbind())
}

fn py_dict_insert_rkvy_value(
    py: Python,
    py_dict: &Bound<PyDict>,
    key: &str,
    value: &RkyvValue,
) -> PyResult<()> {
    match value {
        RkyvValue::Null => py_dict.set_item(key, py.None())?,
        RkyvValue::Bool(v) => py_dict.set_item(key, v)?,
        RkyvValue::Number(v) => py_dict_insert_rkyv_num(py_dict, key, v)?,
        RkyvValue::String(v) => py_dict.set_item(key, v)?,
        RkyvValue::Array(v) => py_dict.set_item(key, rkyv_array_to_py_list(py, v)?)?,
        RkyvValue::Object(v) => py_dict.set_item(key, rkyv_object_to_py_dict(py, v)?)?,
    }

    Ok(())
}

fn py_dict_insert_rkyv_num(py_dict: &Bound<PyDict>, key: &str, value: &RkyvNumber) -> PyResult<()> {
    match value {
        RkyvNumber::PosInt(v) => py_dict.set_item(key, v)?,
        RkyvNumber::NegInt(v) => py_dict.set_item(key, v)?,
        RkyvNumber::Float(v) => py_dict.set_item(key, v)?,
    }
    Ok(())
}

fn rkyv_array_to_py_list(py: Python, values: &Vec<RkyvValue>) -> PyResult<Py<PyAny>> {
    let py_list = PyList::empty(py);
    for value in values {
        py_list_insert_rkyv_value(py, &py_list, value)?;
    }
    Ok(py_list.unbind().into())
}

fn py_list_insert_rkyv_value(
    py: Python,
    py_list: &Bound<PyList>,
    value: &RkyvValue,
) -> PyResult<()> {
    match value {
        RkyvValue::Null => py_list.append(py.None())?,
        RkyvValue::Bool(v) => py_list.append(v)?,
        RkyvValue::Number(v) => py_list_insert_rkyv_num(py_list, v)?,
        RkyvValue::String(v) => py_list.append(v)?,
        RkyvValue::Array(v) => py_list.append(rkyv_array_to_py_list(py, v)?)?,
        RkyvValue::Object(v) => py_list.append(rkyv_object_to_py_dict(py, v)?)?,
    }
    Ok(())
}

fn py_list_insert_rkyv_num(py_list: &Bound<PyList>, value: &RkyvNumber) -> PyResult<()> {
    match value {
        RkyvNumber::PosInt(v) => py_list.append(v)?,
        RkyvNumber::NegInt(v) => py_list.append(v)?,
        RkyvNumber::Float(v) => py_list.append(v)?,
    };

    Ok(())
}
