use crate::log_e;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use parking_lot::Mutex;
use sha2::digest::Output;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;

const MAX_CACHE_ENTRIES: usize = 10000;

struct MemoState {
    sha_hasher: Sha256,
    user_hash_cache: HashMap<String, u64>,
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
            }),
        }
    }

    pub fn compute_hash(&self, input: &str) -> Option<u64> {
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

        if let Some(cache) = state.user_hash_cache.get(input) {
            return Some(*cache);
        }

        if state.user_hash_cache.len() > MAX_CACHE_ENTRIES {
            state.user_hash_cache.clear();
        }

        let u_bytes = Self::compute_u64_hash(&mut state.sha_hasher, [input])?;
        state.user_hash_cache.insert(input.to_owned(), u_bytes);
        Some(u_bytes)
    }

    pub fn compute_hash_parts(&self, parts: &[&str]) -> Option<u64> {
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

        Self::compute_u64_hash(&mut state.sha_hasher, parts.iter().copied())
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

    fn compute_u64_hash<'a>(
        hasher: &mut Sha256,
        parts: impl IntoIterator<Item = &'a str>,
    ) -> Option<u64> {
        for part in parts {
            hasher.update(part.as_bytes());
        }

        let hash = hasher.finalize_reset();
        match hash.split_at(size_of::<u64>()).0.try_into() {
            Ok(bytes) => Some(u64::from_be_bytes(bytes)),
            _ => None,
        }
    }
}
