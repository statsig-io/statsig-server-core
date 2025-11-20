use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::interned_string::InternedString;

#[derive(Clone, PartialEq, Debug, Default)]
pub struct ExplicitParameters {
    inner: Arc<Vec<InternedString>>,
}

impl ExplicitParameters {
    pub fn from_vec(parameters: Vec<String>) -> Self {
        Self {
            inner: Arc::new(
                parameters
                    .into_iter()
                    .map(InternedString::from_string)
                    .collect(),
            ),
        }
    }

    pub fn contains(&self, parameter: &str) -> bool {
        self.inner.iter().any(|p| p.as_str() == parameter)
    }

    pub fn unperformant_to_vec(&self) -> Vec<String> {
        self.inner
            .iter()
            .map(|p| p.unperformant_to_string())
            .collect()
    }
}

impl<'de> Deserialize<'de> for ExplicitParameters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parameters = Vec::<InternedString>::deserialize(deserializer)?;
        Ok(ExplicitParameters {
            inner: Arc::new(parameters),
        })
    }
}

impl Serialize for ExplicitParameters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}
