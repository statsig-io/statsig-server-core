use crate::{hashing::ahash_str, log_e};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::{to_raw_value, RawValue};
use std::{borrow::Cow, sync::Arc};

lazy_static::lazy_static! {
    pub static ref EMPTY_STRING: ExposableString = ExposableString::from_str_ref("");
    pub static ref DEFAULT_RULE: ExposableString = ExposableString::from_str_ref("default");
    pub static ref DISABLED_RULE: ExposableString = ExposableString::from_str_ref("disabled");
}

const TAG: &str = "ExposableString";

#[derive(Debug, Clone)]
pub struct ExposableString {
    pub hash_value: u64,

    // DO NOT MAKE "pub" OR FIRED
    raw_value: Arc<Box<RawValue>>,
}

impl ExposableString {
    pub fn from_str_parts(parts: &[&str]) -> Self {
        let mut value = String::new();
        for v in parts {
            value.push_str(v);
        }

        Self::from_str_ref(&value)
    }

    pub fn from_str_ref(value: &str) -> Self {
        let hash_value = ahash_str(value);

        let raw_value: Box<RawValue> = match to_raw_value(value) {
            Ok(raw_value) => raw_value,
            Err(e) => {
                log_e!(
                    TAG,
                    "Failed to convert string to raw value: {}, {}",
                    e,
                    value
                );
                return EMPTY_STRING.clone();
            }
        };

        Self {
            raw_value: Arc::new(raw_value),
            hash_value,
        }
    }

    pub fn from_raw_value(raw_value: Box<RawValue>) -> Self {
        let hash_value = ahash_str(str_from_raw_value(&raw_value));
        Self {
            raw_value: Arc::new(raw_value),
            hash_value,
        }
    }

    pub fn as_str(&self) -> &str {
        str_from_raw_value(&self.raw_value)
    }

    /// Clones the value out of the Arc. This is not performant.
    /// Please only use this if we are giving a value to a caller outside of this library.
    /// Consider using the `as_str` method instead.
    pub fn unperformant_to_string(&self) -> String {
        self.as_str().to_owned()
    }
}

impl Default for ExposableString {
    fn default() -> Self {
        EMPTY_STRING.clone()
    }
}

impl<'de> Deserialize<'de> for ExposableString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Cow::<'de, str>::deserialize(deserializer)?;
        Ok(Self::from_str_ref(&value))
    }
}

impl Serialize for ExposableString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.raw_value.serialize(serializer)
    }
}

impl PartialEq for ExposableString {
    fn eq(&self, other: &Self) -> bool {
        self.hash_value == other.hash_value
    }
}

impl Eq for ExposableString {}

fn str_from_raw_value(raw_value: &RawValue) -> &str {
    let value = raw_value.get();
    if value.len() >= 2 {
        &value[1..value.len() - 1]
    } else {
        ""
    }
}
