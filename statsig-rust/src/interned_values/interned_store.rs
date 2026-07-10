use std::{
    borrow::Cow,
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, OnceLock,
    },
    time::Instant,
};

use ahash::AHashMap;
use dashmap::DashMap;
use lazy_static::lazy_static;
use memmap2::Mmap;
use ouroboros::self_referencing;
use rkyv::{
    collections::swiss_table::ArchivedHashMap, string::ArchivedString, Archive,
    Deserialize as RkyvDeserialize, Serialize as RkyvSerialize,
};
use serde_json::value::RawValue;

use crate::{
    evaluation::{
        dynamic_returnable::DynamicReturnableValue,
        evaluator_value::{EvaluatorValue, EvaluatorValueInner, MemoizedEvaluatorValue},
        rkyv_value::{ArchivedRkyvValue, RkyvValue},
    },
    hashing,
    interned_string::{InternedString, InternedStringValue},
    log_d, log_e,
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
static MMAP_DATA: OnceLock<LoadedMmapData> = OnceLock::new();

lazy_static! {
    static ref MUTABLE_DATA: MutableData = MutableData::default();
}

// [S2SDK-140] How many drops of Pointer-backed interned values are allowed to
// accumulate before one thread sweeps the corresponding map for dead entries.
// Sweeping is O(map len) but happens once per SWEEP_THRESHOLD drops, so the amortized
// per-drop cost is a single atomic increment.
const SWEEP_THRESHOLD: usize = 65_536;

#[derive(Default, Archive, RkyvDeserialize, RkyvSerialize)]
struct MmapData {
    strings: std::collections::HashMap<u64, String>,
    returnables: std::collections::HashMap<u64, HashMap<String, RkyvValue>>,
}

#[self_referencing]
struct LoadedMmapData {
    file: File,
    mmap: Mmap,

    #[borrows(mmap)]
    archived: &'this ArchivedMmapData,
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
    returnables: AHashMap<u64, &'static HashMap<String, RkyvValue>>,
    evaluator_values: AHashMap<u64, &'static MemoizedEvaluatorValue>,
    feature_gates: AHashMap<u64, &'static Spec>,
    dynamic_configs: AHashMap<u64, &'static Spec>,
    layer_configs: AHashMap<u64, &'static Spec>,
}
#[derive(Default)]
struct MutableData {
    strings: LocalStore<String>,
    returnables: LocalStore<HashMap<String, RkyvValue>>,
    evaluator_values: LocalStore<MemoizedEvaluatorValue>,
}

/// [S2SDK-140] Refcount-free interning store.
///
/// The previous implementation kept all three maps behind a single global
/// `Mutex<MutableData>`, and every `Drop` of an interned value re-entered that
/// mutex to decrement/evict. Under concurrent evaluation this mutex serialized
/// every check_gate call (see eval_contention_bench).
///
/// This version:
/// - uses a sharded `DashMap`, so lookups of a hot key only take a shard *read* lock
/// - makes `Drop` free of any map access: it just counts, and once enough drops
///   accumulate, a single thread (CAS-guarded) sweeps the map and removes entries
///   whose only remaining owner is the map itself (`Arc::strong_count == 1`).
struct LocalStore<T> {
    map: DashMap<u64, Arc<T>, ahash::RandomState>,
    pending_releases: AtomicUsize,
    sweep_in_progress: AtomicBool,
}

impl<T> Default for LocalStore<T> {
    fn default() -> Self {
        Self {
            map: DashMap::with_hasher(ahash::RandomState::default()),
            pending_releases: AtomicUsize::new(0),
            sweep_in_progress: AtomicBool::new(false),
        }
    }
}

impl<T> LocalStore<T> {
    fn get(&self, hash: u64) -> Option<Arc<T>> {
        self.map.get(&hash).map(|entry| entry.value().clone())
    }

    fn get_or_insert_with(&self, hash: u64, create: impl FnOnce() -> Arc<T>) -> Arc<T> {
        if let Some(existing) = self.get(hash) {
            return existing;
        }
        self.map.entry(hash).or_insert_with(create).value().clone()
    }

    fn insert(&self, hash: u64, value: Arc<T>) {
        self.map.insert(hash, value);
    }

    /// Called from `Drop` impls. Never touches the map on the fast path.
    fn note_release(&self) {
        let pending = self.pending_releases.fetch_add(1, Ordering::Relaxed) + 1;
        if pending < SWEEP_THRESHOLD {
            return;
        }
        self.try_sweep();
    }

    fn try_sweep(&self) {
        loop {
            // Only one sweeper at a time; also prevents re-entrant sweeps of the same
            // map when dropping swept values triggers further note_release calls.
            if self
                .sweep_in_progress
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                return;
            }

            self.pending_releases.store(0, Ordering::Relaxed);

            // Collect dead values and drop them after releasing the shard locks, so any
            // nested Drop work (e.g. MemoizedEvaluatorValue dropping InternedStrings)
            // happens without holding a shard lock on this map.
            let mut dead: Vec<Arc<T>> = Vec::new();
            self.map.retain(|_, value| {
                if Arc::strong_count(value) == 1 {
                    dead.push(value.clone());
                    return false;
                }
                true
            });

            self.sweep_in_progress.store(false, Ordering::Release);
            drop(dead);

            // Drops that landed while we were sweeping had their CAS fail above. If
            // they pushed the counter back over the threshold and traffic then went
            // idle, no future note_release would run, so re-check before returning.
            if self.pending_releases.load(Ordering::Relaxed) < SWEEP_THRESHOLD {
                return;
            }
        }
    }

    /// Concurrency note: the pre-DashMap version `mem::take`'d all three maps atomically
    /// under one mutex; draining per-map/per-key is not atomic. Callers (preload /
    /// write_mmap_data) must run at startup before concurrent eval traffic. Worst case, an
    /// entry interned mid-drain stays in the mutable tier — still correct, just never
    /// immortalized/persisted.
    fn drain(&self) -> Vec<(u64, Arc<T>)> {
        let keys: Vec<u64> = self.map.iter().map(|entry| *entry.key()).collect();
        keys.into_iter()
            .filter_map(|key| self.map.remove(&key))
            .collect()
    }
}

