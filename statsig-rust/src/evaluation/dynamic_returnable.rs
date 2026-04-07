use std::{borrow::Cow, collections::HashMap, sync::Arc};

use rkyv::{collections::swiss_table::ArchivedHashMap, string::ArchivedString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{
    value::{to_raw_value, RawValue},
    Value as JsonValue,
};

use crate::{
    evaluation::rkyv_value::{ArchivedRkyvValue, RkyvValue},
    interned_values::InternedStore,
    log_e,
};

const TAG: &str = "DynamicReturnable";

lazy_static::lazy_static! {
    static ref EMPTY_DYNAMIC_RETURNABLE: DynamicReturnable = DynamicReturnable {
        hash: 0,
        value: DynamicReturnableValue::Null,
    };

    static ref TRUE_DYNAMIC_RETURNABLE: DynamicReturnable = DynamicReturnable {
        hash: 0,
        value: DynamicReturnableValue::Bool(true),
    };

    static ref FALSE_DYNAMIC_RETURNABLE: DynamicReturnable = DynamicReturnable {
        hash: 0,
        value: DynamicReturnableValue::Bool(false),
    };

}

#[derive(Clone, PartialEq, Debug)]
pub struct DynamicReturnable {
    pub hash: u64,
    pub value: DynamicReturnableValue,
}

impl DynamicReturnable {
    pub fn empty() -> Self {
        EMPTY_DYNAMIC_RETURNABLE.clone()
    }

    pub fn from_bool(value: bool) -> Self {
        if value {
            TRUE_DYNAMIC_RETURNABLE.clone()
        } else {
            FALSE_DYNAMIC_RETURNABLE.clone()
        }
    }

    pub fn from_map(value: HashMap<String, JsonValue>) -> Self {
        let raw_value = match to_raw_value(&value) {
            Ok(raw_value) => raw_value,
            Err(e) => {
                log_e!(TAG, "Failed to convert map to raw value: {}", e);
                return Self::empty();
            }
        };

        InternedStore::get_or_intern_returnable(Cow::Owned(raw_value))
    }

    pub fn get_bool(&self) -> Option<bool> {
        match self.value {
            DynamicReturnableValue::Bool(value) => Some(value),
            _ => None,
        }
    }

    pub fn get_json_archived_ref(
        &self,
    ) -> Option<&'static ArchivedHashMap<ArchivedString, ArchivedRkyvValue>> {
        match self.value {
            DynamicReturnableValue::JsonArchived(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_json_pointer_ref(&self) -> Option<&HashMap<String, RkyvValue>> {
        match &self.value {
            DynamicReturnableValue::JsonPointer(v) => Some(v.as_ref()),
            DynamicReturnableValue::JsonStatic(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_json(&self) -> Option<HashMap<String, JsonValue>> {
        match &self.value {
            DynamicReturnableValue::JsonPointer(v) => rkyv_hashmap_to_owned_json(v.as_ref()),
            DynamicReturnableValue::JsonStatic(v) => rkyv_hashmap_to_owned_json(v),
            DynamicReturnableValue::JsonArchived(v) => archived_hashmap_to_owned(v).map_or_else(
                |e| {
                    log_e!(TAG, "Failed to convert archived json. Error: {}", e);
                    None
                },
                Some,
            ),
            _ => None,
        }
    }

    pub fn get_hash(&self) -> u64 {
        self.hash
    }
}

impl<'de> Deserialize<'de> for DynamicReturnable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value_ref: Cow<RawValue> = Deserialize::deserialize(deserializer)?;
        Ok(InternedStore::get_or_intern_returnable(raw_value_ref))
    }
}

impl Serialize for DynamicReturnable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.value {
            DynamicReturnableValue::JsonPointer(raw) => raw.serialize(serializer),
            DynamicReturnableValue::JsonStatic(raw) => raw.serialize(serializer),
            DynamicReturnableValue::JsonArchived(raw) => {
                let owned = archived_hashmap_to_owned(raw).map_err(serde::ser::Error::custom)?;
                owned.serialize(serializer)
            }
            DynamicReturnableValue::Null => serializer.serialize_none(),
            DynamicReturnableValue::Bool(value) => serializer.serialize_bool(*value),
        }
    }
}

impl Drop for DynamicReturnable {
    fn drop(&mut self) {
        self.value = DynamicReturnableValue::Null;
        InternedStore::release_returnable(self.hash);
    }
}

// ------------------------------------------------------------------------------- [ DynamicReturnableValue ]

#[derive(Clone, Debug)]
pub enum DynamicReturnableValue {
    Null,
    Bool(bool),
    JsonPointer(Arc<HashMap<String, RkyvValue>>),
    JsonStatic(&'static HashMap<String, RkyvValue>),
    JsonArchived(&'static ArchivedHashMap<ArchivedString, ArchivedRkyvValue>),
}

impl PartialEq for DynamicReturnableValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DynamicReturnableValue::Null, DynamicReturnableValue::Null) => return true,
            (DynamicReturnableValue::Bool(a), DynamicReturnableValue::Bool(b)) => return *a == *b,
            _ => {}
        };

        if let DynamicReturnableValue::JsonPointer(a) = self {
            match other {
                DynamicReturnableValue::JsonPointer(b) => return a.as_ref() == b.as_ref(),
                DynamicReturnableValue::JsonStatic(b) => return a.as_ref() == *b,
                DynamicReturnableValue::JsonArchived(b) => return eq_check(b, a.as_ref()),
                _ => return false,
            }
        }

        if let DynamicReturnableValue::JsonStatic(a) = self {
            match other {
                DynamicReturnableValue::JsonPointer(b) => return *a == b.as_ref(),
                DynamicReturnableValue::JsonStatic(b) => return a == b,
                DynamicReturnableValue::JsonArchived(b) => return eq_check(b, a),
                _ => return false,
            }
        }

        if let DynamicReturnableValue::JsonArchived(a) = self {
            match other {
                DynamicReturnableValue::JsonPointer(b) => return eq_check(a, b.as_ref()),
                DynamicReturnableValue::JsonStatic(b) => return eq_check(a, b),
                DynamicReturnableValue::JsonArchived(b) => return a == b,
                _ => return false,
            }
        }

        false
    }
}

// ------------------------------------------------------------------------------- [ Rkyv Helper ]

fn eq_check(
    left: &ArchivedHashMap<ArchivedString, ArchivedRkyvValue>,
    right: &HashMap<String, RkyvValue>,
) -> bool {
    for (key, value) in left.iter() {
        match right.get_key_value(key.as_str()) {
            Some((left_key, left_value)) => {
                if left_key != key {
                    return false;
                }
                if left_value != value {
                    return false;
                }
            }
            None => return false,
        };
    }
    true
}

fn rkyv_hashmap_to_owned_json(
    raw: &HashMap<String, RkyvValue>,
) -> Option<HashMap<String, JsonValue>> {
    match serde_json::to_value(raw) {
        Ok(JsonValue::Object(o)) => Some(o.into_iter().collect()),
        Ok(_) => None,
        Err(e) => {
            log_e!(TAG, "Failed to convert json. Error: {}", e);
            None
        }
    }
}

fn archived_hashmap_to_owned(
    raw: &'static ArchivedHashMap<ArchivedString, ArchivedRkyvValue>,
) -> Result<HashMap<String, JsonValue>, serde_json::Error> {
    let mut taken: HashMap<String, JsonValue> = HashMap::new();
    for (key, value) in raw.iter() {
        taken.insert(key.as_str().to_string(), serde_json::to_value(value)?);
    }

    Ok(taken)
}
