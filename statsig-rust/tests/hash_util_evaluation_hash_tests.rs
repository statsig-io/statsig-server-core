use statsig_rust::hashing::HashUtil;

#[test]
fn test_evaluation_hash_matches_sha256_prefix() {
    let hasher = HashUtil::new();

    // expect first 8 bytes of SHA-256 digest as big-endian u64:
    let got = hasher.evaluation_hash(&"".to_string()).expect("hash");
    assert_eq!(got, 0xE3B0C44298FC1C14);

    let got = hasher.evaluation_hash(&"blargh".to_string()).expect("hash");
    assert_eq!(got, 0x0AC33512D18E20D5);

    let got = hasher.evaluation_hash(&"🗻".to_string()).expect("hash");
    assert_eq!(got, 0x1DDBF4EA8DAE91E5);
}
