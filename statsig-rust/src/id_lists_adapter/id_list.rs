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
        let current_meta = &self.metadata;

        if updated_meta.file_id != current_meta.file_id
            && updated_meta.creation_time >= current_meta.creation_time
        {
            self.update_metadata(updated_meta);
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

    fn update_metadata(&mut self, metadata: IdListMetadata) {
        self.metadata = metadata;
        self.metadata.size = 0;
        self.ids.clear();
    }
}
