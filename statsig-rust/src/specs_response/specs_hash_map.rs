use std::sync::Arc;

use ahash::HashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

use crate::{interned_string::InternedString, log_e, specs_response::spec_types::Spec};

const TAG: &str = "SpecsHashMap";

#[derive(PartialEq, Debug, Default)] /* DO_NOT_CLONE */
pub struct SpecsHashMap(pub HashMap<InternedString, SpecPointer>);

impl<'de> Deserialize<'de> for SpecsHashMap {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_values: HashMap<InternedString, Box<RawValue>> =
            Deserialize::deserialize(_deserializer)?;

        let mut result = HashMap::default();
        for (key, raw_value) in raw_values.into_iter() {
            let json_string = raw_value.get();

            let spec = match serde_json::from_str(json_string) {
                Ok(spec) => spec,
                Err(e) => {
                    log_e!(TAG, "Failed to deserialize spec: {}", e);
                    continue;
                }
            };

            result.insert(
                key,
                SpecPointer {
                    inner: Arc::new(spec),
                },
            );
        }

        Ok(SpecsHashMap(result))
    }
}

impl Serialize for SpecsHashMap {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(_serializer)
    }
}

/// Feel free to add more HashMap pass-through methods here as needed.
impl SpecsHashMap {
    pub fn get(&self, key: &InternedString) -> Option<&SpecPointer> {
        self.0.get(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &InternedString> {
        self.0.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&InternedString, &SpecPointer)> {
        self.0.iter()
    }
}

#[derive(PartialEq, Debug)]
pub struct SpecPointer {
    pub inner: Arc<Spec>,
}

impl Serialize for SpecPointer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}
