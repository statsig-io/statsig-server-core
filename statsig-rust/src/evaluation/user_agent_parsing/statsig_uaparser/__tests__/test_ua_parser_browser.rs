use super::super::ua_parser::UaParser;

const TEST_DATA_PATH: &str = "../statsig-rust/tests/data/test_must_pass_user_agents.txt";

#[test]
fn parsing_browser_names() {
    use super::test_helpers::load_test_cases_from_file;

    for test_case in load_test_cases_from_file(TEST_DATA_PATH) {
        let user_agent = &test_case.user_agent;
        let browser = UaParser::parse_browser(user_agent);

        let expected_browser_name = test_case
            .expected_browser_family
            .as_deref()
            .unwrap_or("Other");

        assert_eq!(
            browser.name, expected_browser_name,
            "\n--------------------------------\nGot: {:?}, \nExpected: {:?}, \nUser Agent: {}\n",
            browser.name, expected_browser_name, user_agent
        );
    }
}

#[test]
fn os_name_parsing_chrome() {
    let user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36";
    let browser = UaParser::parse_browser(user_agent);

    assert_eq!(browser.name, "Chrome");
    let browser_version = browser.version.get_version_string();
    assert_eq!(browser_version, Some("137.0.0.0".to_string()));
}

#[test]
fn os_name_parsing_safari() {
    let user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15";
    let browser = UaParser::parse_browser(user_agent);

    assert_eq!(browser.name, "Safari");
    let browser_version = browser.version.get_version_string();
    assert_eq!(browser_version, Some("17.5".to_string()));
}

#[test]
fn os_name_parsing_firefox() {
    let user_agent =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:139.0) Gecko/20100101 Firefox/139.0";
    let browser = UaParser::parse_browser(user_agent);

    assert_eq!(browser.name, "Firefox");
    let browser_version = browser.version.get_version_string();
    assert_eq!(browser_version, Some("139.0".to_string()));
}
