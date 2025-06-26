use super::spec_types::Spec;
use crate::{event_logging::exposable_string::ExposableString, hashing::ahash_str};
use ahash::HashMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;

#[derive(Debug, PartialEq)]
pub struct AddressableSpec {
    pub name: ExposableString,
    pub spec: Arc<Spec>,
}

#[derive(Debug, Default, PartialEq)]
pub struct SpecDirectory {
    pub specs: HashMap<u64, AddressableSpec>,
}

impl SpecDirectory {
    pub fn get(&self, name: &str) -> Option<&AddressableSpec> {
        let hash = ahash_str(name);
        self.specs.get(&hash)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.specs.values().map(|v| v.name.as_str())
    }

    pub fn unperformant_keys(&self) -> Vec<String> {
        self.specs
            .values()
            .map(|v| v.name.unperformant_to_string())
            .collect()
    }

    pub fn unperformant_keys_entity_filter(&self, entity_type: &str) -> Vec<String> {
        self.specs
            .iter()
            .filter(|(_, v)| v.spec.entity == entity_type)
            .map(|(_, v)| v.name.unperformant_to_string())
            .collect()
    }
}

impl<'de> Deserialize<'de> for SpecDirectory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let specs = HashMap::<String, Spec>::deserialize(deserializer)?
            .into_iter()
            .map(|(k, v)| {
                let name = ExposableString::from_str_ref(k.as_str());
                let hash = name.hash_value;
                let addressable = AddressableSpec {
                    name,
                    spec: Arc::new(v),
                };

                (hash, addressable)
            })
            .collect();

        Ok(SpecDirectory { specs })
    }
}

impl Serialize for SpecDirectory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let map: HashMap<&str, &Spec> = self
            .specs
            .values()
            .map(|v| (v.name.as_str(), v.spec.as_ref()))
            .collect();

        map.serialize(serializer)
    }
}
