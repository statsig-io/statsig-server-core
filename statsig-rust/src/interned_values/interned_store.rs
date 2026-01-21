use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use ahash::AHashMap;
use lazy_static::lazy_static;
use parking_lot::{Mutex, MutexGuard};
use serde_json::Value;

use crate::{
    hashing,
    interned_string::{InternedString, InternedStringValue},
    log_d, log_e, log_w, StatsigErr,
};

const TAG: &str = "InternedStore";

static IMMORTAL_DATA: OnceLock<ImmortalData> = OnceLock::new();

lazy_static! {
    static ref MUTABLE_DATA: Mutex<MutableData> = Mutex::new(MutableData::default());
}

/// Immortal vs Mutable Data
/// ------------------------------------------------------------
/// -`ImmortalData` is static and never changes. It will only exist if a successful call to `bootstrap` is made. It is intentionally
///  leaked so that it can be accessed across forks without incrementing the reference count.
/// -`MutableData` is dynamic and changes over time as values are added and removed.
/// ------------------------------------------------------------
/// In all cases, we first check if there is a ImmortalData entry and then fallback to MutableData.
#[derive(Default)]
struct ImmortalData {
    strings: AHashMap<u64, &'static str>,
}

#[derive(Default)]
struct MutableData {
    strings: AHashMap<u64, Arc<String>>,
}

pub trait Internable: Sized {
    type Input<'a>;
    fn intern(input: Self::Input<'_>) -> Self;
}

pub struct InternedStore;

impl InternedStore {
    pub fn bootstrap(data: &[u8]) -> Result<(), StatsigErr> {
        if IMMORTAL_DATA.get().is_some() {
            log_e!(TAG, "Already bootstrapped");
            return Err(StatsigErr::InvalidOperation(
                "Already bootstrapped".to_string(),
            ));
        }

        let start_time = Instant::now();
        let mut immortal_data = ImmortalData::default();

        let json_obj: serde_json::Map<String, Value> = serde_json::from_slice(data)
            .map_err(|e| StatsigErr::JsonParseError(TAG.to_string(), e.to_string()))?;

        traverse_object(&mut immortal_data, json_obj)?;

        if IMMORTAL_DATA.set(immortal_data).is_err() {
            let err = "Failed to set IMMORTAL_DATA";
            log_e!(TAG, "{}", err);
            return Err(StatsigErr::InvalidOperation(err.to_string()));
        }

        let end_time = Instant::now();
        log_d!(
            TAG,
            "Bootstrap took {}ms",
            end_time.duration_since(start_time).as_millis()
        );

        Ok(())
    }

    pub fn get_or_intern_string<T: AsRef<str> + ToString>(value: T) -> InternedString {
        let hash = hashing::hash_one(value.as_ref().as_bytes());

        if let Some(string) = Self::get_string_from_shared(hash) {
            return InternedString {
                hash,
                value: InternedStringValue::Static(string),
            };
        }

        let ptr = Self::get_string_from_local(hash, value);
        InternedString {
            hash,
            value: InternedStringValue::Pointer(ptr),
        }
    }

    fn get_string_from_shared(hash: u64) -> Option<&'static str> {
        match IMMORTAL_DATA.get() {
            Some(shared) => shared.strings.get(&hash).copied(),
            None => None,
        }
    }

    fn get_string_from_local<T: ToString>(hash: u64, value: T) -> Arc<String> {
        let result = use_mutable_data("intern_string", |data| {
            if let Some(string) = data.strings.get(&hash) {
                return Some(string.clone());
            }

            let ptr = Arc::new(value.to_string());
            data.strings.insert(hash, ptr.clone());
            Some(ptr)
        });

        result.unwrap_or_else(|| {
            log_w!(TAG, "Failed to get string from local");
            Arc::new(value.to_string())
        })
    }
}

fn use_mutable_data<T>(
    reason: &str,
    f: impl FnOnce(&mut MutexGuard<'_, MutableData>) -> Option<T>,
) -> Option<T> {
    let mut data = match MUTABLE_DATA.try_lock_for(Duration::from_secs(5)) {
        Some(data) => data,
        None => {
            log_e!(TAG, "Failed to acquire lock for mutable data ({reason})");
            return None;
        }
    };

    f(&mut data)
}

fn traverse_object(
    data: &mut ImmortalData,
    object: serde_json::Map<String, serde_json::Value>,
) -> Result<(), StatsigErr> {
    for (key, value) in object.into_iter() {
        add_string(data, key)?;

        match value {
            Value::String(s) => add_string(data, s)?,
            Value::Object(o) => traverse_object(data, o)?,
            Value::Array(a) => traverse_array(data, a)?,
            _ => {}
        }
    }

    Ok(())
}

fn traverse_array(data: &mut ImmortalData, array: Vec<Value>) -> Result<(), StatsigErr> {
    for item in array {
        match item {
            Value::String(s) => add_string(data, s)?,
            Value::Object(o) => traverse_object(data, o)?,
            _ => {}
        }
    }
    Ok(())
}

fn add_string(data: &mut ImmortalData, value: String) -> Result<(), StatsigErr> {
    let hash = hashing::hash_one(value.as_bytes());
    data.strings.entry(hash).or_insert_with(|| {
        log_d!(TAG, "Adding string: {} with hash: {}", value, hash);
        Box::leak(value.into_boxed_str())
    });
    Ok(())
}
