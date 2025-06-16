mod utils;

use more_asserts::assert_gt;
use statsig_rust::{
    evaluation::{dynamic_string::DynamicString, ua_parser::UserAgentParser},
    user::StatsigUserInternal,
    StatsigUser,
};
use utils::helpers::load_contents;

struct TestCase {
    user_agent: String,
    expected_os_family: String,
    expected_os_version: String,
    expected_browser_family: String,
    expected_browser_version: String,
}

lazy_static::lazy_static! {
    static ref TEST_CASES: Vec<TestCase> = load_contents("test_user_agents.txt")
        .lines()
        .skip(1) // skip header
        .map(parse_test_case)
        .collect();
}

// Rust 'uaparser' crate correctness agasint Python's 'ua-parser'
const OS_NAME_THRESHOLD: f64 = 0.99;
const OS_VERSION_THRESHOLD: f64 = 0.99;
const BROWSER_NAME_THRESHOLD: f64 = 0.90;
const BROWSER_VERSION_THRESHOLD: f64 = 0.91;

#[test]
fn test_user_agent_parser_os_name() {
    UserAgentParser::load_parser();

    let mut hit = 0;
    let mut miss = 0;

    for test_case in TEST_CASES.iter() {
        let name = extract_field_from_user_agent(&test_case.user_agent, "os_name");

        if name.as_ref() == Some(&test_case.expected_os_family) {
            hit += 1;
        } else {
            miss += 1;
        }
    }

    let score = hit as f64 / (hit + miss) as f64;
    assert_gt!(score, OS_NAME_THRESHOLD);
}

#[test]
fn test_user_agent_parser_os_version() {
    UserAgentParser::load_parser();

    let mut hit = 0;
    let mut miss = 0;

    for test_case in TEST_CASES.iter() {
        let version = extract_field_from_user_agent(&test_case.user_agent, "os_version");

        if version.as_ref() == Some(&test_case.expected_os_version) {
            hit += 1;
        } else {
            miss += 1;
        }
    }

    let score = hit as f64 / (hit + miss) as f64;
    assert_gt!(score, OS_VERSION_THRESHOLD);
}

#[test]
fn test_user_agent_parser_browser_name() {
    UserAgentParser::load_parser();

    let mut hit = 0;
    let mut miss = 0;

    for test_case in TEST_CASES.iter() {
        let name = extract_field_from_user_agent(&test_case.user_agent, "browser_name");

        if name.as_ref() == Some(&test_case.expected_browser_family) {
            hit += 1;
        } else {
            miss += 1;
        }
    }

    let score = hit as f64 / (hit + miss) as f64;
    assert_gt!(score, BROWSER_NAME_THRESHOLD);
}

#[test]
fn test_user_agent_parser_browser_version() {
    UserAgentParser::load_parser();

    let mut hit = 0;
    let mut miss = 0;

    for test_case in TEST_CASES.iter() {
        let version = extract_field_from_user_agent(&test_case.user_agent, "browser_version");

        if version.as_ref() == Some(&test_case.expected_browser_version) {
            hit += 1;
        } else {
            miss += 1;
        }
    }

    let score = hit as f64 / (hit + miss) as f64;
    assert_gt!(score, BROWSER_VERSION_THRESHOLD);
}

fn extract_field_from_user_agent(user_agent: &str, field: &str) -> Option<String> {
    let mut user = StatsigUser::with_user_id("");
    user.set_user_agent(user_agent.to_string());
    let user_internal = StatsigUserInternal::new(&user, None);

    let mut dummy_override_reason = None;
    let field = DynamicString::from(field.to_string());

    let result = UserAgentParser::get_value_from_user_agent(
        &user_internal,
        &Some(field),
        &mut dummy_override_reason,
    );

    match result {
        Some(value) => value.string_value.map(|s| s.value),
        None => None,
    }
}

fn parse_test_case(test_case_data: &str) -> TestCase {
    let parts = test_case_data.split("|").collect::<Vec<&str>>();
    let os_family = unwrap_if_none(parts[1]).unwrap_or("Other");
    let os_major = unwrap_if_none(parts[2]).unwrap_or("0");
    let os_minor = unwrap_if_none(parts[3]).unwrap_or("0");
    let os_patch = unwrap_if_none(parts[4]).unwrap_or("0");

    let browser_family = unwrap_if_none(parts[5]).unwrap_or("Other");
    let browser_major = unwrap_if_none(parts[6]).unwrap_or("0");
    let browser_minor = unwrap_if_none(parts[7]).unwrap_or("0");
    let browser_patch = unwrap_if_none(parts[8]).unwrap_or("0");

    TestCase {
        user_agent: parts[0].to_string(),
        expected_os_family: os_family.to_string(),
        expected_os_version: format!("{}.{}.{}", os_major, os_minor, os_patch),
        expected_browser_family: browser_family.to_string(),
        expected_browser_version: format!("{}.{}.{}", browser_major, browser_minor, browser_patch),
    }
}

fn unwrap_if_none(s: &str) -> Option<&str> {
    if s == "None" {
        None
    } else {
        Some(s)
    }
}