pub trait Internable: Sized {
    type Input<'a>;
    fn intern(input: Self::Input<'_>) -> Self;
}

pub struct InternedStore;

impl InternedStore {
    pub fn preload(data: &[u8]) -> Result<(), StatsigErr> {
        Self::preload_multi(&[data])
    }

    pub fn preload_multi(data: &[&[u8]]) -> Result<(), StatsigErr> {
        let start_time = Instant::now();

        if IMMORTAL_DATA.get().is_some() {
            log_e!(TAG, "Already preloaded");
            return Err(StatsigErr::InvalidOperation(
                "Already preloaded".to_string(),
            ));
        }

        let specs_responses = data
            .iter()
            .map(|data| try_parse_as_json(data).or_else(|_| try_parse_as_proto(data)))
            .collect::<Result<Vec<SpecsResponseFull>, StatsigErr>>()?;

        let immortal = mutable_to_immortal(specs_responses)?;

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

    pub fn write_mmap_data(data: &[&[u8]], path: &str) -> Result<(), StatsigErr> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        let specs_responses = data
            .iter()
            .map(|data| try_parse_as_json(data).or_else(|_| try_parse_as_proto(data)))
            .collect::<Result<Vec<SpecsResponseFull>, StatsigErr>>()?;

        let mmap_data = mutable_to_mmap_data(specs_responses)?;
        let archived = rkyv::to_bytes::<rkyv::rancor::Error>(&mmap_data)
            .map_err(|e| StatsigErr::SerializationError(e.to_string()))?;

        file.write_all(&archived)
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;
        file.sync_all()
            .map_err(|e| StatsigErr::FileError(e.to_string()))?;

        log_d!(TAG, "Wrote {} bytes to mmap file", archived.len());

        Ok(())
    }

