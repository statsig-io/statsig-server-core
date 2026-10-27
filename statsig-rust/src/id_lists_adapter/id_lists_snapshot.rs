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
    pub(crate) fn list_names(manifest: &str) -> Result<Vec<String>, StatsigErr> {
        Ok(parse_manifest(manifest)?.into_keys().collect())
    }

    pub(crate) fn parse(
        manifest: &str,
        children: &HashMap<String, String>,
    ) -> Result<Self, StatsigErr> {
        let entries = parse_manifest(manifest)?;
        let mut lists = HashMap::with_capacity(entries.len());

        for (name, entry) in entries {
            let child = children.get(&name).ok_or_else(|| {
                snapshot_error(format!(
                    "ID-list child is missing for manifest entry '{name}'"
                ))
            })?;
            // Legacy's manifest size is the remote append-only file cursor, not
            // the byte length of its condensed '+'-only DataStore child.
            let ids = parse_complete_child(&name, child, entry.permits_empty_child)?;
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
            let remote_size = wire_metadata.size;
            let metadata = IdListMetadata {
                name: name.clone(),
                url: wire_metadata.url.unwrap_or_default(),
                file_id: wire_metadata.file_id,
                size: remote_size.unwrap_or(0),
                creation_time: wire_metadata.creation_time.unwrap_or(0),
            };
            if entries
                .insert(
                    name.clone(),
                    ManifestEntry {
                        metadata,
                        // A Legacy producer can write an empty materialized
                        // snapshot even when its remote log has nonzero size.
                        permits_empty_child: true,
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
                "Empty ID-list child for '{name}' is ambiguous without object metadata"
            )))
        };
    }

    let mut ids = HashSet::new();
    let mut lines = child.split('\n').enumerate().peekable();
    while let Some((index, raw_line)) = lines.next() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            if lines.peek().is_none() {
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
            "Empty ID-list child for '{name}' is ambiguous without object metadata"
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
        assert_eq!(array.lists["list-a"].metadata.size, 0);

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
            "+2Tv4fIVX\n\n",
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
    fn object_manifest_allows_present_empty_child_independent_of_remote_size() {
        let children = HashMap::from([("list-a".into(), String::new())]);
        assert!(IdListsSnapshot::parse(r#"["list-a"]"#, &children).is_err());
        assert!(IdListsSnapshot::parse(&object_manifest(42), &HashMap::new()).is_err());

        for remote_size in [0, 42] {
            let snapshot =
                IdListsSnapshot::parse(&object_manifest(remote_size), &children).unwrap();
            assert!(snapshot.lists["list-a"].ids.is_empty());
            assert_eq!(snapshot.lists["list-a"].metadata.size, remote_size);
        }
    }

    #[test]
    fn accepts_legacy_object_metadata_without_confusing_remote_and_cached_size() {
        let child = "+2Tv4fIVX\n";
        let children = HashMap::from([("list-a".into(), child.into())]);

        let snapshot = IdListsSnapshot::parse(r#"{"list-a":{}}"#, &children).unwrap();
        assert_eq!(snapshot.lists["list-a"].metadata.name, "list-a");
        assert_eq!(snapshot.lists["list-a"].metadata.size, 0);

        for remote_size in [0, 1, 99] {
            let manifest = object_manifest(remote_size);
            let snapshot = IdListsSnapshot::parse(&manifest, &children).unwrap();
            assert_eq!(snapshot.lists["list-a"].metadata.size, remote_size);
            assert_eq!(snapshot.lists["list-a"].ids.len(), 1);
        }
    }
}
