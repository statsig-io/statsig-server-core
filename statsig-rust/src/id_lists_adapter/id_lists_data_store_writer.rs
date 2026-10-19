use super::{id_lists_snapshot::IdListsSnapshot, IdList, IdListMetadata};
use crate::data_store_interface::{
    get_id_list_data_store_key, get_id_lists_manifest_data_store_key, DataStoreTrait,
};
use crate::hashing::HashUtil;
use crate::{StatsigErr, StatsigOptions};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

/// Publishes Legacy-compatible, complete ID-list children before the manifest.
/// The manifest's `size` remains the remote append-log cursor, not child length.
pub(crate) struct IdListsDataStoreWriter {
    data_store: Arc<dyn DataStoreTrait>,
    sdk_key: String,
    hashing: HashUtil,
    options: StatsigOptions,
    manifest_key: String,
    persisted_manifest: AsyncMutex<Option<HashMap<String, IdListMetadata>>>,
}

impl IdListsDataStoreWriter {
    pub(crate) fn new(
        sdk_key: &str,
        options: &StatsigOptions,
        data_store: Arc<dyn DataStoreTrait>,
    ) -> Self {
        let hashing = HashUtil::new();
        let manifest_key = get_id_lists_manifest_data_store_key(sdk_key, &hashing, options);
        Self {
            data_store,
            sdk_key: sdk_key.to_string(),
            hashing,
            options: options.clone(),
            manifest_key,
            persisted_manifest: AsyncMutex::new(None),
        }
    }

    pub(crate) async fn write_if_changed(
        &self,
        remote_manifest: &HashMap<String, IdListMetadata>,
        get_snapshot: impl FnOnce() -> Result<HashMap<String, IdList>, StatsigErr> + Send,
    ) -> Result<(), StatsigErr> {
        let mut persisted = self.persisted_manifest.lock().await;
        if persisted.as_ref() == Some(remote_manifest) {
            return Ok(());
        }
        // A partial/cancelled publication can overwrite children of the previous
        // checkpoint. Even a return to that old manifest must then retry.
        *persisted = None;
        let snapshot = get_snapshot()?;
        self.write_snapshot(&snapshot, remote_manifest).await?;
        // Only the final manifest write commits the checkpoint. No member copies,
        // sorting, serialization or timestamp refresh is needed on unchanged ticks.
        *persisted = Some(remote_manifest.clone());
        Ok(())
    }

