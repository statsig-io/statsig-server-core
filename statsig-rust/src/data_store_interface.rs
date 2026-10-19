use std::fmt::Display;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{hashing::HashUtil, StatsigErr, StatsigOptions};

pub enum RequestPath {
    RulesetsV2,
    RulesetsV1,
    IDListsV1,
    IDList,
}

/// Logical DataStore schemas, not the HTTP paths or individual cache keys.
/// `IDListSnapshot` includes the manifest and every named child list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataStoreFeature {
    ConfigSpecs,
    IDListSnapshot,
    IDListSnapshotWrite,
}

impl Display for DataStoreFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::ConfigSpecs => "config_specs",
            Self::IDListSnapshot => "id_list_snapshot",
            Self::IDListSnapshotWrite => "id_list_snapshot_write",
        };
        write!(f, "{value}")
    }
}

impl Display for RequestPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            RequestPath::IDListsV1 => "/v1/get_id_lists",
            RequestPath::IDList => "id_list",
            RequestPath::RulesetsV2 => "/v2/download_config_specs",
            RequestPath::RulesetsV1 => "/v1/download_config_specs",
        };
        write!(f, "{value}")
    }
}

pub enum CompressFormat {
    PlainText,
    Gzip,
    StatsigBr,
}

impl Display for CompressFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            CompressFormat::PlainText => "plain_text",
            CompressFormat::Gzip => "gzip",
            CompressFormat::StatsigBr => "statsig-br",
        };
        write!(f, "{value}")
    }
}

#[derive(Deserialize, Serialize)]
pub struct DataStoreResponse {
    pub result: Option<String>,
    pub time: Option<u64>,
}

#[derive(Deserialize, Serialize)]
pub struct DataStoreBytesResponse {
    pub result: Option<Vec<u8>>,
    pub time: Option<u64>,
}

#[derive(Clone, Debug)]
pub(crate) struct DataStoreCacheKeys {
    pub plain_text: String,
    pub statsig_br: String,
}

impl DataStoreCacheKeys {
    pub(crate) fn from_selected_key(key: &str) -> Self {
        Self {
            plain_text: data_store_key_with_compress_format(key, CompressFormat::PlainText),
            statsig_br: data_store_key_with_compress_format(key, CompressFormat::StatsigBr),
        }
    }
}

#[allow(
    clippy::double_must_use,
    reason = "async_trait adds must_use to methods returning an already-must-use boxed Future"
)]
#[async_trait]
pub trait DataStoreTrait: Send + Sync {
    async fn initialize(&self) -> Result<(), StatsigErr>;
    async fn shutdown(&self) -> Result<(), StatsigErr>;
    async fn get(&self, key: &str) -> Result<DataStoreResponse, StatsigErr>;
    async fn set(&self, key: &str, value: &str, time: Option<u64>) -> Result<(), StatsigErr>;
    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        time: Option<u64>,
    ) -> Result<(), StatsigErr> {
        let _ = (key, value, time);
        Err(StatsigErr::BytesNotImplemented)
    }

    async fn get_bytes(&self, key: &str) -> Result<DataStoreBytesResponse, StatsigErr> {
        let _ = key;
        Err(StatsigErr::BytesNotImplemented)
    }

    async fn support_polling_updates_for(&self, path: RequestPath) -> bool;

    /// Existing adapters support config specs but must explicitly opt in to
    /// the complete ID-list snapshot key schema before the SDK reads any ID-list
    /// keys. HTTP snapshot write-back additionally requires IDListSnapshotWrite.
    async fn supports_feature(&self, feature: DataStoreFeature) -> bool {
        matches!(feature, DataStoreFeature::ConfigSpecs)
    }

    /// `None` means the adapter has no ID-list polling preference. An opted-in
    /// snapshot then polls the DataStore by default; `Some(false)` explicitly
    /// assigns scheduled refreshes to HTTP while startup still tries the cache.
    async fn support_polling_updates_for_feature(
        &self,
        _feature: DataStoreFeature,
    ) -> Option<bool> {
        None
    }
}

#[derive(Clone, Debug, Default)]
pub enum DataStoreKeyVersion {
    #[default]
    V2Hashed,
    V3HumanReadable,
}

impl From<&str> for DataStoreKeyVersion {
    fn from(level: &str) -> Self {
        match level.to_lowercase().as_str() {
            "v2" | "2" => DataStoreKeyVersion::V2Hashed,
            "v3" | "3" => DataStoreKeyVersion::V3HumanReadable,
            _ => DataStoreKeyVersion::default(),
        }
    }
}

#[must_use]
pub(crate) fn get_data_store_key(
    path: RequestPath,
    sdk_key: &str,
    hashing: &HashUtil,
    options: &StatsigOptions,
) -> String {
    get_data_store_key_for_path(&path.to_string(), sdk_key, hashing, options)
}

