use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{from_value, Value as JsonValue};

#[derive(Clone, Debug)]
pub struct DynamicReturnable {
    pub value: JsonValue,
    pub string_value: String,
    pub json_value: Option<HashMap<String, JsonValue>>,
}

impl Serialize for DynamicReturnable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.string_value)
    }
}

impl<'de> Deserialize<'de> for DynamicReturnable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = JsonValue::deserialize(deserializer)?;
        let string_value = value.to_string();
        let json_value = from_value(value.clone()).ok();

        Ok(DynamicReturnable {
            string_value,
            value,
            json_value,
        })
    }
}
