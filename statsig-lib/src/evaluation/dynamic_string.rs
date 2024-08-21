use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

#[derive(Clone)]
pub struct DynamicString {
    pub value: String,
    pub lowercased_value: String,
}

impl Serialize for DynamicString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize only the original value field
        serializer.serialize_str(&self.value)
    }
}

impl<'de> Deserialize<'de> for DynamicString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json_value = JsonValue::deserialize(deserializer)?;
        let value = match json_value {
            JsonValue::String(value) => value,
            _ => {
                return Err(serde::de::Error::custom("Expected a string"));
            }
        };

        Ok(DynamicString {
            lowercased_value: value.to_lowercase(),
            value: value.to_string(),
        })
    }
}
