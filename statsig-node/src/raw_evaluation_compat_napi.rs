use napi::{Env, JsObject, JsUnknown};
use napi_derive::napi;
use rkyv::{collections::swiss_table::ArchivedHashMap, string::ArchivedString, vec::ArchivedVec};
use serde_json::Value as SerdeValue;
use statsig_rust::{
    evaluation::rkyv_value::{ArchivedRkyvNumber, ArchivedRkyvValue, RkyvNumber, RkyvValue},
    interned_string::InternedString,
    log_e,
    statsig_types_raw::{
        DynamicConfigRaw, ExperimentRaw, FeatureGateRaw, LayerRaw, PartialLayerRaw, SuffixedRuleId,
    },
    EvaluationDetails, SecondaryExposure,
};

const TAG: &str = "RawEvaluationCompatNapi";

#[napi]
pub struct LayerParamExposureData {
    pub(crate) inner: PartialLayerRaw,
}

pub(crate) fn raw_gate_to_js_object(env: &Env, raw: &FeatureGateRaw) -> napi::Result<JsObject> {
    let mut object = env.create_object()?;

    object.set_named_property("name", raw.name)?;
    object.set_named_property("value", raw.value)?;
    insert_rule_id(&mut object, &raw.rule_id)?;
    object.set_named_property("idType", opt_interned_str_to_js(env, &raw.id_type)?)?;
    object.set_named_property(
        "details",
        evaluation_details_to_js_object(env, raw.details)?,
    )?;

    Ok(object)
}

pub(crate) fn raw_dynamic_config_to_js_object(
    env: &Env,
    raw: &DynamicConfigRaw,
) -> napi::Result<JsObject> {
    let mut object = env.create_object()?;

    object.set_named_property("name", raw.name)?;

    if let Some(value) = raw.value {
        if let Some(value) = value.get_json_archived_ref() {
            object.set_named_property("value", rkyv_archived_object_to_js_object(env, value)?)?;
        } else if let Some(value) = value.get_json_pointer_ref() {
            object.set_named_property("value", rkyv_object_to_js_object(env, value)?)?;
        }
    }

    insert_rule_id(&mut object, &raw.rule_id)?;
    object.set_named_property("idType", opt_interned_str_to_js(env, &raw.id_type)?)?;
    object.set_named_property(
        "details",
        evaluation_details_to_js_object(env, raw.details)?,
    )?;

    Ok(object)
}

pub(crate) fn raw_experiment_to_js_object(
    env: &Env,
    raw: &ExperimentRaw,
) -> napi::Result<JsObject> {
    let mut object = raw_dynamic_config_to_js_object(
        env,
        &DynamicConfigRaw {
            name: raw.name,
            value: raw.value,
            rule_id: SuffixedRuleId {
                rule_id: raw.rule_id.rule_id,
                rule_id_suffix: raw.rule_id.rule_id_suffix,
            },
            id_type: raw.id_type,
            details: raw.details,
        },
    )?;

    object.set_named_property("groupName", opt_interned_str_to_js(env, &raw.group_name)?)?;
    object.set_named_property(
        "isExperimentActive",
        opt_bool_to_js(env, raw.is_experiment_active)?,
    )?;

    if let Some(secondary_exposures) = raw.secondary_exposures {
        let mut exposures = env.create_array_with_length(secondary_exposures.len())?;
        for (index, exposure) in secondary_exposures.iter().enumerate() {
            exposures.set_element(
                index as u32,
                secondary_exposure_to_js_object(env, exposure)?,
            )?;
        }
        object.set_named_property("secondaryExposures", exposures)?;
    }

    Ok(object)
}

