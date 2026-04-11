use crate::hashing::djb2;

#[test]
fn djb2_matches_js_client_hashing() {
    assert_eq!(djb2("a_gate"), "2867927529");
    assert_eq!(djb2("test_public"), "3968762550");
}

#[test]
fn djb2_covers_ascii_and_unicode_inputs() {
    struct TestCase {
        label: &'static str,
        input: &'static str,
        expected_hash: &'static str,
    }

    let cases = [
        TestCase {
            label: "empty",
            input: "",
            expected_hash: "0",
        },
        TestCase {
            label: "ascii gate name",
            input: "a_gate",
            expected_hash: "2867927529",
        },
        TestCase {
            label: "ascii public gate",
            input: "test_public",
            expected_hash: "3968762550",
        },
        TestCase {
            label: "bmp latin",
            input: "é",
            expected_hash: "233",
        },
        TestCase {
            label: "bmp cjk",
            input: "你好",
            expected_hash: "652829",
        },
        TestCase {
            label: "non-bmp emoji",
            input: "😀",
            expected_hash: "1772899",
        },
        TestCase {
            label: "mixed ascii and emoji",
            input: "a😀z",
            expected_hash: "57849718",
        },
        TestCase {
            label: "non-bmp gothic letter",
            input: "𐍈",
            expected_hash: "1771336",
        },
        TestCase {
            label: "non-english letter",
            input: "大",
            expected_hash: "22823",
        },
    ];

    for case in cases {
        assert_eq!(
            djb2(case.input),
            case.expected_hash,
            "unexpected hash for {}",
            case.label
        );
    }
}
