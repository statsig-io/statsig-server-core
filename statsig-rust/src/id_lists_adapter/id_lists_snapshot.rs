use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::de::{Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use super::{IdList, IdListMetadata};
use crate::StatsigErr;

pub(crate) struct IdListsSnapshot {
    pub(crate) lists: HashMap<String, IdList>,
}

struct ManifestEntry {
    metadata: IdListMetadata,
    declared_size: Option<u64>,
    permits_empty_child: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ObjectManifestMetadata {
    name: Option<String>,
    url: Option<String>,
    #[serde(rename = "fileID")]
    file_id: Option<String>,
    size: Option<u64>,
    creation_time: Option<i64>,
}

impl IdListsSnapshot {
    pub(crate) fn parse(
        manifest: &str,
        children: &HashMap<String, String>,
    ) -> Result<Self, StatsigErr> {
        let entries = parse_manifest(manifest)?;
        let mut lists = HashMap::with_capacity(entries.len());

        for (name, mut entry) in entries {
            let child = children.get(&name).ok_or_else(|| {
                snapshot_error(format!(
                    "ID-list child is missing for manifest entry '{name}'"
                ))
            })?;
            if let Some(declared_size) = entry.declared_size {
                if child.len() as u64 != declared_size {
                    return Err(snapshot_error(format!(
                        "ID-list child '{name}' has {} bytes, expected {declared_size}",
                        child.len()
                    )));
                }
            }
            let ids = parse_complete_child(&name, child, entry.permits_empty_child)?;
            if entry.declared_size.is_none() {
                entry.metadata.size = child.len() as u64;
            }
            lists.insert(name, IdList::from_complete_snapshot(entry.metadata, ids));
        }

        Ok(Self { lists })
    }
}

fn parse_manifest(manifest: &str) -> Result<HashMap<String, ManifestEntry>, StatsigErr> {
    let mut deserializer = serde_json::Deserializer::from_str(manifest);
    let entries = ManifestEntries::deserialize(&mut deserializer)
        .map_err(|error| snapshot_error(format!("Invalid ID-list manifest: {error}")))?;
    deserializer
        .end()
        .map_err(|error| snapshot_error(format!("Invalid ID-list manifest: {error}")))?;
    Ok(entries.0)
}

struct ManifestEntries(HashMap<String, ManifestEntry>);

impl<'de> Deserialize<'de> for ManifestEntries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ManifestVisitor)
    }
}

struct ManifestVisitor;

impl<'de> Visitor<'de> for ManifestVisitor {
    type Value = ManifestEntries;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an array of ID-list names or an object of ID-list metadata")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut entries = HashMap::new();
        while let Some(name) = sequence.next_element::<String>()? {
            if name.is_empty() {
                return Err(serde::de::Error::custom(
                    "array manifest names must be non-empty strings",
                ));
            }
            let metadata = IdListMetadata {
                name: name.clone(),
                url: String::new(),
                file_id: None,
                size: 0,
                creation_time: 0,
            };
            if entries
                .insert(
                    name.clone(),
                    ManifestEntry {
                        metadata,
                        declared_size: None,
                        permits_empty_child: false,
                    },
                )
                .is_some()
            {
                return Err(serde::de::Error::custom(format!(
                    "duplicate ID-list manifest entry '{name}'"
                )));
            }
        }
        Ok(ManifestEntries(entries))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = HashMap::new();
        while let Some((name, wire_metadata)) =
            map.next_entry::<String, ObjectManifestMetadata>()?
        {
            if name.is_empty() {
                return Err(serde::de::Error::custom(
                    "object manifest names must be non-empty",
                ));
            }
            if let Some(embedded_name) = wire_metadata.name.as_deref() {
                if embedded_name != name {
                    return Err(serde::de::Error::custom(format!(
                        "ID-list metadata name '{embedded_name}' does not match manifest key '{name}'"
                    )));
                }
            }
            let declared_size = wire_metadata.size;
            let permits_empty_child = declared_size == Some(0);
            let metadata = IdListMetadata {
                name: name.clone(),
                url: wire_metadata.url.unwrap_or_default(),
                file_id: wire_metadata.file_id,
                size: declared_size.unwrap_or(0),
                creation_time: wire_metadata.creation_time.unwrap_or(0),
            };
            if entries
                .insert(
                    name.clone(),
                    ManifestEntry {
                        metadata,
                        declared_size,
                        permits_empty_child,
                    },
                )
                .is_some()
            {
                return Err(serde::de::Error::custom(format!(
                    "duplicate ID-list manifest entry '{name}'"
                )));
            }
        }
        Ok(ManifestEntries(entries))
    }
}

