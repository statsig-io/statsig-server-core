use base64::{engine::general_purpose::STANDARD, Engine};
use rcgen::{
    BasicConstraints, CertificateParams, IsCa, KeyPair, PublicKeyData, SignatureAlgorithm,
};
use rustls::client::{danger::ServerCertVerifier, WebPkiServerVerifier};
use rustls::pki_types::{ServerName, UnixTime};
use std::{sync::Arc, time::Duration};

// Public SPKI SEQUENCE contents from rustls-webpki 0.103.16, src/data/distrusted:
// https://github.com/rustls/webpki/tree/8ecdcc7/src/data/distrusted
// No compromised private keys are needed: a local test CA signs certificates
// containing these public keys. This exercises the shared HTTP/gRPC verifier,
// not TLS proof-of-possession or a connection to a live compromised endpoint.
const DISTRUSTED_SPKIS: [(&str, &str); 8] = [
    ("EMDVTLSCAG3B", "MBAGByqGSM49AgEGBSuBBAAiA2IABOee2cBKl3PEc15RER2oWsK9XmSrN0uA4Sy8IDjsHk8OO4EvyYWeHmevsE4vi8l/dIW6ZaqRHuik5Fti1cwD1WvmeLPDjLTck3yiRg4jhFPIkpNdKdRPkeFWBrY8ztq0aA=="),
    ("EMDVTLSCAG3C", "MBAGByqGSM49AgEGBSuBBAAiA2IABDp6eGLIxE0yTd1pAzpngsDv7v5AVeaKEj+gNxujvZGUUU7hcqQz42MPAr0YgDfZ5RSfm3fblKKB35h7L+qIi3S9/o/aTBCZ7myhlWb+PvDl9rCaVWzMi6UFYd8bKkQdAQ=="),
    ("EMEVTLSCAG3B", "MBAGByqGSM49AgEGBSuBBAAiA2IABCXbqFpk05faCxBS6epUbkh77+qcnZ1ooMXq3P7vdaabtdb8AfQr+8putAvNZMB2fjb/0F2XqBPFqgxslTDnnPXwhmRo8TgGQgqlyFGECx4lJUDnkOS2VMOFLtZK8ehlfQ=="),
    ("EMEVTLSCAG3C", "MBAGByqGSM49AgEGBSuBBAAiA2IABC1W6Cmm+4LGSkaX2WCvlR6QZ66R+SbTQwbaSKiq63hptqZ+bSdCbgRCqgOzbhmI3/Y2UU7hzwf1uks/8qBmRa59MdWGatMao9JG6wsxd+6Zy3VmxRbOdICKZo+R8MR/yg=="),
    ("EMOVCSCAG3B", "MBAGByqGSM49AgEGBSuBBAAiA2IABKq7D7Bmas/PAfrVK/z9TCvyyzw0l95zlQ5YGbYf5pUJh0HIxJFDmzSgQxyv/lv5tMHI6K4AAfn03IrYLFnzY56WzWw9TbDO/sJsPWJRezd2E3J0P46K3XexAGTyq+XgUA=="),
    ("EMOVCSCAG3C", "MBAGByqGSM49AgEGBSuBBAAiA2IABLnw4GNHAqacpqrdRDz9YsQbm6SyCSRpfXl052kaEm+5avs+NML9O8X0gwYfJl3CpYQ2NHMjUQQ2so8E84YjYwZ5gOJQPgJTLWVGI1pxZaJZoe9Jb3QbX97neHII5Lj67w=="),
    ("EMOVTLSCAG3B", "MBAGByqGSM49AgEGBSuBBAAiA2IABHOJeqDrPP3KixJAL/rwq8ThISuvje53isnuLW24P6woDF4FY4rnoqzXHDIqjjYc3rn2nEMCud2Nc9HMbKqyzAruTiFs6XndUYpK9LYm5WhM3WWAN+0MhU5MwS9M+GRtDQ=="),
    ("EMOVTLSCAG3C", "MBAGByqGSM49AgEGBSuBBAAiA2IABBOCUc1SrJKYwciq2XcyHeuX5wm9C0ygrKVdyK1R3PydFYmhWX46USDh79Yxxj4YNcrK4phppi4WMeiigYGrVmFtxF2RirwJ86sOY895KqTc7XOHvje7pWlUnxwCsnDtZw=="),
];

struct PublicKeyOnly(Vec<u8>);

impl PublicKeyData for PublicKeyOnly {
    fn der_bytes(&self) -> &[u8] {
        &self.0
    }

    fn algorithm(&self) -> &SignatureAlgorithm {
        &rcgen::PKCS_ECDSA_P384_SHA384
    }
}

#[test]
fn distrusted_public_keys_are_rejected_even_with_a_trusted_custom_ca() {
    let mut ca_params = CertificateParams::new(vec![]).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca_key = KeyPair::generate().unwrap();
    let ca = ca_params.self_signed(&ca_key).unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(ca.der().clone()).unwrap();
    let verifier = WebPkiServerVerifier::builder_with_provider(
        Arc::new(roots),
        Arc::new(rustls::crypto::ring::default_provider()),
    )
    .build()
    .unwrap();
    let name = ServerName::try_from("localhost").unwrap();
    let time = UnixTime::since_unix_epoch(Duration::from_secs(1_800_000_000));
    let params = || CertificateParams::new(vec!["localhost".into()]).unwrap();

    // Positive control: the same CA, hostname, validity and signature algorithm
    // must accept an ordinary public key. Rejection must not be a broken fixture.
    let ordinary_key = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P384_SHA384).unwrap();
    let ordinary = params().signed_by(&ordinary_key, &ca, &ca_key).unwrap();
    verifier
        .verify_server_cert(ordinary.der(), &[], &name, &[], time)
        .expect("ordinary certificate signed by the custom CA must verify");
    assert!(verifier
        .verify_server_cert(
            ordinary.der(),
            &[],
            &ServerName::try_from("wrong.example").unwrap(),
            &[],
            time,
        )
        .is_err());

    for (label, encoded) in DISTRUSTED_SPKIS {
        let spki = STANDARD.decode(encoded).unwrap();
        // All upstream fixtures use id-ecPublicKey/secp384r1 followed by an
        // uncompressed EC point. Strip only that exact algorithm/BIT STRING prefix.
        let point = spki
            .strip_prefix(&[
                0x30, 0x10, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x05, 0x2b,
                0x81, 0x04, 0x00, 0x22, 0x03, 0x62, 0x00,
            ])
            .unwrap();
        assert_eq!(point.len(), 97, "{label}");
        let leaf = params()
            .signed_by(&PublicKeyOnly(point.to_vec()), &ca, &ca_key)
            .unwrap();
        let error = verifier
            .verify_server_cert(leaf.der(), &[], &name, &[], time)
            .expect_err(label);
        assert!(
            format!("{error:?}").contains("CertDistrusted"),
            "{label} must fail for explicit SPKI distrust, not another TLS error: {error:?}"
        );
    }
}
