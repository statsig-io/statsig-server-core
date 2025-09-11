use std::{
    borrow::Cow,
    sync::{Arc, Weak},
    time::Duration,
};

use ahash::HashMap;
use parking_lot::Mutex;
use serde_json::value::RawValue;

use crate::{hashing, log_e};

pub trait FromRawValue {
    fn from_raw_value(raw_value: Cow<'_, RawValue>) -> Self;
}

pub struct InternedValueStore<T: FromRawValue> {
    tag: &'static str,
    pub(crate) values: Mutex<HashMap<u64, Weak<T>>>,
}

impl<T: FromRawValue> InternedValueStore<T> {
    pub fn new(tag: &'static str) -> Self {
        Self {
            tag,
            values: Mutex::new(HashMap::default()),
        }
    }

    pub fn get_or_create_interned_value(&self, raw_value: Cow<'_, RawValue>) -> (u64, Arc<T>) {
        let raw_value_str = raw_value.get();
        let hash = hashing::hash_one(raw_value_str);

        if let Some(value) = self.try_get_interned_value(hash) {
            return (hash, value);
        }

        let value = T::from_raw_value(raw_value);
        let value_arc = Arc::new(value);
        self.set_interned_value(hash, &value_arc);

        (hash, value_arc)
    }

    pub fn try_remove_interned_value(&self, hash: u64) {
        let mut memoized_values = match self.values.try_lock_for(Duration::from_secs(5)) {
            Some(values) => values,
            None => return,
        };

        let found = match memoized_values.get(&hash) {
            Some(value) => value,
            None => return,
        };

        if found.strong_count() == 1 {
            memoized_values.remove(&hash);
        }
    }

    fn set_interned_value(&self, hash: u64, value: &Arc<T>) {
        let mut memoized_values = match self.values.try_lock_for(Duration::from_secs(5)) {
            Some(values) => values,
            None => return,
        };
        memoized_values.insert(hash, Arc::downgrade(value));
    }

    fn try_get_interned_value(&self, hash: u64) -> Option<Arc<T>> {
        let memoized_values = match self.values.try_lock_for(Duration::from_secs(5)) {
            Some(values) => values,
            None => {
                log_e!(self.tag, "Failed to lock interned values map");
                return None;
            }
        };

        memoized_values.get(&hash).and_then(|value| value.upgrade())
    }
}

#[macro_export]
macro_rules! impl_interned_value {
    ($struct_name:ident, $memoized_type:ident, $tag:literal) => {
        lazy_static::lazy_static! {
            pub(crate) static ref INTERNED_STORE: $crate::interned_value_store::InternedValueStore<$memoized_type> = $crate::interned_value_store::InternedValueStore::new($tag);
        }

        impl Drop for $struct_name {
            fn drop(&mut self) {
                INTERNED_STORE.try_remove_interned_value(self.hash);
            }
        }

        impl $memoized_type {
            fn get_or_create(raw_value: Cow<'_, RawValue>) -> (u64, Arc<$memoized_type>) {
                INTERNED_STORE.get_or_create_interned_value(raw_value)
            }
        }
    };
}
