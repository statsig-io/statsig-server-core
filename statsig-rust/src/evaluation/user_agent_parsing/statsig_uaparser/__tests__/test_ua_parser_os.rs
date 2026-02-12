use super::super::ua_parser::UaParser;

const TEST_DATA_PATH: &str = "../statsig-rust/tests/data/test_must_pass_user_agents.txt";

#[test]
fn parsing_os_names() {
    use super::test_helpers::load_test_cases_from_file;

    for test_case in load_test_cases_from_file(TEST_DATA_PATH) {
        let user_agent = &test_case.user_agent;
        let os = UaParser::parse_os(user_agent);

        let expected_os_name = test_case.expected_os_family.as_deref().unwrap_or("Other");

        assert_eq!(
            os.name, expected_os_name,
            "\n--------------------------------\nGot: {:?}, \nExpected: {:?}, \nUser Agent: {}\n",
            os.name, expected_os_name, user_agent
        );
    }
}

#[test]
fn parsing_windows_10() {
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 Trailer/93.3.8652.5"; // |Windows|10|None|None|None|Chrome|134|0|0|0
    let os = UaParser::parse_os(user_agent);

    assert_eq!(os.name, "Windows");
    let os_version = os.version.get_version_string();
    assert_eq!(os_version, Some("10".to_string()));
}

#[test]
fn os_name_parsing_windows_8() {
    let user_agent = "Mozilla/5.0 (Windows NT 6.3; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2226.0 Safari/537.36"; //|Windows|8|1|None|Chrome|41|0|2226
    let os = UaParser::parse_os(user_agent);

    assert_eq!(os.name, "Windows");
    let os_version = os.version.get_version_string();
    assert_eq!(os_version, Some("8.1".to_string()));
}

#[test]
fn os_name_parsing_ios() {
    let user_agent = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_7_10 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604."; //|iOS|16|7|10|None|Mobile Safari|16|6|None|None
    let os = UaParser::parse_os(user_agent);

    assert_eq!(os.name, "iOS");
    let os_version = os.version.get_version_string();
    assert_eq!(os_version, Some("16.7.10".to_string()));
}

#[test]
fn os_name_parsing_macos() {
    let user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.10 Safari/605.1.1"; //|Mac OS X|10|15|7|Safari|17|10|None
    let os = UaParser::parse_os(user_agent);

    assert_eq!(os.name, "Mac OS X");
    let os_version = os.version.get_version_string();
    assert_eq!(os_version, Some("10.15.7".to_string()));
}

#[test]
fn os_name_parsing_macos_no_x() {
    let user_agent = "Codex Desktop/0.93.0-alpha.13 (Mac OS 26.2.0; arm64) unknown";
    let os = UaParser::parse_os(user_agent);

    assert_eq!(os.name, "Mac OS X");
    let os_version = os.version.get_version_string();
    assert_eq!(os_version, Some("26.2.0".to_string()));
}

#[test]
fn os_name_parsing_ubuntu() {
    let user_agent = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/604.1 (KHTML, like Gecko) Version/11.0 Safari/604.1 Ubuntu/17.04 (3.24.1-0ubuntu1) Epiphany/3.24.1"; // |Ubuntu|17|04|None|None|Epiphany|3|24|1|None
    let os = UaParser::parse_os(user_agent);

    assert_eq!(os.name, "Ubuntu");
    let os_version = os.version.get_version_string();
    assert_eq!(os_version, Some("17.04".to_string()));
}