    pub fn preload_mmap(path: &str) -> Result<(), StatsigErr> {
        let file = File::open(path).map_err(|e| StatsigErr::FileError(e.to_string()))?;
        let mmap = unsafe { Mmap::map(&file).map_err(|e| StatsigErr::FileError(e.to_string()))? };

        let loaded_result = LoadedMmapDataTryBuilder {
            file,
            mmap,
            archived_builder: |mmap| rkyv::access::<ArchivedMmapData, rkyv::rancor::Error>(mmap),
        }
        .try_build();

        let loaded = match loaded_result {
            Ok(loaded) => loaded,
            Err(e) => {
                return Err(StatsigErr::SerializationError(e.to_string()));
            }
        };

        MMAP_DATA
            .set(loaded)
            .map_err(|_| StatsigErr::LockFailure("Failed to set MMAP_DATA".to_string()))
    }

    pub fn get_or_intern_string<T: AsRef<str> + ToString>(value: T) -> InternedString {
        let hash = hashing::hash_one(value.as_ref().as_bytes());

        if let Some(string) = get_string_from_mmap(hash) {
            return InternedString::from_static(hash, string);
        }

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

        if let Some(returnable) = get_returnable_from_mmap(hash) {
            return DynamicReturnable::from_archived(hash, returnable);
        }

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
        MUTABLE_DATA.evaluator_values.insert(hash, evaluator_value);
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

        if let Some(returnable) = get_returnable_from_mmap(hash) {
            return Some(DynamicReturnable::from_archived(hash, returnable));
        }

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

    // [S2SDK-140] The release_* functions are called from Drop impls on the
    // evaluation hot path. They must not take any lock: cleanup is deferred to an
    // amortized sweep. The hash argument is kept for API compatibility but unused.
    pub fn release_returnable(_hash: u64) {
        MUTABLE_DATA.returnables.note_release();
    }

    pub fn release_string(_hash: u64) {
        MUTABLE_DATA.strings.note_release();
    }

    pub fn release_evaluator_value(_hash: u64) {
        MUTABLE_DATA.evaluator_values.note_release();
    }

    /// Immediately evict entries that are no longer referenced outside the store.
    /// Only needed by tests and (optionally) explicit shutdown paths; normal cleanup
    /// happens automatically via the amortized sweep.
    pub fn run_maintenance() {
        MUTABLE_DATA.strings.try_sweep();
        MUTABLE_DATA.returnables.try_sweep();
        MUTABLE_DATA.evaluator_values.try_sweep();
    }

    #[cfg(test)]
    pub fn get_memoized_len() -> (
        /* strings */ usize,
        /* returnables */ usize,
        /* evaluator values */ usize,
    ) {
        (
            MUTABLE_DATA.strings.map.len(),
            MUTABLE_DATA.returnables.map.len(),
            MUTABLE_DATA.evaluator_values.map.len(),
        )
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

fn get_string_from_mmap(hash: u64) -> Option<&'static str> {
    let data = MMAP_DATA.get()?;
    let archived_hash = rkyv::primitive::ArchivedU64::from_native(hash);
    let found = data.borrow_archived().strings.get(&archived_hash);
    found.map(|s| s.as_str())
}

fn get_string_from_shared(hash: u64) -> Option<&'static str> {
    match IMMORTAL_DATA.get() {
        Some(shared) => shared.strings.get(&hash).copied(),
        None => None,
    }
}

fn get_string_from_local<T: ToString>(hash: u64, value: T) -> Arc<String> {
    MUTABLE_DATA
        .strings
        .get_or_insert_with(hash, || Arc::new(value.to_string()))
}

// ------------------------------------------------------------------------------- [ Returnable ]

fn get_returnable_from_mmap(
    hash: u64,
) -> Option<&'static ArchivedHashMap<ArchivedString, ArchivedRkyvValue>> {
    let data = MMAP_DATA.get()?;

    let archived_hash = rkyv::primitive::ArchivedU64::from_native(hash);
    let found = data.borrow_archived().returnables.get(&archived_hash)?;
    Some(found)
}

fn get_returnable_from_shared(hash: u64) -> Option<&'static HashMap<String, RkyvValue>> {
    match IMMORTAL_DATA.get() {
        Some(shared) => shared.returnables.get(&hash).copied(),
        None => None,
    }
}