    pub(crate) async fn write_snapshot(
        &self,
        snapshot: &HashMap<String, IdList>,
        remote_manifest: &HashMap<String, IdListMetadata>,
    ) -> Result<(), StatsigErr> {
        if snapshot.len() != remote_manifest.len() {
            return Err(StatsigErr::DataStoreFailure(
                "ID-list snapshot is incomplete; skipping DataStore write".to_string(),
            ));
        }

        let mut names: Vec<&String> = remote_manifest.keys().collect();
        names.sort_unstable();
        let timestamp = Utc::now().timestamp_millis() as u64;
        let mut children = HashMap::with_capacity(names.len());

        for &name in &names {
            let expected = &remote_manifest[name];
            let list = snapshot.get(name).ok_or_else(|| {
                StatsigErr::DataStoreFailure(format!(
                    "ID-list '{name}' is missing from the applied snapshot"
                ))
            })?;
            let actual = &list.metadata;
            if actual.size != expected.size
                || actual.file_id != expected.file_id
                || actual.creation_time != expected.creation_time
            {
                return Err(StatsigErr::DataStoreFailure(format!(
                    "ID-list '{name}' has not reached the remote manifest cursor"
                )));
            }

            let mut ids: Vec<&String> = list.ids.iter().collect();
            ids.sort_unstable();
            let mut child = String::new();
            for id in ids {
                child.push('+');
                child.push_str(id);
                child.push('\n');
            }
            children.insert(name.clone(), child);
        }

        let manifest = serde_json::to_string(remote_manifest)
            .map_err(|e| StatsigErr::SerializationError(e.to_string()))?;
        IdListsSnapshot::parse(&manifest, &children)?;
        for name in names {
            let key = get_id_list_data_store_key(name, &self.sdk_key, &self.hashing, &self.options);
            self.data_store
                .set(&key, &children[name], Some(timestamp))
                .await?;
        }
        self.data_store
            .set(&self.manifest_key, &manifest, Some(timestamp))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_store_interface::{DataStoreResponse, RequestPath};
    use crate::id_lists_adapter::id_lists_snapshot::IdListsSnapshot;
    use crate::id_lists_adapter::IdListUpdate;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::Notify;

    #[derive(Default)]
    struct RecordingDataStore {
        writes: Mutex<Vec<(String, String)>>,
        fail_child: bool,
        fail_key_fragment: Option<&'static str>,
        transient_failure: Mutex<Option<&'static str>>,
        block_next_set: AtomicBool,
        set_started: Notify,
    }

    #[async_trait]
    impl DataStoreTrait for RecordingDataStore {
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

        async fn set(&self, key: &str, value: &str, _time: Option<u64>) -> Result<(), StatsigErr> {
            if self.block_next_set.swap(false, Ordering::SeqCst) {
                self.set_started.notify_one();
                std::future::pending::<()>().await;
            }
            if (self.fail_child && key.contains("id_list::"))
                || self
                    .transient_failure
                    .lock()
                    .is_some_and(|fragment| key.contains(fragment))
                || self
                    .fail_key_fragment
                    .is_some_and(|fragment| key.contains(fragment))
            {
                return Err(StatsigErr::DataStoreFailure(
                    "child write failed".to_string(),
                ));
            }
            self.writes
                .lock()
                .push((key.to_string(), value.to_string()));
            Ok(())
        }

        async fn support_polling_updates_for(&self, _path: RequestPath) -> bool {
            false
        }
    }

    fn metadata(name: &str, remote_size: u64) -> IdListMetadata {
        IdListMetadata {
            name: name.to_string(),
            url: "https://example.invalid/list".to_string(),
            file_id: Some("file-1".to_string()),
            size: remote_size,
            creation_time: 123,
        }
    }

    fn snapshot_for(manifest: &HashMap<String, IdListMetadata>) -> HashMap<String, IdList> {
        manifest
            .iter()
            .map(|(name, meta)| {
                (
                    name.clone(),
                    IdList::from_complete_snapshot(
                        meta.clone(),
                        HashSet::from(["A1234567".into()]),
                    ),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn unchanged_manifest_skips_snapshot_work_but_all_metadata_changes_publish() {
        let store = Arc::new(RecordingDataStore::default());
        let writer =
            IdListsDataStoreWriter::new("secret-test", &StatsigOptions::default(), store.clone());
        let mut manifest = HashMap::from([("a".to_string(), metadata("a", 10))]);
        for change in 0..8 {
            match change {
                0 => {}
                1 => manifest.get_mut("a").unwrap().size += 10,
                2 => manifest.get_mut("a").unwrap().file_id = Some("rotated".into()),
                3 => manifest.get_mut("a").unwrap().creation_time += 1,
                4 => manifest.get_mut("a").unwrap().url.push_str("?renewed"),
                5 => {
                    let mut renamed = manifest.remove("a").unwrap();
                    renamed.name = "renamed".into();
                    manifest.insert("renamed".into(), renamed);
                }
                6 => {
                    manifest.insert("b".into(), metadata("b", 10));
                }
                7 => {
                    manifest.clear();
                }
                _ => unreachable!(),
            }
            let before = store.writes.lock().len();
            writer
                .write_if_changed(&manifest, || Ok(snapshot_for(&manifest)))
                .await
                .unwrap();
            assert_eq!(
                store.writes.lock().len(),
                before + manifest.len() + 1,
                "change={change}"
            );
            // Reinsert in another order: compare parsed metadata, not JSON/map order.
            let mut entries: Vec<_> = manifest
                .iter()
                .map(|(name, meta)| (name.clone(), meta.clone()))
                .collect();
            entries.reverse();
            let equivalent = entries.into_iter().collect();
            writer
                .write_if_changed(&equivalent, || {
                    panic!("unchanged tick must not clone any ID sets")
                })
                .await
                .unwrap();
            assert_eq!(store.writes.lock().len(), before + manifest.len() + 1);
        }
    }

    #[tokio::test]
    async fn failed_child_or_manifest_publication_retries_even_when_remote_returns_to_old_state() {
        for failure in ["|id_list::a|", "/v1/get_id_lists|"] {
            let store = Arc::new(RecordingDataStore::default());
            let writer = IdListsDataStoreWriter::new(
                "secret-test",
                &StatsigOptions::default(),
                store.clone(),
            );
            let original = HashMap::from([("a".to_string(), metadata("a", 10))]);
            let changed = HashMap::from([("a".to_string(), metadata("a", 20))]);
            let changed_snapshot = || {
                let mut snapshot = snapshot_for(&changed);
                snapshot.get_mut("a").unwrap().ids = HashSet::from(["B1234567".into()]);
                Ok(snapshot)
            };
            writer
                .write_if_changed(&original, || Ok(snapshot_for(&original)))
                .await
                .unwrap();
            *store.transient_failure.lock() = Some(failure);
            assert!(writer
                .write_if_changed(&changed, changed_snapshot)
                .await
                .is_err());
            *store.transient_failure.lock() = None;
            let before = store.writes.lock().len();
            writer
                .write_if_changed(&original, || Ok(snapshot_for(&original)))
                .await
                .unwrap();
            assert_eq!(store.writes.lock().len(), before + 2, "failure={failure}");
            assert_eq!(store.writes.lock()[before].1, "+A1234567\n");
            *store.transient_failure.lock() = Some(failure);
            assert!(writer
                .write_if_changed(&changed, changed_snapshot)
                .await
                .is_err());
            *store.transient_failure.lock() = None;
            let before = store.writes.lock().len();
            writer
                .write_if_changed(&changed, changed_snapshot)
                .await
                .unwrap();
            assert_eq!(store.writes.lock().len(), before + 2);
            assert_eq!(store.writes.lock()[before].1, "+B1234567\n");
            writer
                .write_if_changed(&changed, || {
                    panic!("successful retry must establish checkpoint")
                })
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn unavailable_or_invalid_snapshot_is_retried_before_checkpointing() {
        let store = Arc::new(RecordingDataStore::default());
        let writer =
            IdListsDataStoreWriter::new("secret-test", &StatsigOptions::default(), store.clone());
        let manifest = HashMap::from([("a".to_string(), metadata("a", 10))]);
        assert!(writer
            .write_if_changed(&manifest, || Err(StatsigErr::LockFailure(
                "snapshot unavailable".into()
            )))
            .await
            .is_err());
        assert!(writer
            .write_if_changed(&manifest, || Ok(HashMap::new()))
            .await
            .is_err());
        assert!(store.writes.lock().is_empty());
        writer
            .write_if_changed(&manifest, || Ok(snapshot_for(&manifest)))
            .await
            .unwrap();
        assert_eq!(store.writes.lock().len(), 2);
        writer
            .write_if_changed(&manifest, || {
                panic!("successful retry establishes checkpoint")
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn cancelled_publication_does_not_leave_the_old_checkpoint_clean() {
        let store = Arc::new(RecordingDataStore::default());
        let writer =
            IdListsDataStoreWriter::new("secret-test", &StatsigOptions::default(), store.clone());
        let original = HashMap::from([("a".to_string(), metadata("a", 10))]);
        let changed = HashMap::from([("a".to_string(), metadata("a", 20))]);
        writer
            .write_if_changed(&original, || Ok(snapshot_for(&original)))
            .await
            .unwrap();
        store.block_next_set.store(true, Ordering::SeqCst);
        {
            let write = writer.write_if_changed(&changed, || Ok(snapshot_for(&changed)));
            tokio::pin!(write);
            tokio::select! {
                _ = store.set_started.notified() => {},
                result = &mut write => panic!("write should block, got {result:?}"),
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => panic!("write never started"),
            }
        }
        writer
            .write_if_changed(&original, || Ok(snapshot_for(&original)))
            .await
            .unwrap();
        assert_eq!(store.writes.lock().len(), 4);
    }

    #[tokio::test]
    async fn publishes_condensed_children_before_remote_cursor_manifest() {
        let store = Arc::new(RecordingDataStore::default());
        let writer =
            IdListsDataStoreWriter::new("secret-test", &StatsigOptions::default(), store.clone());
        let remote_log = "+A1234567\n+B1234567\n-B1234567\n+C1234567\n";
        let remote = HashMap::from([
            (
                "active".to_string(),
                metadata("active", remote_log.len() as u64),
            ),
            ("empty".to_string(), metadata("empty", 99)),
        ]);
        let mut active = IdList::new(remote["active"].clone());
        active.apply_update(IdListUpdate {
            raw_changeset: Some(remote_log.to_string()),
            new_metadata: remote["active"].clone(),
        });
        let snapshot = HashMap::from([
            ("active".to_string(), active),
            (
                "empty".to_string(),
                IdList::from_complete_snapshot(remote["empty"].clone(), HashSet::new()),
            ),
        ]);

        writer.write_snapshot(&snapshot, &remote).await.unwrap();

        let writes = store.writes.lock();
        assert_eq!(writes.len(), 3);
        assert!(writes[0].0.contains("id_list::active"));
        assert_eq!(writes[0].1, "+A1234567\n+C1234567\n");
        assert!(writes[1].0.contains("id_list::empty"));
        assert!(writes[1].1.is_empty());
        assert!(writes[2].0.contains("/v1/get_id_lists"));
        let saved: HashMap<String, IdListMetadata> = serde_json::from_str(&writes[2].1).unwrap();
        assert_eq!(saved["active"].size, remote_log.len() as u64);
        assert_eq!(saved["empty"].size, 99);
        let children = HashMap::from([
            ("active".to_string(), writes[0].1.clone()),
            ("empty".to_string(), writes[1].1.clone()),
        ]);
        let parsed = IdListsSnapshot::parse(&writes[2].1, &children).unwrap();
        assert_eq!(parsed.lists["active"].ids.len(), 2);
        assert!(parsed.lists["empty"].ids.is_empty());
        assert_eq!(
            parsed.lists["active"].metadata.size,
            remote_log.len() as u64
        );
    }

    #[tokio::test]
    async fn refuses_to_publish_manifest_when_child_write_fails() {
        let store = Arc::new(RecordingDataStore {
            fail_child: true,
            ..RecordingDataStore::default()
        });
        let writer =
            IdListsDataStoreWriter::new("secret-test", &StatsigOptions::default(), store.clone());
        let meta = metadata("active", 42);
        let remote = HashMap::from([("active".to_string(), meta.clone())]);
        let snapshot = HashMap::from([(
            "active".to_string(),
            IdList::from_complete_snapshot(meta, HashSet::from(["A1234567".to_string()])),
        )]);

        assert!(writer.write_snapshot(&snapshot, &remote).await.is_err());
        assert!(store.writes.lock().is_empty());
    }

    #[tokio::test]
    async fn later_write_failure_keeps_prior_children_but_never_replaces_manifest() {
        for (fragment, expected_children) in [("|id_list::b|", 1), ("/v1/get_id_lists|", 2)] {
            let store = Arc::new(RecordingDataStore {
                fail_key_fragment: Some(fragment),
                ..Default::default()
            });
            let writer = IdListsDataStoreWriter::new(
                "secret-test",
                &StatsigOptions::default(),
                store.clone(),
            );
            let remote: HashMap<_, _> = ["a", "b"]
                .into_iter()
                .map(|name| (name.to_string(), metadata(name, 42)))
                .collect();
            let snapshot = remote
                .iter()
                .map(|(name, meta)| {
                    (
                        name.clone(),
                        IdList::from_complete_snapshot(
                            meta.clone(),
                            HashSet::from(["A1234567".into()]),
                        ),
                    )
                })
                .collect();
            assert!(writer.write_snapshot(&snapshot, &remote).await.is_err());
            let writes = store.writes.lock();
            assert_eq!(writes.len(), expected_children);
            assert!(writes.iter().all(|(key, _)| key.contains("|id_list::")));
            // Fixed-key writes are ordered, not transactional: successful earlier
            // children remain, but failure never publishes a new manifest.
            assert!(writes[0].0.contains("|id_list::a|"));
        }
    }

    #[tokio::test]
    async fn rejects_incomplete_applied_cursor_before_any_write() {
        let store = Arc::new(RecordingDataStore::default());
        let writer =
            IdListsDataStoreWriter::new("secret-test", &StatsigOptions::default(), store.clone());
        let remote = HashMap::from([("active".to_string(), metadata("active", 42))]);
        let snapshot = HashMap::from([(
            "active".to_string(),
            IdList::from_complete_snapshot(
                metadata("active", 11),
                HashSet::from(["a".to_string()]),
            ),
        )]);

        assert!(writer.write_snapshot(&snapshot, &remote).await.is_err());
        assert!(store.writes.lock().is_empty());
    }

    #[tokio::test]
    async fn rejects_invalid_applied_child_before_any_write() {
        let store = Arc::new(RecordingDataStore::default());
        let writer =
            IdListsDataStoreWriter::new("secret-test", &StatsigOptions::default(), store.clone());
        let meta = metadata("active", 42);
        let remote = HashMap::from([("active".to_string(), meta.clone())]);
        let snapshot = HashMap::from([(
            "active".to_string(),
            IdList::from_complete_snapshot(meta, HashSet::from(["bad".to_string()])),
        )]);

        assert!(writer.write_snapshot(&snapshot, &remote).await.is_err());
        assert!(store.writes.lock().is_empty());
    }
}
