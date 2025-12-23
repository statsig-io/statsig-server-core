use ahash::RandomState;
use std::hash::Hash;

lazy_static::lazy_static! {
    pub static ref HASHER: RandomState = RandomState::with_seeds(420, 42, 24, 4);
}

/// Uses the ahash crate: https://crates.io/crates/ahash
/// - Faster than djb2.
/// - Randomized between each run of an application.
/// - Non-cryptographic.
/// - One way hash.
///
/// **Profiled**
///
/// djb2  time: 4.1869 ns
///
/// ahash time: 1.5757 ns
///
#[must_use]
pub fn ahash_str(input: &str) -> u64 {
    HASHER.hash_one(input)
}

pub fn hash_one<T: Hash>(x: T) -> u64 {
    HASHER.hash_one(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn test_gets_same_result_for_same_input() {
        let input = "some_random_input";
        let mut result = ahash_str(input);
        for _ in 0..1000 {
            let new_result = ahash_str(input);
            assert_eq!(result, new_result);
            result = new_result;
        }
    }

    #[test]
    fn test_gets_different_result_for_different_input() {
        let mut seen = HashSet::new();
        for i in 0..100_000 {
            let result = ahash_str(&format!("iter_{i}"));
            assert!(!seen.contains(&result));
            seen.insert(result);
        }
    }
}
