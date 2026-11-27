use crate::{AmplitudeError, Event, EventOptions, JsonObject, JsonValue};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::collections::{BTreeMap, HashSet};

pub type Identify = Vec<IdentifyOperation>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum IdentifyOperation {
    Set { key: String, value: JsonValue },
    SetOnce { key: String, value: JsonValue },
    Append { key: String, value: JsonValue },
    Prepend { key: String, value: JsonValue },
    PreInsert { key: String, value: JsonValue },
    PostInsert { key: String, value: JsonValue },
    Remove { key: String, value: JsonValue },
    Add { key: String, value: f64 },
    Unset { key: String },
    ClearAll,
}

impl Event {
    /// Builds the event submitted by an identify operation.
    pub fn from_identify(
        operations: Identify,
        options: EventOptions,
        event_properties: Option<JsonObject>,
    ) -> Result<Self, AmplitudeError> {
        Ok(Self {
            options,
            event_properties,
            user_properties: Some(user_properties(operations)?),
            ..Self::new("$identify")
        })
    }
}

fn user_properties(operations: Identify) -> Result<JsonObject, AmplitudeError> {
    if operations.is_empty() {
        return Err(AmplitudeError::InvalidInput(
            "At least one identify operation is required",
        ));
    }
    if matches!(operations.as_slice(), [IdentifyOperation::ClearAll]) {
        return Ok(JsonObject::from_iter([("$clearAll".into(), "-".into())]));
    }

    let mut keys = HashSet::new();
    let mut properties = BTreeMap::<&str, JsonObject>::new();
    for operation in operations {
        let (name, key, value) = match operation {
            IdentifyOperation::Set { key, value } => ("$set", key, value),
            IdentifyOperation::SetOnce { key, value } => ("$setOnce", key, value),
            IdentifyOperation::Append { key, value } => ("$append", key, value),
            IdentifyOperation::Prepend { key, value } => ("$prepend", key, value),
            IdentifyOperation::PreInsert { key, value } => ("$preInsert", key, value),
            IdentifyOperation::PostInsert { key, value } => ("$postInsert", key, value),
            IdentifyOperation::Remove { key, value } => ("$remove", key, value),
            IdentifyOperation::Add { key, value } => {
                let number = Number::from_f64(value).ok_or(AmplitudeError::InvalidInput(
                    "The identify add value must be finite",
                ))?;
                ("$add", key, JsonValue::Number(number))
            }
            IdentifyOperation::Unset { key } => ("$unset", key, "-".into()),
            IdentifyOperation::ClearAll => {
                return Err(AmplitudeError::InvalidInput(
                    "Clear-all must be the only identify operation",
                ));
            }
        };
        if key.trim().is_empty() {
            return Err(AmplitudeError::InvalidInput(
                "Identify property names must not be empty",
            ));
        }
        if !keys.insert(key.clone()) {
            return Err(AmplitudeError::InvalidInput(
                "Supply one identify operation per property",
            ));
        }
        // Each operator has its own group of properties in the ingestion request.
        properties.entry(name).or_default().insert(key, value);
    }
    Ok(properties
        .into_iter()
        .map(|(name, values)| (name.into(), JsonValue::Object(values)))
        .collect())
}
