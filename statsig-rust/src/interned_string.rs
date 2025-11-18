use std::{
    borrow::Cow,
    fmt::Display,
    hash::{Hash, Hasher},
    ops::Deref,
    sync::Arc,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

use crate::{
    evaluation::dynamic_string::DynamicString, hashing, impl_interned_value,
    interned_value_store::FromRawValue, log_e,
};

lazy_static::lazy_static! {
    static ref EMPTY: InternedString = InternedString {
        hash: 0,
        value: Arc::new(String::new()),
    };

    static ref TRUE_STRING: InternedString = InternedString::from_string("true".to_string());
    static ref FALSE_STRING: InternedString = InternedString::from_string("false".to_string());
}

const TAG: &str = "InternedString";

#[derive(Clone, Debug, Eq)]
pub struct InternedString {
    pub hash: u64,
    pub value: Arc<String>,
}

#[macro_export]
macro_rules! interned_str {
    // String literal -> from_str_ref
    ($value:literal) => {
        InternedString::from_str_ref($value)
    };
    // Bool -> from_bool
    (bool: $value:literal) => {
        InternedString::from_bool($value)
    };
    // String (owned) -> from_string
    ($value:expr) => {
        InternedString::from_string($value)
    };
    // DynamicString -> from_dynamic_string
    ($value:expr) => {
        InternedString::from_dynamic_string($value)
    };
    // String parts (slice) -> from_str_parts
    ( $value:expr) => {
        InternedString::from_str_parts($value)
    };
}

impl InternedString {
    pub fn from_str_ref(value: &str) -> Self {
        let hash = hashing::hash_one(value);
        let value = InternedString::get_or_create_memoized_string(hash, Cow::Borrowed(value));
        Self { hash, value }
    }

    pub fn from_string(value: String) -> Self {
        let hash = hashing::hash_one(value.as_str());
        let value = InternedString::get_or_create_memoized_string(hash, Cow::Owned(value));
        Self { hash, value }
    }

    pub fn from_bool(value: bool) -> Self {
        if value {
            TRUE_STRING.clone()
        } else {
            FALSE_STRING.clone()
        }
    }

    pub fn from_dynamic_string(value: &DynamicString) -> Self {
        let hash = value.hash_value;
        let value =
            InternedString::get_or_create_memoized_string(hash, Cow::Borrowed(&value.value));
        Self { hash, value }
    }

    pub fn from_str_parts(parts: &[&str]) -> Self {
        let mut value = String::new();
        for v in parts {
            value.push_str(v);
        }

        Self::from_string(value)
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    /// Clones the value out of the Arc. This is not performant.
    /// Please only use this if we are giving a value to a caller outside of this library.
    pub fn unperformant_to_string(&self) -> String {
        self.value.to_string()
    }

    pub fn empty_ref() -> &'static Self {
        &EMPTY
    }

    pub fn empty() -> Self {
        EMPTY.clone()
    }

    fn get_or_create_memoized_string(hash: u64, input: Cow<'_, str>) -> Arc<String> {
        if let Some(value) = INTERNED_STORE.try_get_interned_value(hash) {
            return value;
        }

        let value = input.to_string();
        let value_arc = Arc::new(value);
        INTERNED_STORE.set_interned_value(hash, &value_arc);
        value_arc
    }
}

impl_interned_value!(InternedString, String);

impl<'de> Deserialize<'de> for InternedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum CowString<'a> {
            #[serde(borrow)]
            Borrowed(&'a str), // zero-copy for serde_json::from_str
            Owned(String), // allocation for serde_json::from_value
        }

        let raw: CowString<'de> = CowString::deserialize(deserializer)?;
        match raw {
            CowString::Borrowed(raw) => {
                let hash = hashing::hash_one(raw);
                let value = InternedString::get_or_create_memoized_string(hash, Cow::Borrowed(raw));
                Ok(InternedString { hash, value })
            }
            CowString::Owned(raw) => {
                let hash = hashing::hash_one(raw.as_str());
                let value = InternedString::get_or_create_memoized_string(hash, Cow::Owned(raw));
                Ok(InternedString { hash, value })
            }
        }
    }
}

impl Serialize for InternedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl FromRawValue for String {
    fn from_raw_value(raw_value: Cow<'_, RawValue>) -> Self {
        match serde_json::from_str(raw_value.get()) {
            Ok(value) => value,
            Err(e) => {
                log_e!(TAG, "Failed to convert raw value to String: {}", e);
                String::new()
            }
        }
    }
}

impl PartialEq for InternedString {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Hash for InternedString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq<&str> for InternedString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<str> for InternedString {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<String> for InternedString {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == *other
    }
}

impl Deref for InternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Display for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl Default for InternedString {
    fn default() -> Self {
        EMPTY.clone()
    }
}