fn get_returnable_from_local(hash: u64, value: Cow<RawValue>) -> Arc<HashMap<String, RkyvValue>> {
    if let Some(returnable) = MUTABLE_DATA.returnables.get(hash) {
        return returnable;
    }

    // Parse outside of any shard lock; a racing thread may parse too, but the
    // entry API below keeps exactly one copy.
    let owned: HashMap<String, RkyvValue> = match serde_json::from_str(value.get()) {
        Ok(owned) => owned,
        Err(e) => {
            log_e!(TAG, "Failed to parse returnable from local: {}", e);
            return Arc::new(HashMap::new());
        }
    };

    let ptr = Arc::new(owned);
    MUTABLE_DATA
        .returnables
        .get_or_insert_with(hash, || ptr.clone())
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
    if let Some(evaluator_value) = MUTABLE_DATA.evaluator_values.get(hash) {
        return evaluator_value;
    }

    // Constructed outside of any shard lock: from_raw_value interns nested
    // InternedStrings, which touch the strings map. A racing thread may
    // construct too, but the entry API below keeps exactly one copy.
    let ptr = Arc::new(MemoizedEvaluatorValue::from_raw_value(value));
    MUTABLE_DATA
        .evaluator_values
        .get_or_insert_with(hash, || ptr.clone())
}

// ------------------------------------------------------------------------------- [ Helpers ]

fn mutable_to_immortal(
    specs_responses: Vec<SpecsResponseFull>,
) -> Result<ImmortalData, StatsigErr> {
    let mut immortal = ImmortalData::default();

    for (hash, arc) in MUTABLE_DATA.strings.drain() {
        let raw = Arc::into_raw(arc);
        let leaked: &'static str = unsafe { &*raw };
        immortal.strings.insert(hash, leaked);
    }

    for (hash, returnable) in MUTABLE_DATA.returnables.drain() {
        let raw_returnable = Arc::into_raw(returnable);
        let leaked = unsafe { &*raw_returnable };
        immortal.returnables.insert(hash, leaked);
    }

    for (hash, evaluator_value) in MUTABLE_DATA.evaluator_values.drain() {
        let raw_evaluator_value = Arc::into_raw(evaluator_value);
        let leaked = unsafe { &*raw_evaluator_value };
        immortal.evaluator_values.insert(hash, leaked);
    }

    for response in specs_responses {
        try_insert_specs(response.feature_gates, &mut immortal.feature_gates);
        try_insert_specs(response.dynamic_configs, &mut immortal.dynamic_configs);
        try_insert_specs(response.layer_configs, &mut immortal.layer_configs);
    }

    Ok(immortal)
}

fn mutable_to_mmap_data(specs_responses: Vec<SpecsResponseFull>) -> Result<MmapData, StatsigErr> {
    let mut mmap_data = MmapData::default();

    for (hash, arc) in MUTABLE_DATA.strings.drain() {
        let taken = arc.to_string();
        mmap_data.strings.insert(hash, taken);
    }

    for (hash, returnable) in MUTABLE_DATA.returnables.drain() {
        let taken: HashMap<String, RkyvValue> = returnable.as_ref().clone();
        mmap_data.returnables.insert(hash, taken);
    }

    // TODO: Add evaluator values to mmap data
    // for (hash, evaluator_value) in mutable_data.evaluator_values.into_iter() {
    //     let raw_evaluator_value = Arc::into_raw(evaluator_value);
    //     let leaked = unsafe { &*raw_evaluator_value };
    //     mmap_data.evaluator_values.insert(hash, leaked);
    // }

    // held until after the mmap data is written to the file
    for response in specs_responses {
        drop(response);
    }

    Ok(mmap_data)
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
    fn from_static(hash: u64, returnable: &'static HashMap<String, RkyvValue>) -> Self {
        Self {
            hash,
            value: DynamicReturnableValue::JsonStatic(returnable),
        }
    }

    fn from_archived(
        hash: u64,
        returnable: &'static ArchivedHashMap<ArchivedString, ArchivedRkyvValue>,
    ) -> Self {
        Self {
            hash,
            value: DynamicReturnableValue::JsonArchived(returnable),
        }
    }

    fn from_pointer(hash: u64, pointer: Arc<HashMap<String, RkyvValue>>) -> Self {
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
