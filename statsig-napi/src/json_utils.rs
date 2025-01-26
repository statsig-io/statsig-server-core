use serde::de::{MapAccess, Visitor};
use serde::{de, Deserializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

pub fn deserialize_as_str_map(
  json_str: &str,
) -> Result<HashMap<String, String>, serde_json::Error> {
  let deserializer = &mut serde_json::Deserializer::from_str(json_str);
  deserialize_to_string_map(deserializer)
}

fn deserialize_to_string_map<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
  D: Deserializer<'de>,
{
  struct StringMapVisitor;

  impl<'de> Visitor<'de> for StringMapVisitor {
    type Value = HashMap<String, String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
      formatter.write_str("a map with string keys and string/number values")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
      M: MapAccess<'de>,
    {
      let mut result = HashMap::new();

      while let Some((key, value)) = map.next_entry::<String, Value>()? {
        let value_as_string = match value {
          Value::String(s) => s,
          Value::Number(n) => n.to_string(),
          _ => return Err(de::Error::custom("Expected string or number value")),
        };
        result.insert(key, value_as_string);
      }

      Ok(result)
    }
  }

  deserializer.deserialize_map(StringMapVisitor)
}