pub(crate) fn raw_layer_to_js_object(env: &Env, raw: &LayerRaw) -> napi::Result<JsObject> {
    let mut object = env.create_object()?;

    object.set_named_property("name", raw.name)?;

    if let Some(value) = raw.value {
        if let Some(value) = value.get_json_archived_ref() {
            object.set_named_property("value", rkyv_archived_object_to_js_object(env, value)?)?;
        } else if let Some(value) = value.get_json_pointer_ref() {
            object.set_named_property("value", rkyv_object_to_js_object(env, value)?)?;
        }
    }

    insert_rule_id(&mut object, &raw.rule_id)?;
    object.set_named_property("idType", opt_interned_str_to_js(env, &raw.id_type)?)?;
    object.set_named_property(
        "details",
        evaluation_details_to_js_object(env, raw.details)?,
    )?;
    object.set_named_property("groupName", opt_interned_str_to_js(env, &raw.group_name)?)?;
    object.set_named_property(
        "isExperimentActive",
        opt_bool_to_js(env, raw.is_experiment_active)?,
    )?;
    object.set_named_property(
        "allocatedExperimentName",
        opt_interned_str_to_js(env, &raw.allocated_experiment_name)?,
    )?;

    if let Some(secondary_exposures) = raw.secondary_exposures {
        let mut exposures = env.create_array_with_length(secondary_exposures.len())?;
        for (index, exposure) in secondary_exposures.iter().enumerate() {
            exposures.set_element(
                index as u32,
                secondary_exposure_to_js_object(env, exposure)?,
            )?;
        }
        object.set_named_property("secondaryExposures", exposures)?;
    }

    object.set_named_property(
        "__exposure",
        LayerParamExposureData {
            inner: PartialLayerRaw::from(raw),
        },
    )?;

    Ok(object)
}

pub(crate) fn string_to_js_object(env: &Env, raw: &str) -> napi::Result<JsObject> {
    serde_value_to_js_object(
        env,
        serde_json::from_str(raw).unwrap_or_else(|error| {
            log_e!(TAG, "Failed to parse raw evaluation JSON: {}", error);
            SerdeValue::Object(serde_json::Map::new())
        }),
    )
}

fn evaluation_details_to_js_object(
    env: &Env,
    details: &EvaluationDetails,
) -> napi::Result<JsObject> {
    let mut object = env.create_object()?;

    object.set_named_property("reason", details.reason.as_str())?;
    object.set_named_property("lcut", opt_u64_to_js(env, details.lcut)?)?;
    object.set_named_property("received_at", opt_u64_to_js(env, details.received_at)?)?;
    object.set_named_property("version", opt_u32_to_js(env, details.version)?)?;

    Ok(object)
}

fn secondary_exposure_to_js_object(
    env: &Env,
    exposure: &SecondaryExposure,
) -> napi::Result<JsObject> {
    let mut object = env.create_object()?;

    object.set_named_property("gate", exposure.gate.as_str())?;
    object.set_named_property("gateValue", exposure.gate_value.as_str())?;
    object.set_named_property("ruleID", exposure.rule_id.as_str())?;

    Ok(object)
}

fn insert_rule_id(object: &mut JsObject, rule_id: &SuffixedRuleId) -> napi::Result<()> {
    let value = rule_id
        .try_as_unprefixed_str()
        .map(str::to_string)
        .unwrap_or_else(|| rule_id.unperformant_to_string());
    object.set_named_property("ruleID", value)
}

fn opt_interned_str_to_js(env: &Env, value: &Option<&InternedString>) -> napi::Result<JsUnknown> {
    match value {
        Some(value) => Ok(env.create_string(value.as_str())?.into_unknown()),
        None => Ok(env.get_null()?.into_unknown()),
    }
}

fn opt_bool_to_js(env: &Env, value: Option<bool>) -> napi::Result<JsUnknown> {
    match value {
        Some(value) => Ok(env.get_boolean(value)?.into_unknown()),
        None => Ok(env.get_null()?.into_unknown()),
    }
}

fn opt_u64_to_js(env: &Env, value: Option<u64>) -> napi::Result<JsUnknown> {
    match value {
        Some(value) => Ok(env.create_double(value as f64)?.into_unknown()),
        None => Ok(env.get_null()?.into_unknown()),
    }
}

fn opt_u32_to_js(env: &Env, value: Option<u32>) -> napi::Result<JsUnknown> {
    match value {
        Some(value) => Ok(env.create_double(value as f64)?.into_unknown()),
        None => Ok(env.get_null()?.into_unknown()),
    }
}

fn rkyv_archived_object_to_js_object(
    env: &Env,
    object: &ArchivedHashMap<ArchivedString, ArchivedRkyvValue>,
) -> napi::Result<JsObject> {
    let mut result = env.create_object()?;

    for (key, value) in object.iter() {
        result.set_named_property(key.as_str(), archived_rkyv_value_to_js(env, value)?)?;
    }

    Ok(result)
}

