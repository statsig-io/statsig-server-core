use std::{
    borrow::Cow,
    collections::hash_map::Entry,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use ahash::AHashMap;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde_json::value::RawValue;

use crate::{
    evaluation::{
        dynamic_returnable::DynamicReturnableValue,
        evaluator_value::{EvaluatorValue, EvaluatorValueInner, MemoizedEvaluatorValue},
    },
    hashing,
    interned_string::{InternedString, InternedStringValue},
    log_d, log_e, log_w,
    networking::ResponseData,
    observability::ops_stats::OpsStatsForInstance,
    specs_response::{
        proto_specs::deserialize_protobuf,
        spec_types::{Spec, SpecsResponseFull},
        specs_hash_map::{SpecPointer, SpecsHashMap},
    },
    DynamicReturnable, StatsigErr,
};

const TAG: &str = "InternedStore";

static IMMORTAL_DATA: OnceLock<ImmortalData> = OnceLock::new();

lazy_static! {
    static ref MUTABLE_DATA: Mutex<MutableData> = Mutex::new(MutableData::default());
}

/// Immortal vs Mutable Data
/// ------------------------------------------------------------
/// -`ImmortalData` is static and never changes. It will only exist if a successful call to `preload` is made. It is intentionally
///  leaked so that it can be accessed across forks without incrementing the reference count.
/// -`MutableData` is dynamic and changes over time as values are added and removed.
/// ------------------------------------------------------------
/// In all cases, we first check if there is a ImmortalData entry and then fallback to MutableData.
#[derive(Default)]
struct ImmortalData {
    strings: AHashMap<u64, &'static str>,
    returnables: AHashMap<u64, &'static RawValue>,
    evaluator_values: AHashMap<u64, &'static MemoizedEvaluatorValue>,
    feature_gates: AHashMap<u64, &'static Spec>,
    dynamic_configs: AHashMap<u64, &'static Spec>,
    layer_configs: AHashMap<u64, &'static Spec>,
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
    pub fn preload(data: &[u8]) -> Result<(), StatsigErr> {
        let start_time = Instant::now();

        if IMMORTAL_DATA.get().is_some() {
            log_e!(TAG, "Already preloaded");
            return Err(StatsigErr::InvalidOperation(
                "Already preloaded".to_string(),
            ));
        }

        let specs_response = try_parse_as_json(data).or_else(|_| try_parse_as_proto(data))?;

        let immortal = mutable_to_immortal(specs_response)?;

        if IMMORTAL_DATA.set(immortal).is_err() {
            return Err(StatsigErr::LockFailure(
                "Failed to set IMMORTAL_DATA".to_string(),
            ));
        }

        let end_time = Instant::now();
        log_d!(
            TAG,
            "Preload took {}ms",
            end_time.duration_since(start_time).as_millis()
        );

        Ok(())
    }

    pub fn get_or_intern_string<T: AsRef<str> + ToString>(value: T) -> InternedString {
        let hash = hashing::hash_one(value.as_ref().as_bytes());

        if let Some(string) = get_string_from_shared(hash) {
            return InternedString::from_static(hash, string);
        }

        let ptr = get_string_from_local(hash, value);
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

        if let Some(returnable) = get_returnable_from_shared(hash) {
            return DynamicReturnable::from_static(hash, returnable);
        }

        let ptr = get_returnable_from_local(hash, value);
        DynamicReturnable::from_pointer(hash, ptr)
    }

    pub fn get_or_intern_evaluator_value(value: Cow<'_, RawValue>) -> EvaluatorValue {
        let raw_string = value.get();
        let hash = hashing::hash_one(raw_string.as_bytes());

        if let Some(evaluator_value) = get_evaluator_value_from_shared(hash) {
            return EvaluatorValue::from_static(hash, evaluator_value);
        }

        let ptr = get_evaluator_value_from_local(hash, value);
        EvaluatorValue::from_pointer(hash, ptr)
    }

    pub fn replace_evaluator_value(hash: u64, evaluator_value: Arc<MemoizedEvaluatorValue>) {
        let old = use_mutable_data("replace_evaluator_value", |data| {
            data.evaluator_values.insert(hash, evaluator_value)
        });
        drop(old);
    }

    pub fn try_get_preloaded_evaluator_value(bytes: &[u8]) -> Option<EvaluatorValue> {
        let hash = hashing::hash_one(bytes);
        if let Some(evaluator_value) = get_evaluator_value_from_shared(hash) {
            return Some(EvaluatorValue::from_static(hash, evaluator_value));
        }

        None
    }

    pub fn try_get_preloaded_returnable(bytes: &[u8]) -> Option<DynamicReturnable> {
        match bytes {
            b"true" => return Some(DynamicReturnable::from_bool(true)),
            b"false" => return Some(DynamicReturnable::from_bool(false)),
            b"null" => return Some(DynamicReturnable::empty()),
            _ => {}
        }

        let hash = hashing::hash_one(bytes);
        if let Some(returnable) = get_returnable_from_shared(hash) {
            return Some(DynamicReturnable::from_static(hash, returnable));
        }

        None
    }

    pub fn try_get_preloaded_dynamic_config(name: &InternedString) -> Option<SpecPointer> {
        match IMMORTAL_DATA.get() {
            Some(shared) => shared
                .dynamic_configs
                .get(&name.hash)
                .map(|s| SpecPointer::Static(s)),
            None => None,
        }
    }

    pub fn try_get_preloaded_layer_config(name: &InternedString) -> Option<SpecPointer> {
        match IMMORTAL_DATA.get() {
            Some(shared) => shared
                .layer_configs
                .get(&name.hash)
                .map(|s| SpecPointer::Static(s)),
            None => None,
        }
    }

    pub fn try_get_preloaded_feature_gate(name: &InternedString) -> Option<SpecPointer> {
        match IMMORTAL_DATA.get() {
            Some(shared) => shared
                .feature_gates
                .get(&name.hash)
                .map(|s| SpecPointer::Static(s)),
            None => None,
        }
    }

    pub fn release_returnable(hash: u64) {
        let ptr = use_mutable_data("release_returnable", |data| {
            try_release_entry(&mut data.returnables, hash)
        });
        drop(ptr);
    }

    pub fn release_string(hash: u64) {
        let ptr = use_mutable_data("release_string", |data| {
            try_release_entry(&mut data.strings, hash)
        });
        drop(ptr);
    }

    pub fn release_evaluator_value(hash: u64) {
        let ptr = use_mutable_data("release_eval_value", |data| {
            try_release_entry(&mut data.evaluator_values, hash)
        });
        drop(ptr);
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
}

// ------------------------------------------------------------------------------- [ Preloading ]

fn try_parse_as_json(data: &[u8]) -> Result<SpecsResponseFull, StatsigErr> {
    serde_json::from_slice(data)
        .map_err(|e| StatsigErr::JsonParseError(TAG.to_string(), e.to_string()))
}

fn try_parse_as_proto(data: &[u8]) -> Result<SpecsResponseFull, StatsigErr> {
    let current = SpecsResponseFull::default();
    let mut next = SpecsResponseFull::default();

    let mut response_data = ResponseData::from_bytes_with_headers(
        data.to_vec(),
        Some(std::collections::HashMap::from([(
            "content-encoding".to_string(),
            "statsig-br".to_string(),
        )])),
    );

    let ops_stats = OpsStatsForInstance::new();

    deserialize_protobuf(&ops_stats, &current, &mut next, &mut response_data)?;

    Ok(next)
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

        None
    });

    if let Some(returnable) = result {
        return returnable;
    }

    let owned = match value {
        Cow::Borrowed(value) => value.to_owned(),
        Cow::Owned(value) => value,
    };

    let ptr = Arc::new(owned);

    use_mutable_data("intern_returnable", |data| {
        data.returnables.insert(hash, ptr.clone());
        Some(())
    });

    ptr
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

fn use_mutable_data<T>(reason: &str, f: impl FnOnce(&mut MutableData) -> Option<T>) -> Option<T> {
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

fn mutable_to_immortal(specs_res: SpecsResponseFull) -> Result<ImmortalData, StatsigErr> {
    let mutable_data: MutableData = {
        let mut mutable_data_lock = MUTABLE_DATA.lock();
        std::mem::take(&mut *mutable_data_lock)
    };
    let mut immortal = ImmortalData::default();

    for (hash, arc) in mutable_data.strings.into_iter() {
        let raw = Arc::into_raw(arc);
        let leaked: &'static str = unsafe { &*raw };
        immortal.strings.insert(hash, leaked);
    }

    for (hash, returnable) in mutable_data.returnables.into_iter() {
        let raw_returnable = Arc::into_raw(returnable);
        let leaked = unsafe { &*raw_returnable };
        immortal.returnables.insert(hash, leaked);
    }

    for (hash, evaluator_value) in mutable_data.evaluator_values.into_iter() {
        let raw_evaluator_value = Arc::into_raw(evaluator_value);
        let leaked = unsafe { &*raw_evaluator_value };
        immortal.evaluator_values.insert(hash, leaked);
    }

    try_insert_specs(specs_res.feature_gates, &mut immortal.feature_gates);
    try_insert_specs(specs_res.dynamic_configs, &mut immortal.dynamic_configs);
    try_insert_specs(specs_res.layer_configs, &mut immortal.layer_configs);

    Ok(immortal)
}

fn try_insert_specs(source: SpecsHashMap, destination: &mut AHashMap<u64, &'static Spec>) {
    for (name, spec_ptr) in source.0.into_iter() {
        let spec = match spec_ptr {
            SpecPointer::Pointer(spec) => spec,
            _ => continue,
        };

        if spec.checksum.is_none() {
            // no point doint this if there is no checksum field to verify against later
            continue;
        }

        let raw_spec = Arc::into_raw(spec);
        let spec = unsafe { &*raw_spec };
        destination.insert(name.hash, spec);
    }
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
