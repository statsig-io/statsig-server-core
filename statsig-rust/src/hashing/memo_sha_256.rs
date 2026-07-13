use ahash::AHasher;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use dashmap::DashMap;
use sha2::digest::Output;
use sha2::{Digest, Sha256};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_CACHE_ENTRIES: usize = 10000;

enum EvaluationHashKey {
    Raw(String),
    Dot2(String, String),
    Dot3(String, String, String),
}

enum EvaluationHashKeyRef<'a> {
    Raw(&'a str),
    Dot2(&'a str, &'a str),
    Dot3(&'a str, &'a str, &'a str),
}

impl EvaluationHashKey {
    fn matches(&self, other: &EvaluationHashKeyRef) -> bool {
        match (self, other) {
            (EvaluationHashKey::Raw(a), EvaluationHashKeyRef::Raw(b)) => a == b,
            (EvaluationHashKey::Dot2(a1, a2), EvaluationHashKeyRef::Dot2(b1, b2)) => {
                a1 == b1 && a2 == b2
            }
            (EvaluationHashKey::Dot3(a1, a2, a3), EvaluationHashKeyRef::Dot3(b1, b2, b3)) => {
                a1 == b1 && a2 == b2 && a3 == b3
            }
            _ => false,
        }
    }
}

impl<'a> EvaluationHashKeyRef<'a> {
    fn fast_hash(&self) -> u64 {
        let mut hasher = AHasher::default();
        std::mem::discriminant(self).hash(&mut hasher);
        match self {
            EvaluationHashKeyRef::Raw(a) => a.hash(&mut hasher),
            EvaluationHashKeyRef::Dot2(a, b) => {
                a.hash(&mut hasher);
                b.hash(&mut hasher);
            }
            EvaluationHashKeyRef::Dot3(a, b, c) => {
                a.hash(&mut hasher);
                b.hash(&mut hasher);
                c.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    fn to_owned_key(&self) -> EvaluationHashKey {
        match self {
            EvaluationHashKeyRef::Raw(a) => EvaluationHashKey::Raw((*a).to_string()),
            EvaluationHashKeyRef::Dot2(a, b) => {
                EvaluationHashKey::Dot2((*a).to_string(), (*b).to_string())
            }
            EvaluationHashKeyRef::Dot3(a, b, c) => {
                EvaluationHashKey::Dot3((*a).to_string(), (*b).to_string(), (*c).to_string())
            }
        }
    }
}

/// [S2SDK-140] Previously this held a single `Mutex<MemoState>` that
/// every `evaluate_pass_percentage` call locked, even for cache hits, which
/// serialized concurrent evaluations. The cache is now a sharded `DashMap`
/// (hits only take a shard read lock) and misses hash with a stack-local
/// `Sha256` instead of a shared one behind the mutex.
pub struct MemoSha256 {
    user_hash_cache: DashMap<u64, Vec<(EvaluationHashKey, u64)>, ahash::RandomState>,
    user_hash_cache_entries: AtomicUsize,
}

impl MemoSha256 {
    pub fn new() -> Self {
        Self {
            user_hash_cache: DashMap::with_hasher(ahash::RandomState::default()),
            user_hash_cache_entries: AtomicUsize::new(0),
        }
    }

    pub fn compute_hash(&self, input: &str) -> Option<u64> {
        self.compute_hash_for_key(EvaluationHashKeyRef::Raw(input))
    }

    pub fn compute_hash_dot2(&self, a: &str, b: &str) -> Option<u64> {
        self.compute_hash_for_key(EvaluationHashKeyRef::Dot2(a, b))
    }

    pub fn compute_hash_dot3(&self, a: &str, b: &str, c: &str) -> Option<u64> {
        self.compute_hash_for_key(EvaluationHashKeyRef::Dot3(a, b, c))
    }

    fn compute_hash_for_key(&self, key: EvaluationHashKeyRef) -> Option<u64> {
        let fast_hash = key.fast_hash();
        if let Some(bucket) = self.user_hash_cache.get(&fast_hash) {
            if let Some((_, value)) = bucket
                .iter()
                .find(|(cached_key, _)| cached_key.matches(&key))
            {
                return Some(*value);
            }
        }

        // swap(0) elects a single evicting thread: racing callers that also saw
        // the count above the cap will swap out 0 and skip the clear().
        if self.user_hash_cache_entries.load(Ordering::Relaxed) > MAX_CACHE_ENTRIES
            && self.user_hash_cache_entries.swap(0, Ordering::Relaxed) > MAX_CACHE_ENTRIES
        {
            self.user_hash_cache.clear();
        }

        let hash = compute_bytes_for_key(&key);

        match hash.split_at(size_of::<u64>()).0.try_into() {
            Ok(bytes) => {
                let u_bytes = u64::from_be_bytes(bytes);
                let mut bucket = self.user_hash_cache.entry(fast_hash).or_default();
                // Re-check under the write guard: a racing thread may have cached
                // the same key between our lock-free read miss and this point, and
                // duplicate entries would grow the bucket and inflate the counter.
                if !bucket
                    .iter()
                    .any(|(cached_key, _)| cached_key.matches(&key))
                {
                    bucket.push((key.to_owned_key(), u_bytes));
                    self.user_hash_cache_entries.fetch_add(1, Ordering::Relaxed);
                }
                Some(u_bytes)
            }
            _ => None,
        }
    }

    pub fn hash_string(&self, input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        BASE64_STANDARD.encode(hasher.finalize())
    }
}

fn compute_bytes_for_key(key: &EvaluationHashKeyRef) -> Output<Sha256> {
    let mut hasher = Sha256::new();
    match key {
        EvaluationHashKeyRef::Raw(a) => {
            hasher.update(a.as_bytes());
        }
        EvaluationHashKeyRef::Dot2(a, b) => {
            hasher.update(a.as_bytes());
            hasher.update(b".");
            hasher.update(b.as_bytes());
        }
        EvaluationHashKeyRef::Dot3(a, b, c) => {
            hasher.update(a.as_bytes());
            hasher.update(b".");
            hasher.update(b.as_bytes());
            hasher.update(b".");
            hasher.update(c.as_bytes());
        }
    }
    hasher.finalize()
}
