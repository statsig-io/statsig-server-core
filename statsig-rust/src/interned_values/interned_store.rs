use std::{
    borrow::Cow,
    collections::hash_map::Entry,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use ahash::AHashMap;
use lazy_static::lazy_static;
use parking_lot::{Mutex, MutexGuard};
use serde_json::{
    value::{to_raw_value, RawValue},
    Value,
};

use crate::{
    evaluation::dynamic_returnable::DynamicReturnableValue,
    hashing,
    interned_string::{InternedString, InternedStringValue},
    log_d, log_e, log_w, DynamicReturnable, StatsigErr,
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
    returnables: AHashMap<u64, &'static RawValue>,
}

#[derive(Default)]
struct MutableData {
    strings: AHashMap<u64, Arc<String>>,
    returnables: AHashMap<u64, Arc<Box<RawValue>>>,
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

    pub fn get_or_intern_returnable(value: Cow<'_, RawValue>) -> DynamicReturnable {
        let raw_string = value.get();
        match raw_string {
            "true" => return DynamicReturnable::from_bool(true),
            "false" => return DynamicReturnable::from_bool(false),
            "null" => return DynamicReturnable::empty(),
            _ => {}
        }

        let hash = hashing::hash_one(raw_string.as_bytes());

        if let Some(returnable) = Self::get_returnable_from_shared(hash) {
            return DynamicReturnable {
                hash,
                value: DynamicReturnableValue::JsonStatic(returnable),
            };
        }

        let ptr = Self::get_returnable_from_local(hash, value);
        DynamicReturnable {
            hash,
            value: DynamicReturnableValue::JsonPointer(ptr),
        }
    }

    pub fn release_returnable(hash: u64) {
        use_mutable_data("release_returnable", |data| {
            Self::try_release_entry(&mut data.returnables, hash)
        });
    }

    pub fn release_string(hash: u64) {
        use_mutable_data("release_string", |data| {
            Self::try_release_entry(&mut data.strings, hash)
        });
    }

    #[cfg(test)]
    pub fn get_memoized_len() -> (/* strings */ usize, /* returnables */ usize) {
        match MUTABLE_DATA.try_lock() {
            Some(memo) => (memo.strings.len(), memo.returnables.len()),
            None => (0, 0),
        }
    }

    // ------------------------------------------------------------------------------- [ String ]

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

    // ------------------------------------------------------------------------------- [ Returnable ]

    fn get_returnable_from_shared(hash: u64) -> Option<&'static RawValue> {
        match IMMORTAL_DATA.get() {
            Some(shared) => shared.returnables.get(&hash).copied(),
            None => None,
        }
    }

    fn get_returnable_from_local(hash: u64, value: Cow<RawValue>) -> Arc<Box<RawValue>> {
        let result = use_mutable_data("intern_returnable", |data| {
            if let Some(returnable) = data.returnables.get(&hash) {
                return Some(returnable.clone());
            }

            let owned = match value.clone() {
                Cow::Borrowed(value) => value.to_owned(),
                Cow::Owned(value) => value,
            };

            let ptr = Arc::new(owned);
            data.returnables.insert(hash, ptr.clone());
            Some(ptr)
        });

        result.unwrap_or_else(|| {
            log_w!(TAG, "Failed to get returnable from local");
            match value {
                Cow::Borrowed(value) => Arc::new(value.to_owned()),
                Cow::Owned(value) => Arc::new(value),
            }
        })
    }

    // ------------------------------------------------------------------------------- [ Helpers ]

    fn try_release_entry<T>(data: &mut AHashMap<u64, Arc<T>>, hash: u64) -> Option<()> {
        let found = match data.entry(hash) {
            Entry::Occupied(entry) => entry,
            Entry::Vacant(_) => return None,
        };

        let strong_count = Arc::strong_count(found.get());
        if strong_count == 1 {
            found.remove();
        }

        Some(())
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
        let is_returnable = key == "returnValue" || key == "defaultValue";

        immortalize_string(data, key)?;

        match value {
            Value::String(s) => immortalize_string(data, s)?,
            Value::Object(o) if is_returnable => immortalize_returnable(data, o)?,
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
            Value::String(s) => immortalize_string(data, s)?,
            Value::Object(o) => traverse_object(data, o)?,
            _ => {}
        }
    }
    Ok(())
}

fn immortalize_string(data: &mut ImmortalData, value: String) -> Result<(), StatsigErr> {
    let hash = hashing::hash_one(value.as_bytes());
    data.strings.entry(hash).or_insert_with(|| {
        log_d!(TAG, "Adding string: {} with hash: {}", value, hash);
        Box::leak(value.into_boxed_str())
    });
    Ok(())
}

fn immortalize_returnable(
    data: &mut ImmortalData,
    value: serde_json::Map<String, serde_json::Value>,
) -> Result<(), StatsigErr> {
    let raw_value = match to_raw_value(&value) {
        Ok(raw_value) => raw_value,
        Err(e) => {
            log_e!(TAG, "Failed to convert map to bytes: {}", e);
            return Err(StatsigErr::JsonParseError(TAG.to_string(), e.to_string()));
        }
    };

    let hash = hashing::hash_one(raw_value.get().as_bytes());
    data.returnables.entry(hash).or_insert_with(|| {
        log_d!(
            TAG,
            "Adding returnable: {} with hash: {}",
            raw_value.get().len(),
            hash
        );

        Box::leak(Box::new(raw_value))
    });
    Ok(())
}