fn parse_complete_child(
    name: &str,
    child: &str,
    permits_empty_child: bool,
) -> Result<HashSet<String>, StatsigErr> {
    if child.is_empty() {
        return if permits_empty_child {
            Ok(HashSet::new())
        } else {
            Err(snapshot_error(format!(
                "Empty ID-list child for '{name}' is ambiguous without size 0 metadata"
            )))
        };
    }

    let mut ids = HashSet::new();
    let lines: Vec<_> = child.split('\n').collect();
    for (index, raw_line) in lines.iter().enumerate() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            if index + 1 == lines.len() {
                continue;
            }
            return Err(snapshot_error(format!(
                "ID-list child '{name}' line {} is empty",
                index + 1
            )));
        }
        let id = line.strip_prefix('+').ok_or_else(|| {
            snapshot_error(format!(
                "ID-list child '{name}' line {} is not a complete-snapshot '+' record",
                index + 1
            ))
        })?;
        if !is_valid_hash(id) {
            return Err(snapshot_error(format!(
                "ID-list child '{name}' line {} has an invalid hash",
                index + 1
            )));
        }
        if !ids.insert(id.to_string()) {
            return Err(snapshot_error(format!(
                "ID-list child '{name}' contains duplicate hash records"
            )));
        }
    }

    if ids.is_empty() && !permits_empty_child {
        return Err(snapshot_error(format!(
            "Empty ID-list child for '{name}' is ambiguous without size 0 metadata"
        )));
    }
    Ok(ids)
}

fn is_valid_hash(value: &str) -> bool {
    value.len() == 8
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
}

fn snapshot_error(message: String) -> StatsigErr {
    StatsigErr::DataStoreFailure(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_manifest(size: u64) -> String {
        format!(
            r#"{{"list-a":{{"url":"","fileID":"generation-1","size":{size},"creationTime":1}}}}"#
        )
    }

    #[test]
    fn parses_array_and_object_manifests_and_crlf_children() {
        let children = HashMap::from([("list-a".into(), "+2Tv4fIVX\r\n+/WGgOvT3\r\n".into())]);

        let array = IdListsSnapshot::parse(r#"["list-a"]"#, &children).unwrap();
        assert_eq!(array.lists["list-a"].ids.len(), 2);
        assert_eq!(
            array.lists["list-a"].metadata.size,
            children["list-a"].len() as u64
        );

        let object = IdListsSnapshot::parse(&object_manifest(22), &children).unwrap();
        assert_eq!(
            object.lists["list-a"].metadata.file_id.as_deref(),
            Some("generation-1")
        );
        assert_eq!(object.lists["list-a"].ids.len(), 2);
    }

    #[test]
    fn rejects_invalid_manifest_shapes_and_duplicates() {
        let children = HashMap::new();
        for manifest in [
            "null",
            "1",
            r#"["a","a"]"#,
            r#"[1]"#,
            r#"{"a":{"name":"a","url":"","size":0,"creationTime":0},"a":{"name":"a","url":"","size":0,"creationTime":0}}"#,
            r#"{"a":{"name":"other"}}"#,
        ] {
            assert!(
                IdListsSnapshot::parse(manifest, &children).is_err(),
                "{manifest}"
            );
        }
    }

    #[test]
    fn rejects_missing_partial_malformed_and_duplicate_children() {
        assert!(IdListsSnapshot::parse(r#"["a"]"#, &HashMap::new()).is_err());
        for child in [
            "-2Tv4fIVX",
            "?2Tv4fIVX",
            "+",
            "+abc xyz",
            "+short7!",
            "+2Tv4fIVX\n\n+34jaY4Sb",
            "+2Tv4fIVX\n+2Tv4fIVX",
        ] {
            let children = HashMap::from([("a".into(), child.into())]);
            assert!(
                IdListsSnapshot::parse(r#"["a"]"#, &children).is_err(),
                "{child}"
            );
        }
    }

    #[test]
    fn only_object_metadata_with_zero_size_allows_empty_child() {
        let children = HashMap::from([("list-a".into(), String::new())]);
        assert!(IdListsSnapshot::parse(r#"["list-a"]"#, &children).is_err());

        let snapshot = IdListsSnapshot::parse(&object_manifest(0), &children).unwrap();
        assert!(snapshot.lists["list-a"].ids.is_empty());
    }

    #[test]
    fn accepts_legacy_object_optional_metadata_and_enforces_declared_size() {
        let child = "+2Tv4fIVX\n";
        let children = HashMap::from([("list-a".into(), child.into())]);

        let snapshot = IdListsSnapshot::parse(r#"{"list-a":{}}"#, &children).unwrap();
        assert_eq!(snapshot.lists["list-a"].metadata.name, "list-a");
        assert_eq!(snapshot.lists["list-a"].metadata.size, child.len() as u64);

        for manifest in [
            r#"{"list-a":{"size":1}}"#,
            r#"{"list-a":{"size":99}}"#,
            r#"{"list-a":{"size":0}}"#,
        ] {
            assert!(
                IdListsSnapshot::parse(manifest, &children).is_err(),
                "{manifest}"
            );
        }
    }
}
