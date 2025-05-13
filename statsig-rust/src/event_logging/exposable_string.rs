use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::hashing::ahash_str;

lazy_static::lazy_static! {
    pub static ref EMPTY_STRING: ExposableString = ExposableString::new(String::new());
    pub static ref DEFAULT_RULE_ID: ExposableString = ExposableString::new(String::from("default"));
    pub static ref DISABLED_RULE_ID: ExposableString = ExposableString::new(String::from("disabled"));
}

#[derive(Debug, Clone)]
pub struct ExposableString {
    pub hash_value: u64,

    // DO NOT MAKE "pub" OR FIRED
    value: Arc<String>,
}

impl ExposableString {
    pub fn new(value: String) -> Self {
        let hash_value = ahash_str(&value);
        Self {
            value: Arc::new(value),
            hash_value,
        }
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn clone_inner(&self) -> Arc<String> {
        self.value.clone()
    }

    pub fn from_arc(value: Arc<String>) -> Self {
        let hash_value = ahash_str(&value);
        Self { value, hash_value }
    }

    /// Clones the value out of the Arc. This is not performant.
    /// Please only use this if we are giving a value to a caller outside of this library.
    /// Consider using the `as_str` or `clone_inner` methods instead.
    pub fn unperformant_to_string(&self) -> String {
        self.value.to_string()
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
        let value = String::deserialize(deserializer)?;
        Ok(Self::new(value))
    }
}

impl Serialize for ExposableString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

impl PartialEq for ExposableString {
    fn eq(&self, other: &Self) -> bool {
        self.hash_value == other.hash_value
    }
}

impl Eq for ExposableString {}
