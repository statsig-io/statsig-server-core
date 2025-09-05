use statsig_rust::hashing::HashUtil;

#[test]
fn test_evaluation_hash_matches_sha256_prefix() {
    let hasher = HashUtil::new();

    // expect first 8 bytes of SHA-256 digest as big-endian u64:
    assert_eq!(
        hasher.evaluation_hash(&"".to_string()).unwrap() as u64,
        0xE3B0C44298FC1C14_u64
    );
    assert_eq!(
        hasher.evaluation_hash(&"blargh".to_string()).unwrap() as u64,
        0x0AC33512D18E20D5_u64
    );
    assert_eq!(
        hasher.evaluation_hash(&"unicode 🗻".to_string()).unwrap() as u64,
        0xD460740C12959D83_u64
    );
}