fn archived_rkyv_value_to_js(env: &Env, value: &ArchivedRkyvValue) -> napi::Result<JsUnknown> {
    Ok(match value {
        ArchivedRkyvValue::Null => env.get_null()?.into_unknown(),
        ArchivedRkyvValue::Bool(value) => env.get_boolean(*value)?.into_unknown(),
        ArchivedRkyvValue::Number(value) => archived_rkyv_number_to_js(env, value)?,
        ArchivedRkyvValue::String(value) => env.create_string(value.as_str())?.into_unknown(),
        ArchivedRkyvValue::Array(value) => {
            rkyv_archived_array_to_js_object(env, value)?.into_unknown()
        }
        ArchivedRkyvValue::Object(value) => {
            rkyv_archived_object_to_js_object(env, value)?.into_unknown()
        }
    })
}

fn archived_rkyv_number_to_js(env: &Env, value: &ArchivedRkyvNumber) -> napi::Result<JsUnknown> {
    let value = match value {
        ArchivedRkyvNumber::PosInt(value) => value.to_native() as f64,
        ArchivedRkyvNumber::NegInt(value) => value.to_native() as f64,
        ArchivedRkyvNumber::Float(value) => value.to_native(),
    };
    Ok(env.create_double(value)?.into_unknown())
}

fn rkyv_archived_array_to_js_object(
    env: &Env,
    values: &ArchivedVec<ArchivedRkyvValue>,
) -> napi::Result<JsObject> {
    let mut result = env.create_array_with_length(values.len())?;
    for (index, value) in values.iter().enumerate() {
        result.set_element(index as u32, archived_rkyv_value_to_js(env, value)?)?;
    }
    Ok(result)
}

fn rkyv_object_to_js_object(
    env: &Env,
    object: &std::collections::HashMap<String, RkyvValue>,
) -> napi::Result<JsObject> {
    let mut result = env.create_object()?;

    for (key, value) in object {
        result.set_named_property(key, rkyv_value_to_js(env, value)?)?;
    }

    Ok(result)
}

fn rkyv_value_to_js(env: &Env, value: &RkyvValue) -> napi::Result<JsUnknown> {
    Ok(match value {
        RkyvValue::Null => env.get_null()?.into_unknown(),
        RkyvValue::Bool(value) => env.get_boolean(*value)?.into_unknown(),
        RkyvValue::Number(value) => rkyv_number_to_js(env, value)?,
        RkyvValue::String(value) => env.create_string(value)?.into_unknown(),
        RkyvValue::Array(value) => rkyv_array_to_js_object(env, value)?.into_unknown(),
        RkyvValue::Object(value) => rkyv_object_to_js_object(env, value)?.into_unknown(),
    })
}

fn rkyv_array_to_js_object(env: &Env, values: &[RkyvValue]) -> napi::Result<JsObject> {
    let mut result = env.create_array_with_length(values.len())?;
    for (index, value) in values.iter().enumerate() {
        result.set_element(index as u32, rkyv_value_to_js(env, value)?)?;
    }
    Ok(result)
}

fn rkyv_number_to_js(env: &Env, value: &RkyvNumber) -> napi::Result<JsUnknown> {
    let value = match value {
        RkyvNumber::PosInt(value) => *value as f64,
        RkyvNumber::NegInt(value) => *value as f64,
        RkyvNumber::Float(value) => *value,
    };
    Ok(env.create_double(value)?.into_unknown())
}

fn serde_value_to_js_object(env: &Env, value: SerdeValue) -> napi::Result<JsObject> {
    match value {
        SerdeValue::Object(object) => {
            let mut result = env.create_object()?;
            for (key, value) in object {
                result.set_named_property(&key, serde_value_to_js(env, value)?)?;
            }
            Ok(result)
        }
        _ => env.create_object(),
    }
}

fn serde_value_to_js(env: &Env, value: SerdeValue) -> napi::Result<JsUnknown> {
    Ok(match value {
        SerdeValue::Null => env.get_null()?.into_unknown(),
        SerdeValue::Bool(value) => env.get_boolean(value)?.into_unknown(),
        SerdeValue::Number(value) => env
            .create_double(value.as_f64().unwrap_or_default())?
            .into_unknown(),
        SerdeValue::String(value) => env.create_string(&value)?.into_unknown(),
        SerdeValue::Array(values) => {
            let mut result = env.create_array_with_length(values.len())?;
            for (index, value) in values.into_iter().enumerate() {
                result.set_element(index as u32, serde_value_to_js(env, value)?)?;
            }
            result.into_unknown()
        }
        SerdeValue::Object(object) => {
            let mut result = env.create_object()?;
            for (key, value) in object {
                result.set_named_property(&key, serde_value_to_js(env, value)?)?;
            }
            result.into_unknown()
        }
    })
}
