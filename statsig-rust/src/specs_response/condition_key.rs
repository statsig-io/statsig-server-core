use std::hash::{Hash, Hasher};

use serde::{Deserialize, Deserializer, Serialize};

/**
DCS v2 stores conditions in a map. This means that for each evaluation, we do a lookup from
the Rule.conditions vec to the top level condition_map.

Given that we know the keys are a djb2 hash, we can convert them to u64 numbers during deserialization.
Having these keys as numbers makes the lookup a lot faster as the "hash" for the HashMap is just the u64 integer.

I have included fallback support for Strings in-case the DCS payload changes in the future
*/

#[derive(PartialEq, Eq, Debug)]
pub struct ConditionKey {
    pub u64_value: Option<u64>,
    pub string_value: String,
}

impl<'de> Deserialize<'de> for ConditionKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_value = String::deserialize(deserializer)?;
        let u64_value = string_value.parse::<u64>().ok();
        Ok(ConditionKey {
            u64_value,
            string_value,
        })
    }
}

impl Serialize for ConditionKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.string_value)
    }
}

impl Hash for ConditionKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(u64_value) = self.u64_value {
            u64_value.hash(state);
        } else {
            self.string_value.hash(state);
        }
    }
}
