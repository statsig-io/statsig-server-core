use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Value as JsonValue, Value};

#[derive(Clone, Eq, Debug)]
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

impl PartialEq for DynamicString {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl From<Value> for DynamicString {
    fn from(value: Value) -> Self {
        let str_value = match value.as_str() {
            Some(value) => value.to_string(),
            None => value.to_string(),
        };
        DynamicString::from(str_value)
    }
}

impl From<String> for DynamicString {
    fn from(value: String) -> Self {
        let lowercased_value = value.to_lowercase();
        DynamicString {
            value,
            lowercased_value,
        }
    }
}
