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
    evaluation::{
        dynamic_returnable::DynamicReturnableValue,
        evaluator_value::{EvaluatorValue, EvaluatorValueInner, MemoizedEvaluatorValue},
    },
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
    evaluator_values: AHashMap<u64, &'static MemoizedEvaluatorValue>,
}

#[derive(Default)]
struct MutableData {
    strings: AHashMap<u64, Arc<String>>,
    returnables: AHashMap<u64, Arc<Box<RawValue>>>,
    evaluator_values: AHashMap<u64, Arc<MemoizedEvaluatorValue>>,
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
            return InternedString::from_static(hash, string);
        }

        let ptr = Self::get_string_from_local(hash, value);
        InternedString::from_pointer(hash, ptr)
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
            return DynamicReturnable::from_static(hash, returnable);
        }

        let ptr = Self::get_returnable_from_local(hash, value);
        DynamicReturnable::from_pointer(hash, ptr)
    }

    pub fn get_or_intern_evaluator_value(value: Cow<'_, RawValue>) -> EvaluatorValue {
        let raw_string = value.get();
        let hash = hashing::hash_one(raw_string.as_bytes());

        if let Some(evaluator_value) = Self::get_evaluator_value_from_shared(hash) {
            return EvaluatorValue::from_static(hash, evaluator_value);
        }

        let ptr = Self::get_evaluator_value_from_local(hash, value);
        EvaluatorValue::from_pointer(hash, ptr)
    }

    pub fn try_get_bootstrapped_evaluator_value(bytes: &[u8]) -> Option<EvaluatorValue> {
        let hash = hashing::hash_one(bytes);
        if let Some(evaluator_value) = Self::get_evaluator_value_from_shared(hash) {
            return Some(EvaluatorValue::from_static(hash, evaluator_value));
        }

        None
    }

    pub fn try_get_bootstrapped_returnable(bytes: &[u8]) -> Option<DynamicReturnable> {
        match bytes {
            b"true" => return Some(DynamicReturnable::from_bool(true)),
            b"false" => return Some(DynamicReturnable::from_bool(false)),
            b"null" => return Some(DynamicReturnable::empty()),
            _ => {}
        }

        let hash = hashing::hash_one(bytes);
        if let Some(returnable) = Self::get_returnable_from_shared(hash) {
            return Some(DynamicReturnable::from_static(hash, returnable));
        }

        None
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

    pub fn release_evaluator_value(hash: u64) {
        use_mutable_data("release_eval_value", |data| {
            Self::try_release_entry(&mut data.evaluator_values, hash)
        });
    }

    #[cfg(test)]
    pub fn get_memoized_len() -> (
        /* strings */ usize,
        /* returnables */ usize,
        /* evaluator values */ usize,
    ) {
        match MUTABLE_DATA.try_lock() {
            Some(memo) => (
                memo.strings.len(),
                memo.returnables.len(),
                memo.evaluator_values.len(),
            ),
            None => (0, 0, 0),
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

    // ------------------------------------------------------------------------------- [ Evaluator Value ]

    fn get_evaluator_value_from_shared(hash: u64) -> Option<&'static MemoizedEvaluatorValue> {
        match IMMORTAL_DATA.get() {
            Some(shared) => shared.evaluator_values.get(&hash).copied(),
            None => None,
        }
    }

    fn get_evaluator_value_from_local(
        hash: u64,
        value: Cow<'_, RawValue>,
    ) -> Arc<MemoizedEvaluatorValue> {
        let result = use_mutable_data("eval_value_lookup", |data| {
            if let Some(evaluator_value) = data.evaluator_values.get(&hash) {
                return Some(evaluator_value.clone());
            }

            None
        });

        if let Some(evaluator_value) = result {
            return evaluator_value;
        }

        // intentinonally done across two locks to avoid deadlock with InternedString creation
        let ptr = Arc::new(MemoizedEvaluatorValue::from_raw_value(value));
        let _ = use_mutable_data("intern_evaluator_value", |data| {
            data.evaluator_values.insert(hash, ptr.clone());
            Some(())
        });

        ptr
    }

    // ------------------------------------------------------------------------------- [ Helpers ]

    fn try_release_entry<T>(data: &mut AHashMap<u64, Arc<T>>, hash: u64) -> Option<Arc<T>> {
        let found = match data.entry(hash) {
            Entry::Occupied(entry) => entry,
            Entry::Vacant(_) => return None,
        };

        let strong_count = Arc::strong_count(found.get());
        if strong_count == 1 {
            let value = found.remove();
            // return the value so it isn't dropped while holding the lock
            return Some(value);
        }

        None
    }
}

fn use_mutable_data<T>(
    reason: &str,
    f: impl FnOnce(&mut MutexGuard<'_, MutableData>) -> Option<T>,
) -> Option<T> {
    let mut data = match MUTABLE_DATA.try_lock_for(Duration::from_secs(5)) {
        Some(data) => data,
        None => {
            #[cfg(test)]
            panic!("Failed to acquire lock for mutable data ({reason})");

            #[cfg(not(test))]
            {
                log_e!(TAG, "Failed to acquire lock for mutable data ({reason})");
                return None;
            }
        }
    };

    f(&mut data)
}

fn traverse_object(
    data: &mut ImmortalData,
    object: serde_json::Map<String, serde_json::Value>,
) -> Result<(), StatsigErr> {
    let is_regex_operator = object.get("operator").is_some_and(|o| o == "str_matches");

    for (key, value) in object.into_iter() {
        let is_returnable = key == "returnValue" || key == "defaultValue";
        let is_evaluator_value = key == "targetValue";

        immortalize_string(data, key)?;

        match value {
            v if is_evaluator_value => immortalize_evaluator_value(data, v, is_regex_operator)?,
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

fn immortalize_evaluator_value(
    data: &mut ImmortalData,
    value: serde_json::Value,
    should_compile_regex: bool,
) -> Result<(), StatsigErr> {
    let raw_value = match to_raw_value(&value) {
        Ok(raw_value) => raw_value,
        Err(e) => {
            log_e!(TAG, "Failed to convert map to bytes: {}", e);
            return Err(StatsigErr::JsonParseError(TAG.to_string(), e.to_string()));
        }
    };

    let bytes = raw_value.get().as_bytes();
    let hash = hashing::hash_one(bytes);

    data.evaluator_values.entry(hash).or_insert_with(|| {
        log_d!(
            TAG,
            "Adding evaluator value: {} with hash: {}",
            raw_value.get(),
            hash
        );

        let mut evaluator_value: MemoizedEvaluatorValue = match serde_json::from_slice(bytes) {
            Ok(map) => map,
            Err(e) => {
                log_e!(TAG, "InternedStore: Failed to convert value to map: {}", e);
                panic!("InternedStore: Failed to convert value to map: {}", e);
            }
        };

        if should_compile_regex {
            evaluator_value.compile_regex();
        }

        Box::leak(Box::new(evaluator_value))
    });
    Ok(())
}

// ------------------------------------------------------------------------------- [ Helper Implementations ]

impl EvaluatorValue {
    fn from_static(hash: u64, evaluator_value: &'static MemoizedEvaluatorValue) -> Self {
        Self {
            hash,
            inner: EvaluatorValueInner::Static(evaluator_value),
        }
    }

    fn from_pointer(hash: u64, pointer: Arc<MemoizedEvaluatorValue>) -> Self {
        Self {
            hash,
            inner: EvaluatorValueInner::Pointer(pointer),
        }
    }
}

impl DynamicReturnable {
    fn from_static(hash: u64, returnable: &'static RawValue) -> Self {
        Self {
            hash,
            value: DynamicReturnableValue::JsonStatic(returnable),
        }
    }

    fn from_pointer(hash: u64, pointer: Arc<Box<RawValue>>) -> Self {
        Self {
            hash,
            value: DynamicReturnableValue::JsonPointer(pointer),
        }
    }
}

impl InternedString {
    fn from_static(hash: u64, string: &'static str) -> Self {
        Self {
            hash,
            value: InternedStringValue::Static(string),
        }
    }

    fn from_pointer(hash: u64, pointer: Arc<String>) -> Self {
        Self {
            hash,
            value: InternedStringValue::Pointer(pointer),
        }
    }
}