fn get_data_store_key_for_path(
    path: &str,
    sdk_key: &str,
    hashing: &HashUtil,
    options: &StatsigOptions,
) -> String {
    let key = match options
        .data_store_key_schema_version
        .clone()
        .unwrap_or_default()
    {
        DataStoreKeyVersion::V3HumanReadable => {
            let mut key = sdk_key.to_string();
            key.truncate(20);
            key
        }
        DataStoreKeyVersion::V2Hashed => hashing.hash(sdk_key, &crate::HashAlgorithm::Sha256),
    };

    format!("statsig|{path}|{}|{key}", CompressFormat::PlainText)
}

#[must_use]
pub(crate) fn get_id_lists_manifest_data_store_key(
    sdk_key: &str,
    hashing: &HashUtil,
    options: &StatsigOptions,
) -> String {
    get_data_store_key(RequestPath::IDListsV1, sdk_key, hashing, options)
}

/// Builds the key used by Legacy Node for a complete cached ID-list snapshot.
///
/// The list name is part of the path (`id_list::<name>`), rather than a suffix
/// on the fully formatted cache key. This distinction is observable for both
/// the V2 hashed and V3 human-readable key schemas.
#[must_use]
pub(crate) fn get_id_list_data_store_key(
    list_name: &str,
    sdk_key: &str,
    hashing: &HashUtil,
    options: &StatsigOptions,
) -> String {
    get_data_store_key_for_path(
        &format!("{}::{list_name}", RequestPath::IDList),
        sdk_key,
        hashing,
        options,
    )
}

fn data_store_key_with_compress_format(key: &str, compress_format: CompressFormat) -> String {
    let target = format!("|{compress_format}|");

    [
        CompressFormat::PlainText,
        CompressFormat::Gzip,
        CompressFormat::StatsigBr,
    ]
    .into_iter()
    .map(|format| format!("|{format}|"))
    .find_map(|current| {
        key.contains(&current)
            .then(|| key.replacen(&current, &target, 1))
    })
    .unwrap_or_else(|| key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LegacyConfigSpecsDataStore;

    #[async_trait]
    impl DataStoreTrait for LegacyConfigSpecsDataStore {
        async fn initialize(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn get(&self, _key: &str) -> Result<DataStoreResponse, StatsigErr> {
            Ok(DataStoreResponse {
                result: None,
                time: None,
            })
        }

        async fn set(
            &self,
            _key: &str,
            _value: &str,
            _time: Option<u64>,
        ) -> Result<(), StatsigErr> {
            Ok(())
        }

        async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn old_data_store_defaults_to_config_specs_only() {
        let store = LegacyConfigSpecsDataStore;
        assert!(store.supports_feature(DataStoreFeature::ConfigSpecs).await);
        assert!(
            !store
                .supports_feature(DataStoreFeature::IDListSnapshot)
                .await
        );
        assert!(
            !store
                .supports_feature(DataStoreFeature::IDListSnapshotWrite)
                .await
        );
        assert_eq!(
            store
                .support_polling_updates_for_feature(DataStoreFeature::IDListSnapshot)
                .await,
            None
        );
        assert_eq!(DataStoreFeature::ConfigSpecs.to_string(), "config_specs");
        assert_eq!(
            DataStoreFeature::IDListSnapshot.to_string(),
            "id_list_snapshot"
        );
        assert_eq!(
            DataStoreFeature::IDListSnapshotWrite.to_string(),
            "id_list_snapshot_write"
        );
    }

    #[test]
    fn id_list_keys_match_legacy_v2_contract() {
        let hashing = HashUtil::new();
        let options = StatsigOptions::default();
        let sdk_key = "secret-key";
        let legacy_hash = "hdvhXXXvkwjHrg8zx6MkzG9L9Rmi7S8wJ70zwUCk+ao=";

        assert_eq!(
            get_id_lists_manifest_data_store_key(sdk_key, &hashing, &options),
            format!("statsig|/v1/get_id_lists|plain_text|{legacy_hash}")
        );
        assert_eq!(
            get_id_list_data_store_key("paid-users", sdk_key, &hashing, &options),
            format!("statsig|id_list::paid-users|plain_text|{legacy_hash}")
        );
    }

    #[test]
    fn id_list_keys_follow_existing_v3_schema() {
        let hashing = HashUtil::new();
        let options = StatsigOptions {
            data_store_key_schema_version: Some(DataStoreKeyVersion::V3HumanReadable),
            ..StatsigOptions::default()
        };
        let sdk_key = "secret-key-with-more-than-twenty-characters";
        let expected_key_component = "secret-key-with-more";

        assert_eq!(
            get_id_lists_manifest_data_store_key(sdk_key, &hashing, &options),
            format!("statsig|/v1/get_id_lists|plain_text|{expected_key_component}")
        );
        assert_eq!(
            get_id_list_data_store_key("paid-users", sdk_key, &hashing, &options),
            format!("statsig|id_list::paid-users|plain_text|{expected_key_component}")
        );
    }
}
