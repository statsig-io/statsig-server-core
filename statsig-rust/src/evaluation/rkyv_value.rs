use std::collections::HashMap;

use rkyv::Archive;
use serde::ser::{SerializeMap, SerializeSeq};

// A bridging layer between Serde and Rkyv.
// Based on Rkyv Examples: https://github.com/rkyv/rkyv/blob/main/rkyv/examples/json_like_schema.rs
#[derive(
    Archive, Debug, rkyv::Deserialize, rkyv::Serialize, Clone, serde::Serialize, serde::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator,
    __S::Error: rkyv::rancor::Source,
))]
#[rkyv(deserialize_bounds(__D::Error: rkyv::rancor::Source))]
#[rkyv(bytecheck(
    bounds(
        __C: rkyv::validation::ArchiveContext,
    )
))]
#[rkyv(derive(Debug))]
#[serde(untagged)]
pub enum RkyvValue {
    Null,
    Bool(bool),
    Number(RkyvNumber),
    String(String),
    Array(#[rkyv(omit_bounds)] Vec<RkyvValue>),
    Object(#[rkyv(omit_bounds)] HashMap<String, RkyvValue>),
}

impl serde::Serialize for ArchivedRkyvValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ArchivedRkyvValue::Null => serializer.serialize_none(),
            ArchivedRkyvValue::Bool(b) => serializer.serialize_bool(*b),
            ArchivedRkyvValue::Number(n) => n.serialize(serializer),
            ArchivedRkyvValue::String(s) => serializer.serialize_str(s),
            ArchivedRkyvValue::Array(a) => {
                let mut seq = serializer.serialize_seq(Some(a.len()))?;
                for element in a.iter() {
                    seq.serialize_element(&element)?;
                }
                seq.end()
            }
            ArchivedRkyvValue::Object(o) => {
                let mut map = serializer.serialize_map(Some(o.len()))?;

                for (k, v) in o.iter() {
                    map.serialize_entry(k.as_str(), v)?;
                }

                map.end()
            }
        }
    }
}

// ------------------------------------------------------------------------------- [ RkyvNumber ]

#[derive(
    Archive, Debug, rkyv::Deserialize, rkyv::Serialize, Clone, serde::Serialize, serde::Deserialize,
)]
#[rkyv(derive(Debug))]
#[serde(untagged)]
pub enum RkyvNumber {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

impl serde::Serialize for ArchivedRkyvNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ArchivedRkyvNumber::PosInt(n) => serializer.serialize_u64(n.to_native()),
            ArchivedRkyvNumber::NegInt(n) => serializer.serialize_i64(n.to_native()),
            ArchivedRkyvNumber::Float(n) => serializer.serialize_f64(n.to_native()),
        }
    }
}
