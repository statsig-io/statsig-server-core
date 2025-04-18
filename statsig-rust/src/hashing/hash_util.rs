use serde::Deserialize;

use super::{djb2::djb2, memo_sha_256::MemoSha256};
use std::fmt::Display;

#[derive(Deserialize, Eq, PartialEq)]
pub enum HashAlgorithm {
    Djb2,
    None,
    Sha256,
}

impl HashAlgorithm {
    #[must_use]
    pub fn from_string(input: &str) -> Option<Self> {
        match input {
            "sha256" => Some(HashAlgorithm::Sha256),
            "djb2" => Some(HashAlgorithm::Djb2),
            "none" => Some(HashAlgorithm::None),
            _ => None,
        }
    }
}

impl Display for HashAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashAlgorithm::Djb2 => write!(f, "djb2"),
            HashAlgorithm::None => write!(f, "none"),
            HashAlgorithm::Sha256 => write!(f, "sha256"),
        }
    }
}

pub struct HashUtil {
    sha_hasher: MemoSha256,
}

impl Default for HashUtil {
    fn default() -> Self {
        Self::new()
    }
}

impl HashUtil {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sha_hasher: MemoSha256::new(),
        }
    }

    pub fn hash(&self, input: &String, hash_algorithm: &HashAlgorithm) -> String {
        match hash_algorithm {
            HashAlgorithm::Sha256 => self.sha_hasher.hash_string(input),
            HashAlgorithm::Djb2 => djb2(input),
            HashAlgorithm::None => input.to_string(),
        }
    }

    pub fn sha256(&self, input: &String) -> String {
        self.sha_hasher.hash_string(input)
    }

    pub fn sha256_to_u64(&self, input: &String) -> u64 {
        let hash = self.sha_hasher.hash_string(input);

        let mut hasher_bytes = [0u8; 8];
        hasher_bytes.copy_from_slice(&hash.as_bytes()[0..8]);

        u64::from_be_bytes(hasher_bytes)
    }

    pub fn evaluation_hash(&self, input: &String) -> Option<usize> {
        self.sha_hasher.compute_hash(input)
    }
}
