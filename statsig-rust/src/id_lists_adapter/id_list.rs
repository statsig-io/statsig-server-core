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

    pub fn apply_update(&mut self, update: &IdListUpdate) {
        let changed = &update.new_metadata;
        let current = &self.metadata;

        if changed.file_id != current.file_id && changed.creation_time >= current.creation_time {
            self.reset();
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

    pub fn reset(&mut self) {
        self.metadata.size = 0;
        self.ids.clear();
    }
}
