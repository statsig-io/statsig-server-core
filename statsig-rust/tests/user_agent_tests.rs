mod utils;

use more_asserts::assert_gt;
use statsig_rust::{
    evaluation::{dynamic_string::DynamicString, user_agent_parsing::UserAgentParser},
    user::StatsigUserInternal,
    StatsigUser,
};
use utils::helpers::load_contents;

lazy_static::lazy_static! {
    static ref TEST_CASES: Vec<TestCase> = load_contents("test_user_agents.txt")
        .lines()
        .skip(1) // skip header
        .map(parse_test_case)
        .collect();
}

// Rust 'uaparser' crate correctness against Python's 'ua-parser'
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

        if name == test_case.expected_os_family {
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

        if version == test_case.expected_os_version {
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

        if name == test_case.expected_browser_family {
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

        if version == test_case.expected_browser_version {
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
        /* use_experimental_ua_parser */ false,
    );

    match result {
        Some(value) => value.string_value.map(|s| s.value),
        None => None,
    }
}

#[derive(Default)]
pub struct TestCase {
    pub user_agent: String,
    pub expected_os_family: Option<String>,
    pub expected_os_version: Option<String>,
    pub expected_browser_family: Option<String>,
    pub expected_browser_version: Option<String>,
}

fn parse_test_case(test_case_data: &str) -> TestCase {
    let parts = test_case_data.split("|").collect::<Vec<&str>>();

    let mut result = TestCase {
        user_agent: parts[0].to_string(),
        ..Default::default()
    };

    if parts[1] != "None" {
        result.expected_os_family = Some(parts[1].to_string());
    } else {
        result.expected_os_family = Some("Other".to_string());
    }

    if let Some(os_version) = create_version(parts[2], parts[3], parts[4]) {
        result.expected_os_version = Some(os_version);
    }

    // skip patch_minor index(5)

    if parts[6] != "None" {
        result.expected_browser_family = Some(parts[6].to_string());
    } else {
        result.expected_browser_family = Some("Other".to_string());
    }

    if let Some(browser_version) = create_version(parts[7], parts[8], parts[9]) {
        result.expected_browser_version = Some(browser_version);
    }

    result
}

fn create_version(major: &str, minor: &str, patch: &str) -> Option<String> {
    let mut version = String::new();

    if major == "None" {
        version.push('0');
    } else {
        version.push_str(major);
    }

    version.push('.');

    if minor == "None" {
        version.push('0');
    } else {
        version.push_str(minor);
    }

    version.push('.');

    if patch == "None" {
        version.push('0');
    } else {
        version.push_str(patch);
    }

    Some(version)
}
