use crate::log_e;
use ahash::AHasher;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use parking_lot::Mutex;
use sha2::digest::Output;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Duration;

const MAX_CACHE_ENTRIES: usize = 10000;

struct MemoState {
    sha_hasher: Sha256,
    user_hash_cache: HashMap<u64, Vec<(EvaluationHashKey, u64)>>,
    user_hash_cache_entries: usize,
}

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

const TAG: &str = stringify!(MemoSha256);
pub struct MemoSha256 {
    inner: Mutex<MemoState>,
}

impl MemoSha256 {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MemoState {
                sha_hasher: Sha256::new(),
                user_hash_cache: HashMap::new(),
                user_hash_cache_entries: 0,
            }),
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
        let mut state = match self.inner.try_lock_for(Duration::from_secs(5)) {
            Some(state) => state,
            None => {
                log_e!(
                    TAG,
                    "Failed to acquire lock for Sha256: Failed to lock inner"
                );
                return None;
            }
        };

        let fast_hash = key.fast_hash();
        if let Some(bucket) = state.user_hash_cache.get(&fast_hash) {
            if let Some((_, value)) = bucket
                .iter()
                .find(|(cached_key, _)| cached_key.matches(&key))
            {
                return Some(*value);
            }
        }

        if state.user_hash_cache_entries > MAX_CACHE_ENTRIES {
            state.user_hash_cache.clear();
            state.user_hash_cache_entries = 0;
        }

        let hash = self.compute_bytes_for_key(&mut state, &key);

        match hash.split_at(size_of::<u64>()).0.try_into() {
            Ok(bytes) => {
                let u_bytes = u64::from_be_bytes(bytes);
                state
                    .user_hash_cache
                    .entry(fast_hash)
                    .or_default()
                    .push((key.to_owned_key(), u_bytes));
                state.user_hash_cache_entries += 1;
                Some(u_bytes)
            }
            _ => None,
        }
    }

    pub fn hash_string(&self, input: &str) -> String {
        let mut state = match self.inner.try_lock_for(Duration::from_secs(5)) {
            Some(state) => state,
            None => {
                log_e!(
                    TAG,
                    "Failed to acquire lock for Sha256: Failed to lock inner"
                );
                return "STATSIG_HASH_ERROR".to_string();
            }
        };

        let hash = self.compute_bytes(&mut state, input);
        BASE64_STANDARD.encode(hash)
    }

    fn compute_bytes(&self, state: &mut MemoState, input: &str) -> Output<Sha256> {
        state.sha_hasher.update(input.as_bytes());
        state.sha_hasher.finalize_reset()
    }

    fn compute_bytes_for_key(
        &self,
        state: &mut MemoState,
        key: &EvaluationHashKeyRef,
    ) -> Output<Sha256> {
        match key {
            EvaluationHashKeyRef::Raw(a) => {
                state.sha_hasher.update(a.as_bytes());
            }
            EvaluationHashKeyRef::Dot2(a, b) => {
                state.sha_hasher.update(a.as_bytes());
                state.sha_hasher.update(b".");
                state.sha_hasher.update(b.as_bytes());
            }
            EvaluationHashKeyRef::Dot3(a, b, c) => {
                state.sha_hasher.update(a.as_bytes());
                state.sha_hasher.update(b".");
                state.sha_hasher.update(b.as_bytes());
                state.sha_hasher.update(b".");
                state.sha_hasher.update(c.as_bytes());
            }
        }
        state.sha_hasher.finalize_reset()
    }
}
