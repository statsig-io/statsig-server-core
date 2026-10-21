use serde::Serialize;

use crate::{
    id_lists_adapter::{IdListMetadata, IdListUpdate},
    unwrap_or_noop,
};
use std::collections::HashSet;

#[derive(Clone, Serialize)]
pub struct IdList {
    pub metadata: IdListMetadata,

    #[serde(skip_serializing)]
    pub ids: HashSet<String>,
}

impl IdList {
    pub fn new(metadata: IdListMetadata) -> Self {
        let mut local_metadata = metadata;
        local_metadata.size = 0;

        Self {
            metadata: local_metadata,
            ids: HashSet::new(),
        }
    }

    pub fn apply_update(&mut self, update: IdListUpdate) {
        let updated_meta = update.new_metadata;

        if updated_meta.file_id != self.metadata.file_id {
            if updated_meta.creation_time < self.metadata.creation_time {
                return; // older file from a stale manifest
            }
            self.update_metadata(updated_meta);
        } else {
            // Same file: take the manifest's creationTime and URL, keep the bytes already read.
            let size = self.metadata.size;
            self.metadata = updated_meta;
            self.metadata.size = size;
        }

        let changeset_data = unwrap_or_noop!(&update.raw_changeset);

        for change in changeset_data.lines() {
            let trimmed = change.trim();
            if trimmed.len() <= 1 {
                continue;
            }

            let op = change.chars().next();
            let id = &change[1..];

            match op {
                Some('+') => {
                    self.ids.insert(id.to_string());
                }
                Some('-') => {
                    self.ids.remove(id);
                }
                _ => continue,
            }
        }

        self.metadata.size += changeset_data.len() as u64;
    }

    #[allow(dead_code)] // Used by the staged DataStore snapshot parser before adapter wiring.
    pub(crate) fn from_complete_snapshot(metadata: IdListMetadata, ids: HashSet<String>) -> Self {
        Self { metadata, ids }
    }

    fn update_metadata(&mut self, metadata: IdListMetadata) {
        self.metadata = metadata;
        self.metadata.size = 0;
        self.ids.clear();
    }
}
